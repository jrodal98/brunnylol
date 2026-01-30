extern crate clap;

mod domain;
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
    q: String,
    default: Option<String>,
}

// Application state
pub struct AppState {
    pub alias_to_bookmark_map: HashMap<String, Command>,
    pub default_alias: String,
    pub db_pool: SqlitePool,
    pub bookmark_service: std::sync::Arc<services::bookmark_service::BookmarkService>,
}

// Route handlers
async fn index(optional_user: auth::middleware::OptionalUser) -> Result<impl IntoResponse, AppError> {
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
    let alias_to_description: Vec<(String, Vec<String>, bool)> = state
        .alias_to_bookmark_map
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
    let bookmark_alias = splitted.next().unwrap_or("");
    let query = splitted.next().unwrap_or_default();

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
    let command = user_bookmarks
        .as_ref()
        .and_then(|user_map| user_map.get(bookmark_alias))
        .or_else(|| {
            // Check if global bookmark is disabled
            if disabled_globals.contains(bookmark_alias) {
                None
            } else {
                state.alias_to_bookmark_map.get(bookmark_alias)
            }
        });

    let redirect_url = match command {
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
                        state.alias_to_bookmark_map.get(default_alias)
                    }
                });

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
        alias_to_bookmark_map,
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

        // Admin routes (require admin authentication)
        .route("/admin", get(handlers::admin::admin_page))
        .route("/admin/cleanup-sessions", post(handlers::admin::cleanup_sessions))
        .route("/admin/create-user", post(handlers::admin::create_user))

        // Serve static files (JavaScript, CSS, etc.)
        .nest_service("/static", ServeDir::new("static"))

        .with_state(state)
        // Apply security headers to all responses
        .layer(middleware::from_fn(security::security_headers))
}
