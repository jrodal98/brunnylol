// Variable form handlers for /f/:alias routes

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Response},
};
use std::{collections::HashMap, sync::Arc};

use crate::{
    auth::middleware::OptionalUser,
    domain::{template::form_builder, Command},
    error::AppError,
    helpers,
};

/// Resolve hierarchical path to final command
async fn resolve_nested_path(
    path: &str,
    user: Option<&crate::db::User>,
    state: &Arc<crate::AppState>,
) -> Result<(Command, Vec<String>), AppError> {
    let path_segments: Vec<&str> = path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();

    if path_segments.is_empty() {
        return Err(AppError::NotFound("Empty path".to_string()));
    }

    // Load root bookmark
    let root_alias = path_segments[0];
    let mut command = helpers::load_bookmark_for_alias(root_alias, user, state).await
        .ok_or_else(|| AppError::NotFound(format!("Unknown alias: '{}'", root_alias)))?;

    // Navigate through nested path
    for segment in &path_segments[1..] {
        match command {
            Command::Nested { children, .. } => {
                command = children.get(*segment)
                    .ok_or_else(|| AppError::NotFound(format!("Unknown child: '{}'", segment)))?
                    .clone();
            }
            _ => return Err(AppError::BadRequest(format!(
                "Cannot navigate into non-nested command at '{}'", segment
            ))),
        }
    }

    Ok((command, path_segments.iter().map(|s| s.to_string()).collect()))
}


#[derive(Template)]
#[template(path = "variable_form.html")]
struct VariableFormTemplate {
    alias: String,
    description: String,
    variables: Vec<FormVariableDisplay>,
    has_user: bool,
    is_admin: bool,
}

#[derive(Template)]
#[template(path = "nested_selection.html")]
struct NestedSelectionTemplate {
    parent_path: String,
    description: String,
    children: Vec<NestedChildDisplay>,
    has_user: bool,
    is_admin: bool,
}

#[derive(Clone)]
struct NestedChildDisplay {
    alias: String,
    description: String,
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
    Path(path): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Response, AppError> {
    // Resolve hierarchical path to final command
    let (command, path_segments) = resolve_nested_path(&path, optional_user.0.as_ref(), &state).await?;

    let (has_user, is_admin) = if let Some(ref user) = optional_user.0 {
        (true, user.is_admin)
    } else {
        (false, false)
    };

    // Handle different command types
    match command {
        Command::Variable { template, metadata, description, base_url } => {
            // Show variable form
            let form_data = form_builder::build_form_data(&template, metadata.as_ref(), &params);

            // If there are no variables to fill out, auto-redirect to base URL
            if form_data.is_empty() {
                use axum::response::Redirect;
                return Ok(Redirect::to(&base_url).into_response());
            }

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

            let tmpl = VariableFormTemplate {
                alias: path_segments.join("/"),
                description,
                variables,
                has_user,
                is_admin,
            };

            Ok(Html(tmpl.render()?).into_response())
        }
        Command::Nested { children, description } => {
            // Show nested command selection UI
            let mut child_list: Vec<NestedChildDisplay> = children
                .into_iter()
                .map(|(alias, cmd)| NestedChildDisplay {
                    alias,
                    description: cmd.description().to_string(),
                })
                .collect();
            child_list.sort_by(|a, b| a.alias.cmp(&b.alias));

            let tmpl = NestedSelectionTemplate {
                parent_path: path_segments.join("/"),
                description,
                children: child_list,
                has_user,
                is_admin,
            };

            Ok(Html(tmpl.render()?).into_response())
        }
    }
}

// POST /f/:alias - Submit variable form and redirect
pub async fn submit_variable_form(
    optional_user: OptionalUser,
    Path(path): Path<String>,
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

    // Resolve hierarchical path to final command
    let result = resolve_nested_path(&path, optional_user.0.as_ref(), &state).await;

    match result {
        Ok((Command::Variable { base_url, template, .. }, _)) => {
            let resolver = crate::domain::template::TemplateResolver::new();
            let url = resolver.resolve(&template, &form_data).unwrap_or(base_url);
            // Return URL as plain text for JavaScript to navigate
            axum::http::Response::builder()
                .header("Content-Type", "text/plain")
                .body(axum::body::Body::from(url))
                .unwrap()
                .into_response()
        }
        _ => crate::error::AppError::NotFound(format!("Unknown alias: '{}'", path)).into_response(),
    }
}

