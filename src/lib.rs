extern crate clap;

pub mod domain;
pub mod config;
mod error;
pub mod db;
mod auth;
mod handlers;
pub mod services;
pub mod validation;
mod security;

use askama::Template;
use axum::{
    extract::{DefaultBodyLimit, Query, State},
    middleware,
    response::{Html, IntoResponse, Redirect},
    routing::{delete, get, post},
    Router,
};
use tower_http::services::ServeDir;
use domain::Command;
use error::AppError;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Arc};
use clap::Arg;

const DEFAULT_ALIAS: &str = "g";

// Template structs
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    has_user: bool,
    is_admin: bool,
}

#[derive(Template)]
#[template(path = "help.html")]
struct HelpTemplate {
    alias_to_description: Vec<(String, Vec<String>, bool)>, // (alias, description_parts, is_disabled)
    personal_aliases: Vec<(String, Vec<String>)>,
    has_user: bool,
    is_admin: bool,
}

// Query parameter struct for search
#[derive(Deserialize)]
struct SearchParams {
    #[serde(default)]
    q: String,
    default: Option<String>,
}

// Usage mode for bookmark aliases
enum UsageMode {
    Direct,   // alias value
    Form,     // alias?
    Named,    // alias$
    Chained,  // alias?$
}

// Parse alias and detect usage mode from suffix
fn parse_alias_and_mode(input: &str) -> (&str, UsageMode) {
    // Single-char aliases can end in special characters
    if input.len() == 1 {
        return (input, UsageMode::Direct);
    }

    // Check for ?$ or $? suffix (chained mode - both orders)
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

// Parse named variables from query string
// Example: "$page=home; $repo=rust; rest of query" -> ({page: "home", repo: "rust"}, Some("rest of query"))
fn parse_named_variables(query: &str) -> (HashMap<String, String>, Option<String>) {
    let mut variables = HashMap::new();
    let mut remaining = query;

    loop {
        remaining = remaining.trim_start();

        // Check if starts with $
        if !remaining.starts_with('$') {
            break;
        }

        // Find the variable name (between $ and =)
        if let Some(eq_pos) = remaining.find('=') {
            let var_name = remaining[1..eq_pos].trim().to_string();
            remaining = &remaining[eq_pos + 1..];

            // Parse the value (quoted or until semicolon/end)
            let (value, rest) = if remaining.trim_start().starts_with('"') {
                // Quoted value with escape support
                remaining = remaining.trim_start();
                remaining = &remaining[1..]; // Skip opening quote

                let mut value = String::new();
                let mut chars = remaining.chars();
                let mut escaped = false;
                let mut bytes_consumed = 0;
                let mut found_close = false;

                while let Some(ch) = chars.next() {
                    bytes_consumed += ch.len_utf8();
                    if escaped {
                        value.push(ch);
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        // Found closing quote
                        found_close = true;
                        break;
                    } else {
                        value.push(ch);
                    }
                }

                if found_close {
                    (value, &remaining[bytes_consumed..])
                } else {
                    // No closing quote found
                    (value, "")
                }
            } else {
                // Unquoted value until semicolon
                if let Some(semi) = remaining.find(';') {
                    let val = remaining[..semi].trim().to_string();
                    (val, &remaining[semi + 1..])
                } else {
                    (remaining.trim().to_string(), "")
                }
            };

            variables.insert(var_name, value);
            remaining = rest;

            // Skip semicolon if present
            remaining = remaining.trim_start();
            if remaining.starts_with(';') {
                remaining = &remaining[1..];
            }
        } else {
            break;
        }
    }

    let remaining_query = if remaining.is_empty() {
        None
    } else {
        Some(remaining.trim().to_string())
    };

    (variables, remaining_query)
}

// Application state
pub struct AppState {
    pub alias_to_bookmark_map: Arc<tokio::sync::RwLock<HashMap<String, Command>>>,
    pub default_alias: String,
    pub db_pool: SqlitePool,
    pub bookmark_service: std::sync::Arc<services::bookmark_service::BookmarkService>,
}

// Route handlers
async fn index(
    optional_user: auth::middleware::OptionalUser,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, AppError> {
    // If query param is present, redirect to search endpoint
    if !params.q.is_empty() {
        let search_url = format!("/search?q={}", urlencoding::encode(&params.q));
        return Ok(Redirect::to(&search_url).into_response());
    }

    let (has_user, is_admin) = if let Some(ref user) = optional_user.0 {
        (true, user.is_admin)
    } else {
        (false, false)
    };

    let template = IndexTemplate { has_user, is_admin };
    Ok(Html(template.render()?).into_response())
}

// Helper function to split command descriptions by pipe
fn split_command_description(description: &str) -> Vec<String> {
    description.split('|').map(|s| s.to_string()).collect()
}

async fn help(
    optional_user: auth::middleware::OptionalUser,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // Load disabled aliases if logged in
    let (personal_aliases, disabled_set, has_user, is_admin) = if let Some(ref user) = optional_user.0 {
        let user_bookmarks = db::bookmarks::load_user_bookmarks(&state.db_pool, user.id)
            .await
            .ok();

        let aliases = user_bookmarks
            .map(|bookmarks| {
                bookmarks
                    .iter()
                    .map(|(alias, cmd)| {
                        let parts = split_command_description(&cmd.description());
                        (alias.clone(), parts)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Get disabled global aliases
        let overrides = db::get_user_overrides(&state.db_pool, user.id)
            .await
            .ok()
            .unwrap_or_default();

        let disabled: std::collections::HashSet<String> = overrides
            .iter()
            .filter(|(_, is_disabled, _, _)| *is_disabled)
            .map(|(alias, _, _, _)| alias.clone())
            .collect();

        (aliases, disabled, true, user.is_admin)
    } else {
        (Vec::new(), std::collections::HashSet::new(), false, false)
    };

    // Pre-process global descriptions with disabled status
    let bookmark_map = state.alias_to_bookmark_map.read().await;
    let alias_to_description: Vec<(String, Vec<String>, bool)> = bookmark_map
        .iter()
        .map(|(alias, cmd)| {
            let parts = split_command_description(&cmd.description());
            let is_disabled = disabled_set.contains(alias);
            (alias.clone(), parts, is_disabled)
        })
        .collect();

    let template = HelpTemplate {
        alias_to_description,
        personal_aliases,
        has_user,
        is_admin,
    };
    Ok(Html(template.render()?))
}

async fn redirect(
    optional_user: auth::middleware::OptionalUser,
    Query(params): Query<SearchParams>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let mut splitted = params.q.splitn(2, ' ');
    let bookmark_alias_raw = splitted.next().unwrap_or("");
    let query = splitted.next().unwrap_or_default();

    // Parse alias and detect usage mode from suffix
    let (bookmark_alias, usage_mode) = parse_alias_and_mode(bookmark_alias_raw);

    // Handle Form and Chained modes - redirect to /f/{alias}
    if matches!(usage_mode, UsageMode::Form | UsageMode::Chained) {
        let mut form_url = format!("/f/{}", bookmark_alias);

        // For chained mode, parse named variables and add as query params
        if matches!(usage_mode, UsageMode::Chained) {
            let (vars, _) = parse_named_variables(query);
            if !vars.is_empty() {
                let query_string: String = vars
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                form_url = format!("{}?{}", form_url, query_string);
            }
        }

        return Redirect::to(&form_url).into_response();
    }

    // Load user bookmarks and disabled global bookmarks if logged in
    let (user_bookmarks, disabled_globals) = if let Some(user) = &optional_user.0 {
        let bookmarks = db::bookmarks::load_user_bookmarks(&state.db_pool, user.id)
            .await
            .ok();

        let overrides = db::get_user_overrides(&state.db_pool, user.id)
            .await
            .ok()
            .unwrap_or_default();

        let disabled: std::collections::HashSet<String> = overrides
            .iter()
            .filter(|(_, is_disabled, _, _)| *is_disabled)
            .map(|(alias, _, _, _)| alias.clone())
            .collect();

        (bookmarks, disabled)
    } else {
        (None, std::collections::HashSet::new())
    };

    // Try user bookmarks first (if logged in), then global bookmarks (if not disabled)
    let bookmark_map = state.alias_to_bookmark_map.read().await;
    let command = user_bookmarks
        .as_ref()
        .and_then(|user_map| user_map.get(bookmark_alias))
        .or_else(|| {
            // Check if global bookmark is disabled
            if disabled_globals.contains(bookmark_alias) {
                None
            } else {
                bookmark_map.get(bookmark_alias)
            }
        }).cloned();

    let redirect_url = match command {
        Some(Command::Variable { ref base_url, ref template, .. }) if matches!(usage_mode, UsageMode::Named) => {
            // Handle named mode
            let (mut vars, remaining) = parse_named_variables(query);

            // Add remaining query to "query" variable if it exists
            if let Some(rem) = remaining {
                if !rem.is_empty() {
                    vars.insert("query".to_string(), rem);
                }
            }

            // Check if all required variables are provided
            let resolver = domain::template::TemplateResolver::new();
            let missing = resolver.validate_variables(template, &vars).unwrap_or_default();

            if !missing.is_empty() {
                // Redirect to form page with prefilled values
                let query_string: String = vars
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                return Redirect::to(&format!("/f/{}?{}", bookmark_alias, query_string)).into_response();
            }

            // Try to resolve - if validation fails (e.g., strict options), redirect to form
            match resolver.resolve(template, &vars) {
                Ok(url) => url,
                Err(_) => {
                    // Validation error - redirect to form page
                    let query_string: String = vars
                        .iter()
                        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                        .collect::<Vec<_>>()
                        .join("&");
                    return Redirect::to(&format!("/f/{}?{}", bookmark_alias, query_string)).into_response();
                }
            }
        }
        Some(Command::Variable { ref template, ref base_url, .. }) => {
            // Direct mode with Variable command - validate before resolving

            // If query is empty, just return base URL
            if query.trim().is_empty() {
                return Redirect::to(base_url).into_response();
            }

            // Build variable map
            let mut vars = HashMap::new();
            let template_vars = template.variables();
            let has_query_var = template_vars.iter().any(|v| v.name == "query");

            // Add {url} as built-in variable
            vars.insert("url".to_string(), base_url.clone());

            // Filter out built-in variables for positional mapping
            let user_vars: Vec<_> = template_vars.iter()
                .filter(|v| v.name != "url")
                .collect();

            if has_query_var && user_vars.len() == 1 && user_vars[0].name == "query" {
                vars.insert("query".to_string(), query.to_string());
            } else if !user_vars.is_empty() {
                let query_parts: Vec<&str> = query.split_whitespace().collect();
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

            // Validate variables
            let resolver = domain::template::TemplateResolver::new();
            let missing = resolver.validate_variables(template, &vars).unwrap_or_default();

            if !missing.is_empty() {
                // Missing required variables - redirect to form
                return Redirect::to(&format!("/f/{}", bookmark_alias)).into_response();
            }

            // Try to resolve - if validation fails (strict options), redirect to form
            match resolver.resolve(template, &vars) {
                Ok(url) => url,
                Err(_) => {
                    // Validation failed - redirect to form with current values
                    let query_string: String = vars
                        .iter()
                        .filter(|(k, _)| *k != "url") // Don't include url in query params
                        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                        .collect::<Vec<_>>()
                        .join("&");
                    return Redirect::to(&format!("/f/{}?{}", bookmark_alias, query_string)).into_response();
                }
            }
        }
        Some(bookmark) => bookmark.get_redirect_url(query),
        None => {
            // Check if user has a custom default alias preference
            let user_default = optional_user.0.as_ref().and_then(|u| u.default_alias.as_deref());

            let default_alias = params
                .default
                .as_deref()
                .or(user_default)
                .unwrap_or(""); // Empty string means no default (will return 404)

            // If no default alias is set, return 404
            if default_alias.is_empty() {
                return AppError::NotFound(format!("Unknown alias: '{}'", bookmark_alias)).into_response();
            }

            // Try user bookmarks first for default alias too
            let default_command = user_bookmarks
                .as_ref()
                .and_then(|user_map| user_map.get(default_alias))
                .or_else(|| {
                    if disabled_globals.contains(default_alias) {
                        None
                    } else {
                        bookmark_map.get(default_alias)
                    }
                }).cloned();

            default_command
                .map(|cmd| cmd.get_redirect_url(&params.q))
                .unwrap_or_else(|| {
                    // Default alias not found either - return 404
                    format!("/404?alias={}", urlencoding::encode(bookmark_alias))
                })
        }
    };

    Redirect::to(&redirect_url).into_response()
}

// Public function to create the router
pub async fn create_router() -> Router {
    // Parse CLI arguments
    let matches = clap::Command::new("Brunnylol")
        .arg(
            Arg::new("commands")
                .short('c')
                .long("commands")
                .value_name("COMMANDS")
                .help("Path to a YAML file containing commands"),
        )
        .arg(
            Arg::new("default_alias")
                .short('a')
                .long("default_alias")
                .value_name("DEFAULT_ALIAS")
                .help("Default alias to use when none is provided"),
        )
        .arg(
            Arg::new("database")
                .short('d')
                .long("database")
                .value_name("DATABASE")
                .help("Path to SQLite database file (default: brunnylol.db, env: BRUNNYLOL_DB)"),
        )
        .get_matches();

    let default_alias = matches
        .get_one("default_alias")
        .map(|c: &String| c.as_str())
        .unwrap_or(DEFAULT_ALIAS)
        .to_string();

    // Priority: CLI arg > ENV var > Default value
    let env_db = std::env::var("BRUNNYLOL_DB").ok();
    let db_path = matches
        .get_one::<String>("database")
        .map(|s| s.as_str())
        .or_else(|| env_db.as_deref())
        .unwrap_or("brunnylol.db");

    eprintln!("Using database: {}", db_path);

    // Initialize database
    let db_pool = db::init_db(db_path)
        .await
        .expect("Failed to initialize database");

    // Seed test user in development (but not for in-memory databases used in tests)
    #[cfg(debug_assertions)]
    if db_path != ":memory:" {
        let _ = db::seed::seed_test_user(&db_pool).await;
    }

    // Create bookmark service
    let bookmark_service = std::sync::Arc::new(services::bookmark_service::BookmarkService::new(db_pool.clone()));

    // Seed global bookmarks from embedded commands.yml if DB is empty
    match bookmark_service.seed_global_bookmarks().await {
        Ok(count) => {
            if count > 0 {
                println!("Seeded {} global bookmarks from commands.yml", count);
            }
        }
        Err(e) => {
            eprintln!("Error: Failed to seed global bookmarks: {}", e);
        }
    }

    // Load global bookmarks from database
    let alias_to_bookmark_map = bookmark_service.load_global_bookmarks()
        .await
        .expect("Failed to load global bookmarks");

    let state = Arc::new(AppState {
        alias_to_bookmark_map: Arc::new(tokio::sync::RwLock::new(alias_to_bookmark_map)),
        default_alias,
        db_pool: db_pool.clone(),
        bookmark_service,
    });

    Router::new()
        // Public routes
        .route("/", get(index))
        .route("/help", get(help))
        .route("/search", get(redirect))

        // Auth routes
        .route("/login", get(handlers::auth::login_page).post(handlers::auth::login_submit))
        .route("/register", get(handlers::auth::register_page).post(handlers::auth::register_submit))
        .route("/logout", post(handlers::auth::logout))

        // Bookmark management routes (require authentication)
        .route("/manage", get(handlers::bookmarks::manage_page))
        .route("/manage/bookmark", post(handlers::bookmarks::create_bookmark))
        .route("/manage/bookmark/{id}",
            delete(handlers::bookmarks::delete_bookmark)
            .put(handlers::bookmarks::update_bookmark))
        .route("/manage/bookmark/{id}/nested", post(handlers::bookmarks::create_nested_bookmark))
        .route("/manage/bookmark/{id}/nested/list", get(handlers::bookmarks::list_nested_bookmarks))
        .route("/manage/nested/{id}", delete(handlers::bookmarks::delete_nested_bookmark))
        .route("/manage/override", post(handlers::bookmarks::toggle_global_bookmark))
        .route("/manage/import",
            post(handlers::bookmarks::import_bookmarks)
                // Limit import request body to 2MB (1MB for content + overhead for form encoding)
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        )
        .route("/manage/export", get(handlers::bookmarks::export_bookmarks))
        .route("/manage/bookmarks/bulk-delete", post(handlers::bookmarks::bulk_delete_bookmarks))
        .route("/manage/overrides/bulk-disable", post(handlers::bookmarks::bulk_toggle_global))
        .route("/manage/fork-global", post(handlers::bookmarks::fork_global_bookmark))

        // User settings routes (require authentication)
        .route("/settings", get(handlers::auth::settings_page))
        .route("/settings/username", post(handlers::auth::change_username))
        .route("/settings/password", post(handlers::auth::change_password))
        .route("/settings/default-alias", post(handlers::auth::change_default_alias))

        // Variable form routes (GET shows form, POST submits)
        .route("/f/{alias}", get(handlers::variable_form::show_variable_form)
                             .post(handlers::variable_form::submit_variable_form))

        // Admin routes (require admin authentication)
        .route("/admin", get(handlers::admin::admin_page))
        .route("/admin/cleanup-sessions", post(handlers::admin::cleanup_sessions))
        .route("/admin/create-user", post(handlers::admin::create_user))
        .route("/admin/reload-global", post(handlers::admin::reload_global_bookmarks))

        // Serve static files (JavaScript, CSS, etc.)
        .nest_service("/static", ServeDir::new("static"))

        .with_state(state)
        // Apply security headers to all responses
        .layer(middleware::from_fn(security::security_headers))
}
