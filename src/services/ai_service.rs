//! Reusable AI service.
//!
//! This module is deliberately surface-agnostic: it knows how to resolve
//! the effective OpenAI-compatible configuration for a user and how to ask
//! a model for a structured bookmark proposal, but it knows nothing about
//! HTMX, the manage page, or any particular UI. Future surfaces (an "edit
//! with AI" flow, a browser extension calling the JSON API, ...) should
//! reuse `AiService` and the config-resolution functions here instead of
//! re-implementing provider calls or permission checks.
//!
//! Configuration resolution (first complete config wins):
//!   1. The user's own settings (`ai_user_settings`), when they have set
//!      base URL, API key and model. A user who saved settings with
//!      `enabled = false` has opted out entirely and gets no AI.
//!   2. The shared admin settings (`ai_global_settings`), where each blank
//!      field falls back to an environment variable, when the admin's
//!      access mode permits this user:
//!        - "disabled"       -> nobody may use the shared config
//!        - "admins_only"    -> admins only (default)
//!        - "all_users"      -> every signed-in user
//!        - "selected_users" -> users on the `ai_allowed_users` list only
//!
//! Environment variables (server-level fallback for the shared config):
//!   BRUNNYLOL_AI_BASE_URL / OPENAI_BASE_URL
//!   BRUNNYLOL_AI_API_KEY  / OPENAI_API_KEY
//!   BRUNNYLOL_AI_MODEL    / OPENAI_MODEL
//!
//! The API key is never included in any response, log line, or `Debug`
//! output produced by this module.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;

use crate::{db, validation};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Snapshot of the AI-related environment variables. Taken as a value (not
/// read inside the resolution logic) so resolution is unit-testable without
/// mutating process-wide env vars.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvAiConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

fn env_non_empty(names: &[&str]) -> Option<String> {
    for name in names {
        if let Ok(v) = std::env::var(name) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

impl EnvAiConfig {
    pub fn from_env() -> Self {
        Self {
            base_url: env_non_empty(&["BRUNNYLOL_AI_BASE_URL", "OPENAI_BASE_URL"]),
            api_key: env_non_empty(&["BRUNNYLOL_AI_API_KEY", "OPENAI_API_KEY"]),
            model: env_non_empty(&["BRUNNYLOL_AI_MODEL", "OPENAI_MODEL"]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiConfigSource {
    /// The user's own base URL / key / model from their settings.
    Personal,
    /// The admin-managed shared config (DB values and/or env vars).
    Shared,
}

impl AiConfigSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiConfigSource::Personal => "personal",
            AiConfigSource::Shared => "shared",
        }
    }
}

/// A complete, usable AI configuration. Deliberately does not implement
/// `Debug` printing of the key: the custom impl redacts it.
#[derive(Clone)]
pub struct ResolvedAiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub source: AiConfigSource,
}

impl std::fmt::Debug for ResolvedAiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedAiConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &"[redacted]")
            .field("model", &self.model)
            .field("source", &self.source)
            .finish()
    }
}

fn complete(
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    source: AiConfigSource,
) -> Option<ResolvedAiConfig> {
    match (base_url, api_key, model) {
        (Some(base_url), Some(api_key), Some(model)) => Some(ResolvedAiConfig {
            base_url,
            api_key,
            model,
            source,
        }),
        _ => None,
    }
}

/// Core resolution logic with an explicit env snapshot (testable).
pub async fn resolve_effective_config_with_env(
    pool: &SqlitePool,
    user: &db::User,
    env: &EnvAiConfig,
) -> Option<ResolvedAiConfig> {
    // 1. Per-user settings.
    if let Ok(Some(personal)) = db::ai::get_user_ai_settings(pool, user.id).await {
        if !personal.enabled {
            // Explicit user-level opt-out: no AI for this user at all.
            return None;
        }
        if let Some(cfg) = complete(
            personal.base_url.clone(),
            personal.api_key.clone(),
            personal.model.clone(),
            AiConfigSource::Personal,
        ) {
            return Some(cfg);
        }
        // Partial personal config falls through to the shared config.
    }

    // 2. Shared admin config, gated by the admin's access mode.
    let global = db::ai::get_global_ai_settings(pool).await.ok()?;
    let permitted = match global.access_mode.as_str() {
        "disabled" => false,
        "admins_only" => user.is_admin,
        "all_users" => true,
        "selected_users" => db::ai::is_user_allowed(pool, user.id)
            .await
            .unwrap_or(false),
        _ => false,
    };
    if !permitted {
        return None;
    }

    complete(
        global.base_url.or_else(|| env.base_url.clone()),
        global.api_key.or_else(|| env.api_key.clone()),
        global.model.or_else(|| env.model.clone()),
        AiConfigSource::Shared,
    )
}

/// Resolve the effective AI config for a user using the process env.
pub async fn resolve_effective_config(
    pool: &SqlitePool,
    user: &db::User,
) -> Option<ResolvedAiConfig> {
    resolve_effective_config_with_env(pool, user, &EnvAiConfig::from_env()).await
}

/// Build the chat completions endpoint URL for an OpenAI-compatible base.
///
/// Accepts all common shapes:
///   "https://host"                -> "https://host/chat/completions"
///   "https://host/v1"             -> "https://host/v1/chat/completions"
///   "https://host/chat/completions" -> unchanged
pub fn chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{}/chat/completions", trimmed)
    }
}

// ---------------------------------------------------------------------------
// Bookmark proposals
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NestedProposal {
    pub alias: String,
    pub url: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_template: Option<String>,
}

/// A bookmark as proposed by the model. Mirrors the fields of the manual
/// create form so "accept" and "edit" are trivial mappings in any UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BookmarkProposal {
    pub alias: String,
    /// "templated" (standard) or "nested" (with sub-commands).
    #[serde(default = "default_bookmark_type")]
    pub bookmark_type: String,
    #[serde(default)]
    pub url: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_template: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nested_commands: Vec<NestedProposal>,
}

fn default_bookmark_type() -> String {
    "templated".to_string()
}

/// One turn of conversation with the user about a proposal. The client
/// (manage page today, other surfaces tomorrow) owns the history and sends
/// it back on follow-up requests; the server stays stateless.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeRequest {
    /// The user's description, or their follow-up message on later turns.
    pub message: String,
    #[serde(default)]
    pub history: Vec<ChatTurn>,
    /// The proposal from the previous turn, when this is a follow-up.
    #[serde(default)]
    pub previous_proposal: Option<BookmarkProposal>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposeOutcome {
    pub proposal: BookmarkProposal,
    pub model: String,
    pub source: AiConfigSource,
    /// How many model calls were needed (1 = first proposal was valid;
    /// higher means earlier proposals failed validation and were fed
    /// back to the model for correction).
    pub attempts: u32,
}

/// Maximum number of model calls per proposal request. A proposal that
/// fails validation is fed back to the model with the validation error
/// and retried, up to this many total attempts.
pub const MAX_PROPOSAL_ATTEMPTS: u32 = 3;

const SYSTEM_PROMPT: &str = r#"You design bookmarks for brunnylol, a bunnylol-style bookmark service. Given a user's description, propose exactly one bookmark.

Respond with ONLY a JSON object (no markdown fences, no commentary) with this shape:
{
  "alias": "short alias, lowercase, no spaces, max 50 chars (letters, digits, '-' or '_')",
  "bookmark_type": "templated" or "nested",
  "url": "base URL, must start with https:// or http://",
  "description": "one concise sentence describing the bookmark",
  "command_template": "template string, or null",
  "nested_commands": []
}

Bookmark semantics:
- "templated" is a standard bookmark: a simple redirect (command_template null) or a search/action template.
- The command_template uses the base URL via {url} and variables in braces:
  - {query} is the user's free-text search query, e.g. "{url}/search?q={query}"
  - {var} required variable, {var?} optional, {var=default} default value
  - Pipelines: {var|!encode} disables URL encoding, {var|trim} trims whitespace,
    {var|options[a,b,c]} suggests values, {var|options[a,b,c][strict]} restricts to values,
    {var|map[m:mail,dr:drive]} expands shortcuts (map BEFORE options[strict]).
  - Example: alias "gh", url "https://github.com", command_template "{url}/{user}/{repo}", description "Open a GitHub repository".
- "nested" is a parent bookmark with sub-commands. For nested bookmarks set "url" to the shared base URL (or "" if none), "command_template" to null, and fill "nested_commands" with objects {"alias","url","description","command_template"} using the same template rules. Sub-commands are invoked as: parent child query.

Rules:
- Prefer "templated" unless the user clearly asks for multiple sub-commands.
- The alias must not contain spaces. Keep it short and mnemonic.
- The url must be a real, plausible URL for the service described; never invent URL paths you are unsure of when a simple search template on the service's homepage works.
- When the user gives follow-up feedback, revise the previous proposal accordingly and return the full revised proposal JSON."#;

/// Extract and validate a proposal from raw model output. Tolerates
/// markdown code fences and surrounding prose by taking the outermost
/// JSON object.
pub fn parse_proposal(content: &str) -> Result<BookmarkProposal> {
    let start = content
        .find('{')
        .ok_or_else(|| anyhow!("Model did not return a JSON object"))?;
    let end = content
        .rfind('}')
        .ok_or_else(|| anyhow!("Model did not return a JSON object"))?;
    if end <= start {
        return Err(anyhow!("Model did not return a JSON object"));
    }
    let json_slice = &content[start..=end];
    let proposal: BookmarkProposal =
        serde_json::from_str(json_slice).context("Model returned invalid proposal JSON")?;
    validate_proposal(&proposal)?;
    Ok(proposal)
}

/// Validate a proposal with the same rules the manual create flow applies,
/// so an accepted proposal can never create a bookmark the form would reject.
pub fn validate_proposal(proposal: &BookmarkProposal) -> Result<()> {
    let alias = proposal.alias.trim();
    if alias.is_empty() || alias.len() > 50 {
        return Err(anyhow!(
            "Proposed alias is empty or longer than 50 characters"
        ));
    }
    if alias.chars().any(|c| c.is_whitespace()) {
        return Err(anyhow!("Proposed alias must not contain spaces"));
    }
    if proposal.description.trim().is_empty() {
        return Err(anyhow!("Proposed description is empty"));
    }

    match proposal.bookmark_type.as_str() {
        "templated" => {
            validate_http_url(&proposal.url)?;
            validate_optional_template(&proposal.command_template)?;
        }
        "nested" => {
            if proposal.nested_commands.is_empty() {
                return Err(anyhow!(
                    "Nested proposal must include at least one sub-command"
                ));
            }
            if !proposal.url.trim().is_empty() {
                validate_http_url(&proposal.url)?;
            }
            for nested in &proposal.nested_commands {
                if nested.alias.trim().is_empty() || nested.alias.chars().any(|c| c.is_whitespace())
                {
                    return Err(anyhow!("Nested sub-command has an invalid alias"));
                }
                if nested.description.trim().is_empty() {
                    return Err(anyhow!(
                        "Nested sub-command '{}' has an empty description",
                        nested.alias
                    ));
                }
                validate_http_url(&nested.url)?;
                validate_optional_template(&nested.command_template)?;
            }
        }
        other => return Err(anyhow!("Unknown bookmark_type '{}'", other)),
    }
    Ok(())
}

fn validate_http_url(url: &str) -> Result<()> {
    let lower = url.trim().to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(anyhow!("URL '{}' must start with http:// or https://", url));
    }
    validation::validate_url_scheme(url).map_err(|e| anyhow!(e.to_string()))?;
    Ok(())
}

fn validate_optional_template(template: &Option<String>) -> Result<()> {
    if let Some(t) = template {
        if !t.trim().is_empty() {
            validation::validate_variable_template(t).map_err(|e| anyhow!(e.to_string()))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AiService {
    client: reqwest::Client,
}

impl Default for AiService {
    fn default() -> Self {
        Self::new()
    }
}

impl AiService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Resolve config for the user and propose a bookmark. This is the
    /// entry point HTTP handlers should use.
    pub async fn propose_bookmark(
        &self,
        pool: &SqlitePool,
        user: &db::User,
        request: &ProposeRequest,
    ) -> Result<ProposeOutcome> {
        let config = resolve_effective_config(pool, user)
            .await
            .ok_or_else(|| anyhow!("AI is not available for your account"))?;
        self.propose_with_config(&config, request).await
    }

    /// Propose a bookmark with an explicit config (also used by tests and
    /// by any future surface that resolves config differently).
    pub async fn propose_with_config(
        &self,
        config: &ResolvedAiConfig,
        request: &ProposeRequest,
    ) -> Result<ProposeOutcome> {
        if request.message.trim().is_empty() {
            return Err(anyhow!("Description must not be empty"));
        }

        let mut messages: Vec<serde_json::Value> =
            vec![serde_json::json!({"role": "system", "content": SYSTEM_PROMPT})];

        // Replay prior turns (capped) so follow-ups keep context.
        for turn in request.history.iter().take(20) {
            let role = if turn.role == "assistant" {
                "assistant"
            } else {
                "user"
            };
            messages.push(serde_json::json!({"role": role, "content": turn.content}));
        }

        if let Some(prev) = &request.previous_proposal {
            let prev_json = serde_json::to_string(prev).unwrap_or_default();
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": prev_json,
            }));
        }

        messages.push(serde_json::json!({"role": "user", "content": request.message}));

        // Validation retry loop: a proposal is only ever returned to the
        // caller after parse_proposal (which validates it with the same
        // rules as the manual create form) accepts it. When validation
        // fails, the invalid response and the validation error are fed
        // back to the model and it tries again, behind the scenes, up to
        // MAX_PROPOSAL_ATTEMPTS total attempts.
        let mut last_error = String::new();
        for attempt in 1..=MAX_PROPOSAL_ATTEMPTS {
            let content = match self.call_model(config, &messages).await {
                Ok(content) => content,
                // Unusable envelope (e.g. empty content): counts as a
                // failed attempt, fed back like a validation failure.
                Err(ModelCallError::InvalidResponse(reason)) => {
                    last_error = reason;
                    if attempt < MAX_PROPOSAL_ATTEMPTS {
                        messages.push(serde_json::json!({
                            "role": "user",
                            "content": format!(
                                "Your previous response could not be used: {}. Respond with ONLY the corrected proposal JSON object, no commentary.",
                                last_error
                            ),
                        }));
                    }
                    continue;
                }
                // Transport / HTTP errors are not the model's output being
                // invalid; fail immediately instead of retrying.
                Err(ModelCallError::Fatal(e)) => return Err(e),
            };

            match parse_proposal(&content) {
                Ok(proposal) => {
                    return Ok(ProposeOutcome {
                        proposal,
                        model: config.model.clone(),
                        source: config.source,
                        attempts: attempt,
                    });
                }
                Err(e) => {
                    last_error = e.to_string();
                    if attempt < MAX_PROPOSAL_ATTEMPTS {
                        messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": content,
                        }));
                        messages.push(serde_json::json!({
                            "role": "user",
                            "content": format!(
                                "That proposal is invalid: {}. Fix the problem and respond with ONLY the corrected proposal JSON object, no commentary.",
                                last_error
                            ),
                        }));
                    }
                }
            }
        }

        Err(anyhow!(
            "AI could not produce a valid bookmark proposal after {} attempts (last error: {})",
            MAX_PROPOSAL_ATTEMPTS,
            last_error
        ))
    }

    /// One chat-completions call. Returns the assistant message content.
    async fn call_model(
        &self,
        config: &ResolvedAiConfig,
        messages: &[serde_json::Value],
    ) -> Result<String, ModelCallError> {
        let body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "temperature": 0.3,
            // Reasoning-style models spend tokens thinking before content;
            // keep this generous so the JSON answer is not truncated.
            "max_tokens": 2000,
        });

        let url = chat_completions_url(&config.base_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&config.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                ModelCallError::Fatal(anyhow!(e).context("Failed to reach the AI provider"))
            })?;

        let status = response.status();
        if !status.is_success() {
            // Read a small snippet for diagnostics; never includes our key
            // (we sent it in a header, not the body).
            let snippet: String = response
                .text()
                .await
                .unwrap_or_default()
                .chars()
                .take(300)
                .collect();
            return Err(ModelCallError::Fatal(anyhow!(
                "AI provider returned {}: {}",
                status,
                snippet
            )));
        }

        let payload: serde_json::Value = response.json().await.map_err(|e| {
            ModelCallError::InvalidResponse(format!("AI provider returned invalid JSON: {}", e))
        })?;

        payload
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .ok_or(ModelCallError::InvalidResponse(
                "AI provider returned an empty response".to_string(),
            ))
    }
}

/// How a single model call failed.
/// - `Fatal`: not worth retrying (transport error, HTTP error status).
/// - `InvalidResponse`: the provider answered but the envelope/content
///   was unusable; treated like a failed validation attempt.
enum ModelCallError {
    Fatal(anyhow::Error),
    InvalidResponse(String),
}
