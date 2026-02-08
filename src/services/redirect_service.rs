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

            remaining = &remaining[1..];

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
        let remaining = &input[1..];
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

    /// Parse nested path from query, detecting suffix at any level
    /// Examples:
    ///   "nested? sub1" -> (["nested", "sub1"], Form, None)
    ///   "nested sub1?" -> (["nested", "sub1"], Form, None)
    ///   "nested sub1 sub2?" -> (["nested", "sub1", "sub2"], Form, None)
    ///   "nested$ sub1 $var=val" -> (["nested", "sub1"], Named, Some("$var=val"))
    fn parse_nested_path(query: &str) -> (Vec<String>, UsageMode, Option<String>) {
        let parts: Vec<&str> = query.split_whitespace().collect();
        if parts.is_empty() {
            return (vec![], UsageMode::Direct, None);
        }

        // Find first part with suffix
        let mut suffix_index = None;
        let mut usage_mode = UsageMode::Direct;

        for (i, part) in parts.iter().enumerate() {
            let (_, mode) = Self::parse_alias_and_mode(part);
            if !matches!(mode, UsageMode::Direct) {
                suffix_index = Some(i);
                usage_mode = mode;
                break;
            }
        }

        match suffix_index {
            Some(idx) => {
                // Build path up to and including the suffix position
                let mut path: Vec<String> = parts[..idx]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                // Strip suffix from the element at suffix position
                let (clean, _) = Self::parse_alias_and_mode(parts[idx]);
                path.push(clean.to_string());

                // Remaining parts after suffix become query
                let remaining = (idx + 1 < parts.len())
                    .then(|| parts[idx + 1..].join(" "));

                (path, usage_mode, remaining)
            }
            None => {
                // No suffix found - single alias in Direct mode
                let remaining = (parts.len() > 1)
                    .then(|| parts[1..].join(" "));
                (vec![parts[0].to_string()], UsageMode::Direct, remaining)
            }
        }
    }

    /// Recursively resolve nested command path
    fn resolve_nested_command<'a>(
        command: &'a Command,
        path: &[String],
        start_index: usize,
    ) -> Option<&'a Command> {
        if start_index >= path.len() {
            return Some(command);
        }

        match command {
            Command::Nested { children, .. } => {
                if let Some(child) = children.get(&path[start_index]) {
                    Self::resolve_nested_command(child, path, start_index + 1)
                } else {
                    None
                }
            }
            Command::Variable { .. } => None,
        }
    }

    /// Resolve a search query to a redirect result
    pub async fn resolve_redirect(
        &self,
        query: &str,
        user: Option<&db::User>,
        global_bookmarks: &HashMap<String, Command>,
        default_alias: Option<&str>,
    ) -> Result<RedirectResult, AppError> {
        // Parse query to detect nested paths with suffixes
        let (path, usage_mode, remaining_query) = Self::parse_nested_path(query);

        // Handle empty query
        if path.is_empty() {
            return Err(AppError::NotFound("Empty query".to_string()));
        }

        // Handle nested paths (multi-segment paths or single segment with specific modes)
        if path.len() > 1 || !matches!(usage_mode, UsageMode::Direct) {
            return self.resolve_nested_redirect(
                &path,
                usage_mode,
                remaining_query.as_deref(),
                user,
                global_bookmarks,
            ).await;
        }

        // Fall back to original single-alias logic for backward compatibility
        let bookmark_alias = &path[0];
        let query_part = remaining_query.as_deref().unwrap_or("");


        // Load user bookmarks and disabled globals
        let (user_bookmarks, disabled_globals) = if let Some(user) = user {
            let bookmarks = db::bookmarks::load_user_bookmarks(&self.pool, user.id)
                .await
                .ok();

            let disabled = db::get_disabled_global_aliases(&self.pool, user.id).await;

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

    /// Resolve nested command with suffix support
    async fn resolve_nested_redirect(
        &self,
        path: &[String],
        usage_mode: UsageMode,
        remaining_query: Option<&str>,
        user: Option<&db::User>,
        global_bookmarks: &HashMap<String, Command>,
    ) -> Result<RedirectResult, AppError> {
        if path.is_empty() {
            return Err(AppError::NotFound("Empty path".to_string()));
        }

        // Load user bookmarks and disabled globals
        let (user_bookmarks, disabled_globals) = if let Some(user) = user {
            let bookmarks = db::bookmarks::load_user_bookmarks(&self.pool, user.id)
                .await
                .ok();
            let disabled = db::get_disabled_global_aliases(&self.pool, user.id).await;
            (bookmarks, disabled)
        } else {
            (None, std::collections::HashSet::new())
        };

        // Find root command
        let root_alias = &path[0];
        let root_command = user_bookmarks
            .as_ref()
            .and_then(|user_map| user_map.get(root_alias))
            .or_else(|| {
                if disabled_globals.contains(root_alias.as_str()) {
                    None
                } else {
                    global_bookmarks.get(root_alias)
                }
            })
            .ok_or_else(|| AppError::NotFound(format!("Unknown alias: '{}'", root_alias)))?;

        // Resolve nested path
        let final_command = if path.len() > 1 {
            Self::resolve_nested_command(root_command, path, 1)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "Unknown nested path: '{}'",
                        path[1..].join("/")
                    ))
                })?
        } else {
            root_command
        };

        // Handle based on usage mode and command type
        match (final_command, usage_mode) {
            // Form or Chained mode - redirect to /f/path/to/command
            (_, UsageMode::Form | UsageMode::Chained) => {
                let mut form_url = format!("/f/{}", path.join("/"));

                if matches!(usage_mode, UsageMode::Chained) {
                    if let Some(query) = remaining_query {
                        let (vars, _) = Self::parse_named_variables(query);
                        if !vars.is_empty() {
                            form_url = format!("{}?{}", form_url, Self::build_query_string(&vars));
                        }
                    }
                }

                Ok(RedirectResult::InternalPath(form_url))
            }
            // Named mode with Variable command
            (Command::Variable { template, base_url, .. }, UsageMode::Named) => {
                let query = remaining_query.unwrap_or("");
                let (mut vars, remaining) = Self::parse_named_variables(query);

                if let Some(rem) = remaining {
                    if !rem.is_empty() {
                        vars.insert("query".to_string(), rem);
                    }
                }

                vars.insert("url".to_string(), base_url.clone());

                let resolver = TemplateResolver::new();
                let missing = resolver.validate_variables(template, &vars).unwrap_or_default();

                if !missing.is_empty() {
                    return Ok(RedirectResult::InternalPath(format!(
                        "/f/{}?{}",
                        path.join("/"),
                        Self::build_query_string(&vars)
                    )));
                }

                match resolver.resolve(template, &vars) {
                    Ok(url) => {
                        if url.starts_with("http://") || url.starts_with("https://") {
                            Ok(RedirectResult::ExternalUrl(url))
                        } else {
                            Ok(RedirectResult::InternalPath(url))
                        }
                    }
                    Err(_) => Ok(RedirectResult::InternalPath(format!(
                        "/f/{}?{}",
                        path.join("/"),
                        Self::build_query_string(&vars)
                    ))),
                }
            }
            // Direct mode - resolve with remaining query
            (command, UsageMode::Direct) => {
                let query = remaining_query.unwrap_or("");
                let redirect_url = command.get_redirect_url(query);

                if redirect_url.is_empty() {
                    return Err(AppError::NotFound(format!(
                        "Failed to resolve nested command: '{}'",
                        path.join("/")
                    )));
                }

                if redirect_url.starts_with("http://") || redirect_url.starts_with("https://") {
                    Ok(RedirectResult::ExternalUrl(redirect_url))
                } else {
                    Ok(RedirectResult::InternalPath(redirect_url))
                }
            }
            // Named mode with Nested command - not supported
            (Command::Nested { .. }, UsageMode::Named) => Err(AppError::BadRequest(
                "Named mode ($) not supported for nested commands without variables".to_string(),
            )),
        }
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
