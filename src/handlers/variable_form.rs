// Variable form handlers for /f/:alias routes

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    auth::middleware::OptionalUser,
    domain::{template::form_builder, Command},
    error::AppError,
};

#[derive(Template)]
#[template(path = "variable_form.html")]
struct VariableFormTemplate {
    alias: String,
    description: String,
    variables: Vec<FormVariableDisplay>,
    has_user: bool,
    is_admin: bool,
}

#[derive(Clone)]
struct FormVariableDisplay {
    name: String,
    is_required: bool,
    default_value: String,
    current_value: String,
    has_options: bool,
    options: Vec<String>,
    has_default: bool,
}

// GET /f/:alias - Show variable form or submit and redirect
pub async fn show_variable_form(
    optional_user: OptionalUser,
    Path(alias): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Response, AppError> {
    // Load bookmark (try user bookmarks first, then global)
    let command = if let Some(ref user) = optional_user.0 {
        let user_bookmarks = crate::db::bookmarks::load_user_bookmarks(&state.db_pool, user.id)
            .await
            .ok();

        if let Some(ref map) = user_bookmarks {
            map.get(&alias).cloned()
        } else {
            let bookmark_map = state.alias_to_bookmark_map.read().await;
            bookmark_map.get(&alias).cloned()
        }
    } else {
        let bookmark_map = state.alias_to_bookmark_map.read().await;
        bookmark_map.get(&alias).cloned()
    };

    let command = command.ok_or_else(|| AppError::NotFound(format!("Unknown alias: '{}'", alias)))?;

    // Only Variable commands have forms
    match command {
        Command::Variable { template, metadata, description, .. } => {
            // ALWAYS show the form - even if variables are prefilled
            // The form will be rendered with current_value set from query params
            let form_data = form_builder::build_form_data(&template, metadata.as_ref(), &params);

            let variables = form_data
                .into_iter()
                .map(|v| {
                    let default_val = v.default_value.clone().unwrap_or_default();
                    let current_val = v.current_value.unwrap_or(default_val.clone());
                    let has_default = v.default_value.is_some();

                    FormVariableDisplay {
                        name: v.name,
                        is_required: v.is_required,
                        default_value: default_val,
                        current_value: current_val,
                        has_options: v.options.is_some(),
                        options: v.options.unwrap_or_default(),
                        has_default,
                    }
                })
                .collect();

            let (has_user, is_admin) = if let Some(ref user) = optional_user.0 {
                (true, user.is_admin)
            } else {
                (false, false)
            };

            let tmpl = VariableFormTemplate {
                alias,
                description,
                variables,
                has_user,
                is_admin,
            };

            Ok(Html(tmpl.render()?).into_response())
        }
        _ => Err(AppError::BadRequest(format!(
            "Bookmark '{}' does not support variable forms",
            alias
        ))),
    }
}

// POST /f/:alias - Submit variable form and redirect
pub async fn submit_variable_form(
    optional_user: OptionalUser,
    Path(alias): Path<String>,
    State(state): State<Arc<crate::AppState>>,
    body: String,
) -> Response {
    // Parse form-url-encoded data manually
    let mut form_data = HashMap::new();

    for pair in body.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key_decoded = urlencoding::decode(key).unwrap_or_default().to_string();
            let value_decoded = urlencoding::decode(value).unwrap_or_default().to_string();
            form_data.insert(key_decoded, value_decoded);
        }
    }

    // Load bookmark
    let command = if let Some(ref user) = optional_user.0 {
        let user_bookmarks = crate::db::bookmarks::load_user_bookmarks(&state.db_pool, user.id)
            .await
            .ok();

        if let Some(ref map) = user_bookmarks {
            map.get(&alias).cloned()
        } else {
            let bookmark_map = state.alias_to_bookmark_map.read().await;
            bookmark_map.get(&alias).cloned()
        }
    } else {
        let bookmark_map = state.alias_to_bookmark_map.read().await;
        bookmark_map.get(&alias).cloned()
    };

    match command {
        Some(Command::Variable { base_url, template, .. }) => {
            let resolver = crate::domain::template::TemplateResolver::new();
            let url = resolver.resolve(&template, &form_data).unwrap_or(base_url);
            // Return URL as plain text for JavaScript to navigate
            axum::http::Response::builder()
                .header("Content-Type", "text/plain")
                .body(axum::body::Body::from(url))
                .unwrap()
                .into_response()
        }
        _ => crate::error::AppError::NotFound(format!("Unknown alias: '{}'", alias)).into_response(),
    }
}

