extern crate clap;

mod domain;
mod config;
mod error;

use askama::Template;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use domain::Command;
use config::commands::AliasAndCommand;
use error::AppError;
use serde::Deserialize;
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
    Query(params): Query<SearchParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut splitted = params.q.splitn(2, ' ');
    let bookmark_alias = splitted.next().unwrap_or("");
    let query = splitted.next().unwrap_or_default();

    let redirect_url = match state.alias_to_bookmark_map.get(bookmark_alias) {
        Some(bookmark) => bookmark.get_redirect_url(query),
        None => {
            let default_alias = params
                .default
                .as_deref()
                .unwrap_or(&state.default_alias);

            state
                .alias_to_bookmark_map
                .get(default_alias)
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
pub fn create_router() -> Router {
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
        .get_matches();

    let yaml_path = matches.get_one("commands").map(|c: &String| c.as_str());
    let default_alias = matches
        .get_one("default_alias")
        .map(|c: &String| c.as_str())
        .unwrap_or(DEFAULT_ALIAS)
        .to_string();

    let alias_to_bookmark_map = AliasAndCommand::get_alias_to_bookmark_map(yaml_path);

    let state = Arc::new(AppState {
        alias_to_bookmark_map,
        default_alias,
    });

    Router::new()
        .route("/", get(index))
        .route("/help", get(help))
        .route("/search", get(redirect))
        .with_state(state)
}
