// AI handlers: settings forms (user + admin) and the JSON proposal API.
//
// The proposal API (`POST /api/ai/bookmark/propose`) is intentionally a
// plain JSON endpoint, separate from the HTMX manage page that calls it
// today, so other surfaces (e.g. a browser extension, or an "edit with AI"
// flow) can reuse it with the caller's session cookie.

use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use super::common::{ErrorTemplate, SuccessTemplate};
use crate::{
    auth::middleware::{AdminUser, CurrentUser},
    db,
    error::AppError,
    services::ai_service::{self, BookmarkProposal, ChatTurn},
    validation,
};
use askama::Template;

// ---------------------------------------------------------------------------
// JSON API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ProposeApiRequest {
    pub message: String,
    #[serde(default)]
    pub history: Vec<ChatTurn>,
    #[serde(default)]
    pub previous_proposal: Option<BookmarkProposal>,
}

/// GET /api/ai/status — is AI usable by the current user, and from where?
pub async fn ai_status(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
) -> impl IntoResponse {
    match ai_service::resolve_effective_config(&state.db_pool, &current_user.0).await {
        Some(cfg) => Json(json!({
            "available": true,
            "source": cfg.source.as_str(),
            "model": cfg.model,
        })),
        None => Json(json!({ "available": false, "source": null, "model": null })),
    }
}

/// POST /api/ai/bookmark/propose — propose a bookmark from a description.
/// On follow-ups, send back `history` and `previous_proposal`.
pub async fn propose_bookmark(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Json(req): Json<ProposeApiRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.message.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Please describe the bookmark you want".to_string(),
        ));
    }
    if req.message.len() > 4000 {
        return Err(AppError::BadRequest(
            "Description is too long (max 4000 characters)".to_string(),
        ));
    }
    if req.history.len() > 20 {
        return Err(AppError::BadRequest(
            "Conversation history is too long".to_string(),
        ));
    }

    let config = ai_service::resolve_effective_config(&state.db_pool, &current_user.0)
        .await
        .ok_or_else(|| AppError::Forbidden(
            "AI is not available for your account. Add your own AI settings in Settings, or ask an admin to enable AI for you.".to_string(),
        ))?;

    let request = ai_service::ProposeRequest {
        message: req.message,
        history: req.history,
        previous_proposal: req.previous_proposal,
    };

    let outcome = state
        .ai_service
        .propose_with_config(&config, &request)
        .await
        .map_err(|e| AppError::Internal(format!("AI proposal failed: {}", e)))?;

    Ok(Json(json!({
        "proposal": outcome.proposal,
        "model": outcome.model,
        "source": outcome.source.as_str(),
    })))
}

// ---------------------------------------------------------------------------
// Shared form helpers
// ---------------------------------------------------------------------------

/// API-key form semantics (both settings forms): the stored key is never
/// rendered into a page, so an empty field means "keep the stored key",
/// an explicit clear flag removes it, and any other value replaces it.
fn resolve_api_key_field(
    submitted: &str,
    clear: &Option<String>,
    existing: Option<String>,
) -> Option<String> {
    if clear.is_some() {
        None
    } else if submitted.trim().is_empty() {
        existing
    } else {
        Some(submitted.trim().to_string())
    }
}

fn validate_base_url(base_url: &Option<String>) -> Result<(), AppError> {
    if let Some(url) = base_url {
        validation::validate_url_scheme(url)?;
        let lower = url.to_lowercase();
        if !lower.starts_with("http://") && !lower.starts_with("https://") {
            return Err(AppError::BadRequest(
                "AI base URL must start with http:// or https://".to_string(),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// User settings
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UserAiSettingsForm {
    #[serde(default)]
    pub enabled: Option<String>, // checkbox
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub clear_api_key: Option<String>,
    #[serde(default)]
    pub model: String,
}

/// POST /settings/ai — save the current user's own AI settings.
pub async fn update_user_ai_settings(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<UserAiSettingsForm>,
) -> Result<impl IntoResponse, AppError> {
    let existing = db::ai::get_user_ai_settings(&state.db_pool, current_user.0.id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?
        .unwrap_or(db::ai::UserAiSettings {
            enabled: true,
            ..Default::default()
        });

    let base_url = db::ai::blank_to_none(&form.base_url);
    validate_base_url(&base_url)?;
    let model = db::ai::blank_to_none(&form.model);
    let api_key =
        resolve_api_key_field(&form.api_key, &form.clear_api_key, existing.api_key.clone());

    let settings = db::ai::UserAiSettings {
        base_url,
        api_key,
        model,
        enabled: form.enabled.is_some(),
    };

    db::ai::upsert_user_ai_settings(&state.db_pool, current_user.0.id, &settings)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let template = SuccessTemplate {
        message: "AI settings saved. They apply only to your account.",
    };
    Ok(Html(template.render()?))
}

// ---------------------------------------------------------------------------
// Admin settings
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AdminAiSettingsForm {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub clear_api_key: Option<String>,
    #[serde(default)]
    pub model: String,
    pub access_mode: String,
    /// Repeated checkbox values from the selected-users list.
    #[serde(default)]
    pub allowed_user_ids: Vec<i64>,
}

/// POST /admin/ai-settings — save the shared AI config and access mode.
pub async fn update_admin_ai_settings(
    _admin_user: AdminUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<AdminAiSettingsForm>,
) -> Result<impl IntoResponse, AppError> {
    if !db::ai::is_valid_access_mode(&form.access_mode) {
        let template = ErrorTemplate {
            message: "Invalid AI access mode",
        };
        return Ok(Html(template.render()?));
    }

    let existing = db::ai::get_global_ai_settings(&state.db_pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let base_url = db::ai::blank_to_none(&form.base_url);
    validate_base_url(&base_url)?;
    let model = db::ai::blank_to_none(&form.model);
    let api_key =
        resolve_api_key_field(&form.api_key, &form.clear_api_key, existing.api_key.clone());

    let settings = db::ai::GlobalAiSettings {
        base_url,
        api_key,
        model,
        access_mode: form.access_mode.clone(),
    };

    db::ai::upsert_global_ai_settings(&state.db_pool, &settings)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    db::ai::set_allowed_users(&state.db_pool, &form.allowed_user_ids)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let template = SuccessTemplate {
        message: "AI settings saved. Blank fields fall back to the BRUNNYLOL_AI_* / OPENAI_* environment variables.",
    };
    Ok(Html(template.render()?))
}
