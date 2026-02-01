// Redirect service - Business logic for handling bookmark redirects

use crate::{
    db,
    domain::{template::TemplateResolver, Command},
    error::AppError,
};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Usage mode for bookmark aliases
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UsageMode {
    Direct,   // alias value
    Form,     // alias?
    Named,    // alias$
    Chained,  // alias?$ or alias$?
}

/// Result of redirect resolution
#[derive(Debug)]
pub enum RedirectResult {
    /// External URL redirect
    ExternalUrl(String),
    /// Internal path redirect
    InternalPath(String),
    /// Not found error
    NotFound(String),
}

/// Service for resolving bookmark redirects
pub struct RedirectService {
    pool: SqlitePool,
}

impl RedirectService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Parse alias and detect usage mode from suffix
    pub fn parse_alias_and_mode(input: &str) -> (&str, UsageMode) {
        if input.len() == 1 {
            return (input, UsageMode::Direct);
        }

        // Check for ?$ or $? suffix (chained mode)
        if input.ends_with("?$") || input.ends_with("$?") {
            return (&input[..input.len() - 2], UsageMode::Chained);
        }

        // Check for ? or $ suffix
        if let Some(ch) = input.chars().last() {
            match ch {
                '?' => (&input[..input.len() - 1], UsageMode::Form),
                '$' => (&input[..input.len() - 1], UsageMode::Named),
                _ => (input, UsageMode::Direct),
            }
        } else {
            (input, UsageMode::Direct)
        }
    }

    /// Parse named variables from query string ($key="value" or $key=value format)
    pub fn parse_named_variables(input: &str) -> (HashMap<String, String>, Option<String>) {
        let mut vars = HashMap::new();
        let mut remaining = input;

        loop {
            remaining = remaining.trim_start();

            if !remaining.starts_with('$') {
                break;
            }

            remaining = &remaining[1..]; // Skip $

            // Find = sign
            if let Some(eq_pos) = remaining.find('=') {
                let key = remaining[..eq_pos].trim();
                remaining = &remaining[eq_pos + 1..];

                // Parse value (quoted or unquoted)
                let (value, rest) = if remaining.starts_with('"') {
                    Self::parse_quoted_value(remaining)
                } else {
                    Self::parse_unquoted_value(remaining)
                };

                vars.insert(key.to_string(), value);
                remaining = rest;
            } else {
                break;
            }
        }

        let remaining_query = if remaining.is_empty() {
            None
        } else {
            Some(remaining.to_string())
        };

        (vars, remaining_query)
    }

    fn parse_quoted_value(input: &str) -> (String, &str) {
        let remaining = &input[1..]; // Skip opening quote
        let mut value = String::new();
        let mut chars = remaining.chars();
        let mut escaped = false;
        let mut bytes_consumed = 0;
        let mut found_close = false;

        #[allow(clippy::while_let_on_iterator)]
        while let Some(ch) = chars.next() {
            bytes_consumed += ch.len_utf8();
            if escaped {
                value.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                found_close = true;
                break;
            } else {
                value.push(ch);
            }
        }

        if found_close {
            (value, &remaining[bytes_consumed..])
        } else {
            (value, "")
        }
    }

    fn parse_unquoted_value(input: &str) -> (String, &str) {
        if let Some(semi_pos) = input.find(';') {
            (input[..semi_pos].trim().to_string(), &input[semi_pos + 1..])
        } else {
            (input.trim().to_string(), "")
        }
    }

    fn build_query_string(vars: &HashMap<String, String>) -> String {
        vars.iter()
            .filter(|(k, _)| *k != "url")
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Resolve a search query to a redirect result
    pub async fn resolve_redirect(
        &self,
        query: &str,
        user: Option<&db::User>,
        global_bookmarks: &HashMap<String, Command>,
        default_alias: Option<&str>,
    ) -> Result<RedirectResult, AppError> {
        let mut splitted = query.splitn(2, ' ');
        let bookmark_alias_raw = splitted.next().unwrap_or("");
        let query_part = splitted.next().unwrap_or_default();

        // Parse alias and detect usage mode
        let (bookmark_alias, usage_mode) = Self::parse_alias_and_mode(bookmark_alias_raw);

        // Handle Form and Chained modes - redirect to /f/{alias}
        if matches!(usage_mode, UsageMode::Form | UsageMode::Chained) {
            let mut form_url = format!("/f/{}", bookmark_alias);

            if matches!(usage_mode, UsageMode::Chained) {
                let (vars, _) = Self::parse_named_variables(query_part);
                if !vars.is_empty() {
                    form_url = format!("{}?{}", form_url, Self::build_query_string(&vars));
                }
            }

            return Ok(RedirectResult::InternalPath(form_url));
        }

        // Load user bookmarks and disabled globals
        let (user_bookmarks, disabled_globals) = if let Some(user) = user {
            let bookmarks = db::bookmarks::load_user_bookmarks(&self.pool, user.id)
                .await
                .ok();

            let disabled = self.load_disabled_globals(user.id).await;

            (bookmarks, disabled)
        } else {
            (None, std::collections::HashSet::new())
        };

        // Find the command
        let command = user_bookmarks
            .as_ref()
            .and_then(|user_map| user_map.get(bookmark_alias))
            .or_else(|| {
                if disabled_globals.contains(bookmark_alias) {
                    None
                } else {
                    global_bookmarks.get(bookmark_alias)
                }
            })
            .cloned();

        let redirect_url = match command {
            Some(Command::Variable { ref template, .. }) if matches!(usage_mode, UsageMode::Named) => {
                // Named mode
                let (mut vars, remaining) = Self::parse_named_variables(query_part);

                if let Some(rem) = remaining {
                    if !rem.is_empty() {
                        vars.insert("query".to_string(), rem);
                    }
                }

                let resolver = TemplateResolver::new();
                let missing = resolver.validate_variables(template, &vars).unwrap_or_default();

                if !missing.is_empty() {
                    return Ok(RedirectResult::InternalPath(format!("/f/{}?{}", bookmark_alias, Self::build_query_string(&vars))));
                }

                match resolver.resolve(template, &vars) {
                    Ok(url) => url,
                    Err(_) => {
                        return Ok(RedirectResult::InternalPath(format!("/f/{}?{}", bookmark_alias, Self::build_query_string(&vars))));
                    }
                }
            }
            Some(Command::Variable { ref template, ref base_url, .. }) => {
                // Direct mode
                if query_part.trim().is_empty() {
                    return Ok(if base_url.starts_with("http://") || base_url.starts_with("https://") {
                        RedirectResult::ExternalUrl(base_url.clone())
                    } else {
                        RedirectResult::InternalPath(base_url.clone())
                    });
                }

                let mut vars = HashMap::new();
                let template_vars = template.variables();
                let has_query_var = template_vars.iter().any(|v| v.name == "query");

                vars.insert("url".to_string(), base_url.clone());

                let user_vars: Vec<_> = template_vars.iter()
                    .filter(|v| v.name != "url")
                    .collect();

                if has_query_var && user_vars.len() == 1 && user_vars[0].name == "query" {
                    vars.insert("query".to_string(), query_part.to_string());
                } else if !user_vars.is_empty() {
                    let query_parts: Vec<&str> = query_part.split_whitespace().collect();
                    for (i, var) in user_vars.iter().enumerate() {
                        if i < query_parts.len() {
                            vars.insert(var.name.clone(), query_parts[i].to_string());
                        }
                    }
                    if query_parts.len() > user_vars.len() && has_query_var {
                        let remaining = query_parts[user_vars.len()..].join(" ");
                        vars.insert("query".to_string(), remaining);
                    }
                }

                let resolver = TemplateResolver::new();
                let missing = resolver.validate_variables(template, &vars).unwrap_or_default();

                if !missing.is_empty() {
                    return Ok(RedirectResult::InternalPath(format!("/f/{}", bookmark_alias)));
                }

                match resolver.resolve(template, &vars) {
                    Ok(url) => url,
                    Err(_) => {
                        return Ok(RedirectResult::InternalPath(format!("/f/{}?{}", bookmark_alias, Self::build_query_string(&vars))));
                    }
                }
            }
            Some(bookmark) => bookmark.get_redirect_url(query_part),
            None => {
                // Try default alias
                let user_default = user.and_then(|u| u.default_alias.as_deref());
                let default = default_alias.or(user_default).unwrap_or("");

                if default.is_empty() {
                    return Err(AppError::NotFound(format!("Unknown alias: '{}'", bookmark_alias)));
                }

                let default_command = user_bookmarks
                    .as_ref()
                    .and_then(|user_map| user_map.get(default))
                    .or_else(|| {
                        if disabled_globals.contains(default) {
                            None
                        } else {
                            global_bookmarks.get(default)
                        }
                    })
                    .cloned();

                default_command
                    .map(|cmd| cmd.get_redirect_url(query))
                    .unwrap_or_else(|| format!("/404?alias={}", urlencoding::encode(bookmark_alias)))
            }
        };

        // Classify redirect type
        if redirect_url.starts_with("http://") || redirect_url.starts_with("https://") {
            Ok(RedirectResult::ExternalUrl(redirect_url))
        } else {
            Ok(RedirectResult::InternalPath(redirect_url))
        }
    }

    async fn load_disabled_globals(&self, user_id: i64) -> std::collections::HashSet<String> {
        db::get_user_overrides(&self.pool, user_id)
            .await
            .ok()
            .unwrap_or_default()
            .iter()
            .filter(|(_, is_disabled, _, _)| *is_disabled)
            .map(|(alias, _, _, _)| alias.clone())
            .collect()
    }
}

impl RedirectResult {
    pub fn into_response(self) -> axum::response::Response {
        use axum::http::{StatusCode, header};
        use axum::response::{IntoResponse, Redirect};

        match self {
            RedirectResult::ExternalUrl(url) => {
                (StatusCode::SEE_OTHER, [(header::LOCATION, url)]).into_response()
            }
            RedirectResult::InternalPath(path) => {
                Redirect::to(&path).into_response()
            }
            RedirectResult::NotFound(alias) => {
                AppError::NotFound(format!("Unknown alias: '{}'", alias)).into_response()
            }
        }
    }
}
