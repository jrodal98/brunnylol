// Bookmark management handlers

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth::middleware::CurrentUser, db, error::AppError};

// Template structs
#[derive(Template)]
#[template(path = "manage.html")]
struct ManageTemplate {
    user: db::User,
    personal_bookmarks: Vec<BookmarkDisplay>,
    personal_count: usize,
    global_bookmarks: Vec<GlobalBookmarkDisplay>,
    conflicts_text: String,
    has_conflicts: bool,
}

#[derive(Clone)]
struct BookmarkDisplay {
    id: i64,
    alias: String,
    bookmark_type: String,
    url: String,
    description: String,
    command_template: String,
    encode_query: bool,
    nested_count: usize,
}

#[derive(Clone)]
struct GlobalBookmarkDisplay {
    alias: String,
    is_overridden: bool,
    is_disabled: bool,
}

// Form structs
#[derive(Deserialize, Debug)]
pub struct NestedCommandData {
    alias: String,
    #[serde(rename = "type")]
    cmd_type: String,
    url: String,
    description: String,
    template: Option<String>,
    encode: bool,
}

#[derive(Deserialize)]
pub struct CreateBookmarkForm {
    alias: String,
    bookmark_type: String,  // "simple", "templated", or "nested"
    url: String,
    description: String,
    command_template: Option<String>,
    encode_query: Option<String>, // checkbox value
    // Nested commands as JSON string
    nested_commands_json: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateBookmarkForm {
    alias: String,
    url: String,
    description: String,
    command_template: Option<String>,
    encode_query: Option<String>,
}

#[derive(Deserialize)]
pub struct DisableGlobalForm {
    builtin_alias: String,
    is_disabled: Option<String>, // checkbox
}

#[derive(Deserialize)]
pub struct CreateNestedForm {
    parent_id: i64,
    alias: String,
    url: String,
    description: String,
    command_template: Option<String>,
    encode_query: Option<String>,
}

// GET /manage - Main bookmark management page
pub async fn manage_page(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Html<String>, AppError> {
    // Get user's personal bookmarks
    let user_bookmarks = db::get_user_bookmarks(&state.db_pool, current_user.0.id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    // Convert to display format
    let mut personal_bookmarks = Vec::new();
    for bookmark in user_bookmarks {
        // Count nested bookmarks if applicable
        let nested_count = if bookmark.bookmark_type == "nested" {
            db::get_nested_bookmarks(&state.db_pool, bookmark.id)
                .await
                .map(|n| n.len())
                .unwrap_or(0)
        } else {
            0
        };

        personal_bookmarks.push(BookmarkDisplay {
            id: bookmark.id,
            alias: bookmark.alias.clone(),
            bookmark_type: bookmark.bookmark_type.clone(),
            url: bookmark.url.clone(),
            description: bookmark.description.clone(),
            command_template: bookmark.command_template.clone().unwrap_or_default(),
            encode_query: bookmark.encode_query,
            nested_count,
        });
    }

    // Get global alias list and user overrides for conflict/disable detection
    let user_aliases: std::collections::HashSet<String> = personal_bookmarks
        .iter()
        .map(|b| b.alias.clone())
        .collect();

    // Get user's disabled bookmarks
    let overrides = db::get_user_overrides(&state.db_pool, current_user.0.id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let mut disabled_aliases = std::collections::HashSet::new();
    for (builtin_alias, is_disabled, _, _) in overrides {
        if is_disabled {
            disabled_aliases.insert(builtin_alias);
        }
    }

    let mut global_bookmarks = Vec::new();
    let mut conflicts = Vec::new();

    for alias in state.alias_to_bookmark_map.keys() {
        let is_overridden = user_aliases.contains(alias);
        let is_disabled = disabled_aliases.contains(alias);

        if is_overridden {
            conflicts.push(alias.clone());
        }

        global_bookmarks.push(GlobalBookmarkDisplay {
            alias: alias.clone(),
            is_overridden,
            is_disabled,
        });
    }

    global_bookmarks.sort_by(|a, b| a.alias.cmp(&b.alias));

    let personal_count = personal_bookmarks.len();
    let has_conflicts = !conflicts.is_empty();
    let conflicts_text = conflicts.join(", ");

    let template = ManageTemplate {
        user: current_user.0,
        personal_bookmarks,
        personal_count,
        global_bookmarks,
        conflicts_text,
        has_conflicts,
    };

    Ok(Html(template.render()?))
}

// POST /manage/bookmark - Create new bookmark
pub async fn create_bookmark(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<CreateBookmarkForm>,
) -> Result<impl IntoResponse, AppError> {
    // Debug logging
    eprintln!("Creating bookmark: alias={}, type={}", form.alias, form.bookmark_type);
    eprintln!("Nested JSON: {:?}", form.nested_commands_json);

    // Validate alias
    if form.alias.is_empty() || form.alias.len() > 50 {
        return Err(AppError::BadRequest("Invalid alias length".to_string()));
    }

    // Validate templated bookmarks have a valid template with {}
    if form.bookmark_type == "templated" {
        let template = form.command_template.as_deref().unwrap_or(&form.url);
        if !template.contains("{}") {
            return Err(AppError::BadRequest(
                "Templated bookmarks must have a template containing {} placeholder".to_string()
            ));
        }
    }

    let encode_query = form.encode_query.is_some();

    // Create parent bookmark in database
    let bookmark_id = db::create_bookmark(
        &state.db_pool,
        current_user.0.id,
        &form.alias,
        &form.bookmark_type,
        &form.url,
        &form.description,
        form.command_template.as_deref(),
        encode_query,
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint") {
            AppError::BadRequest("Alias already exists".to_string())
        } else {
            AppError::Internal(format!("Failed to create bookmark: {}", e))
        }
    })?;

    eprintln!("Created parent bookmark with ID: {}", bookmark_id);

    // If nested bookmark, create sub-commands from JSON
    if form.bookmark_type == "nested" {
        if let Some(json_str) = &form.nested_commands_json {
            let nested_commands: Vec<NestedCommandData> = serde_json::from_str(json_str)
                .map_err(|e| AppError::Internal(format!("Failed to parse nested commands: {}", e)))?;

            eprintln!("Parsed {} nested commands from JSON", nested_commands.len());

            for (i, nested_cmd) in nested_commands.iter().enumerate() {
                let nested_template = if nested_cmd.cmd_type == "templated" {
                    nested_cmd.template.as_deref()
                } else {
                    None
                };

                // Validate nested templated bookmarks have a valid template with {}
                if nested_cmd.cmd_type == "templated" {
                    let template = nested_template.unwrap_or(&nested_cmd.url);
                    if !template.contains("{}") {
                        return Err(AppError::BadRequest(
                            format!("Nested bookmark '{}' is templated but template doesn't contain {{}} placeholder", nested_cmd.alias)
                        ));
                    }
                }

                eprintln!("  Creating nested #{}: alias={}, type={}, url={}",
                         i, nested_cmd.alias, nested_cmd.cmd_type, nested_cmd.url);

                db::create_nested_bookmark(
                    &state.db_pool,
                    bookmark_id,
                    &nested_cmd.alias,
                    &nested_cmd.url,
                    &nested_cmd.description,
                    nested_template,
                    nested_cmd.encode,
                    i as i32,
                )
                .await
                .map_err(|e| {
                    eprintln!("Failed to create nested bookmark: {}", e);
                    AppError::Internal(format!("Failed to create nested bookmark: {}", e))
                })?;

                eprintln!("  Nested #{} created successfully", i);
            }
        }
    }

    // Return success message as HTMX fragment
    Ok(Html(format!(
        r#"<div class="success-message">Bookmark '{}' created successfully! <a href="/manage">Refresh to see changes</a></div>"#,
        form.alias
    )))
}

// DELETE /manage/bookmark/:id - Delete a bookmark
pub async fn delete_bookmark(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Path(bookmark_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    db::delete_bookmark(&state.db_pool, bookmark_id, current_user.0.id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete bookmark: {}", e)))?;

    // Return empty HTML (HTMX will remove the row)
    Ok(Html(""))
}

// PUT /manage/bookmark/:id - Update a bookmark
pub async fn update_bookmark(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Path(bookmark_id): Path<i64>,
    Form(form): Form<UpdateBookmarkForm>,
) -> Result<impl IntoResponse, AppError> {
    let encode_query = form.encode_query.is_some();

    // Validate command template if provided
    if let Some(ref template) = form.command_template {
        if !template.is_empty() && !template.contains("{}") {
            return Err(AppError::BadRequest(
                "Template must contain {} placeholder".to_string()
            ));
        }
    }

    db::update_bookmark(
        &state.db_pool,
        bookmark_id,
        current_user.0.id,
        &form.alias,
        &form.url,
        &form.description,
        form.command_template.as_deref(),
        encode_query,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update bookmark: {}", e)))?;

    Ok(Html(format!(
        r#"<div class="success-message">Bookmark '{}' updated successfully!</div>"#,
        form.alias
    )))
}

// POST /manage/bookmark/:id/nested - Add nested bookmark
pub async fn create_nested_bookmark(
    _current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<CreateNestedForm>,
) -> Result<impl IntoResponse, AppError> {
    let encode_query = form.encode_query.is_some();

    // Validate templated nested bookmarks have a valid template with {}
    if let Some(ref template) = form.command_template {
        if !template.is_empty() && !template.contains("{}") {
            return Err(AppError::BadRequest(
                "Templated bookmarks must have a template containing {} placeholder".to_string()
            ));
        }
    }

    // Get the next display order
    let existing_nested = db::get_nested_bookmarks(&state.db_pool, form.parent_id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let display_order = existing_nested.len() as i32;

    db::create_nested_bookmark(
        &state.db_pool,
        form.parent_id,
        &form.alias,
        &form.url,
        &form.description,
        form.command_template.as_deref(),
        encode_query,
        display_order,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create nested bookmark: {}", e)))?;

    Ok(Html(format!(
        r#"<div class="success-message">Sub-command '{}' added successfully!</div>"#,
        form.alias
    )))
}

// DELETE /manage/nested/:id - Delete nested bookmark
pub async fn delete_nested_bookmark(
    _current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Path(nested_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    db::delete_nested_bookmark(&state.db_pool, nested_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete nested bookmark: {}", e)))?;

    Ok(Html(""))
}

// GET /manage/bookmark/:id/nested/list - List nested bookmarks
pub async fn list_nested_bookmarks(
    _current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Path(bookmark_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let nested = db::get_nested_bookmarks(&state.db_pool, bookmark_id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let mut html = String::from("<div class='nested-commands'>");

    if nested.is_empty() {
        html.push_str("<p><em>No sub-commands yet. Add one below.</em></p>");
    } else {
        html.push_str("<table style='width: 100%; border-collapse: collapse;'>");
        html.push_str("<thead><tr><th>Alias</th><th>URL/Template</th><th>Description</th><th>Actions</th></tr></thead>");
        html.push_str("<tbody>");

        for n in nested {
            let url_display = if let Some(ref template) = n.command_template {
                template.clone()
            } else {
                n.url.clone()
            };

            html.push_str(&format!(
                "<tr id=\"nested-{}\">
                    <td><strong>{}</strong></td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>
                        <button class=\"btn-danger\"
                                hx-delete=\"/manage/nested/{}\"
                                hx-target=\"#nested-{}\"
                                hx-swap=\"outerHTML\"
                                hx-confirm=\"Delete sub-command '{}'?\">
                            Delete
                        </button>
                    </td>
                </tr>",
                n.id, n.alias, url_display, n.description, n.id, n.id, n.alias
            ));
        }

        html.push_str("</tbody></table>");
    }

    html.push_str("</div>");
    Ok(Html(html))
}

// POST /manage/override - Disable/enable global bookmark
pub async fn toggle_global_bookmark(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<DisableGlobalForm>,
) -> Result<impl IntoResponse, AppError> {
    let is_disabled = form.is_disabled.is_some();

    db::upsert_override(
        &state.db_pool,
        current_user.0.id,
        &form.builtin_alias,
        is_disabled,
        None, // custom_alias
        None, // additional_aliases
    )
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update override: {}", e)))?;

    // Return updated table row
    let status_html = if is_disabled {
        "<span style=\"color: #d32f2f;\">✗ Disabled</span>"
    } else {
        "<span style=\"color: #4caf50;\">✓ Active</span>"
    };

    let button_html = if is_disabled {
        format!(
            "<form hx-post=\"/manage/override\" hx-target=\"#global-{}\" hx-swap=\"outerHTML\" style=\"display: inline;\">
                <input type=\"hidden\" name=\"builtin_alias\" value=\"{}\">
                <button type=\"submit\" class=\"btn-primary\">Enable</button>
            </form>",
            form.builtin_alias, form.builtin_alias
        )
    } else {
        format!(
            "<form hx-post=\"/manage/override\" hx-target=\"#global-{}\" hx-swap=\"outerHTML\" style=\"display: inline;\">
                <input type=\"hidden\" name=\"builtin_alias\" value=\"{}\">
                <input type=\"hidden\" name=\"is_disabled\" value=\"true\">
                <button type=\"submit\" class=\"btn-secondary\">Disable</button>
            </form>",
            form.builtin_alias, form.builtin_alias
        )
    };

    Ok(Html(format!(
        "<tr id=\"global-{}\">
            <td><strong>{}</strong></td>
            <td>Built-in bookmark</td>
            <td id=\"status-{}\">{}</td>
            <td>{}</td>
        </tr>",
        form.builtin_alias, form.builtin_alias, form.builtin_alias, status_html, button_html
    )))
}
