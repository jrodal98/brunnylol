extern crate clap;

mod domain;
mod config;
mod error;
mod db;
mod auth;
mod handlers;

use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    routing::{delete, get, post, put},
    Router,
};
use domain::Command;
use config::commands::AliasAndCommand;
use error::AppError;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Arc};
use clap::Arg;

const DEFAULT_ALIAS: &str = "g";

// Template structs
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

#[derive(Template)]
#[template(path = "help.html")]
struct HelpTemplate {
    alias_to_description: Vec<(String, Vec<String>)>,
}

// Query parameter struct for search
#[derive(Deserialize)]
struct SearchParams {
    q: String,
    default: Option<String>,
}

// Application state
pub struct AppState {
    pub alias_to_bookmark_map: HashMap<String, Box<dyn Command>>,
    pub default_alias: String,
    pub db_pool: SqlitePool,
}

// Route handlers
async fn index() -> Result<impl IntoResponse, AppError> {
    let template = IndexTemplate;
    Ok(Html(template.render()?))
}

async fn help(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    // Pre-process descriptions: split by "|" for nested command descriptions
    let alias_to_description: Vec<(String, Vec<String>)> = state
        .alias_to_bookmark_map
        .iter()
        .map(|(alias, cmd)| {
            let description = cmd.description();
            let parts: Vec<String> = description
                .split('|')
                .map(|s| s.to_string())
                .collect();
            (alias.clone(), parts)
        })
        .collect();

    let template = HelpTemplate {
        alias_to_description,
    };
    Ok(Html(template.render()?))
}

async fn redirect(
    optional_user: auth::middleware::OptionalUser,
    Query(params): Query<SearchParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
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
            let default_alias = params
                .default
                .as_deref()
                .unwrap_or(&state.default_alias);

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
                    // Fallback to Google if default alias not found
                    format!("https://www.google.com/search?q={}", urlencoding::encode(&params.q))
                })
        }
    };

    Redirect::to(&redirect_url)
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
                .help("Path to SQLite database file")
                .default_value("brunnylol.db"),
        )
        .get_matches();

    let yaml_path = matches.get_one("commands").map(|c: &String| c.as_str());
    let default_alias = matches
        .get_one("default_alias")
        .map(|c: &String| c.as_str())
        .unwrap_or(DEFAULT_ALIAS)
        .to_string();

    let db_path = matches
        .get_one::<String>("database")
        .map(|s| s.as_str())
        .unwrap_or("brunnylol.db");

    // Initialize database
    let db_pool = db::init_db(db_path)
        .await
        .expect("Failed to initialize database");

    // Seed test user in development
    if cfg!(debug_assertions) {
        let _ = db::seed::seed_test_user(&db_pool).await;
    }

    let alias_to_bookmark_map = AliasAndCommand::get_alias_to_bookmark_map(yaml_path);

    let state = Arc::new(AppState {
        alias_to_bookmark_map,
        default_alias,
        db_pool: db_pool.clone(),
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

        // Admin routes (require admin authentication)
        .route("/admin", get(handlers::admin::admin_page))
        .route("/admin/cleanup-sessions", post(handlers::admin::cleanup_sessions))

        .with_state(state)
}
