// Bookmark management handlers

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{Html, IntoResponse, Response},
    http::{header, StatusCode},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth::middleware::CurrentUser, db, error::{AppError, DbResultExt}, validation};

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
    description: String,
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
    let user_bookmarks = db::get_bookmarks(&state.db_pool, db::BookmarkScope::Personal { user_id: current_user.0.id })
        .await
        .db_err()?;

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
        .db_err()?;

    let mut disabled_aliases = std::collections::HashSet::new();
    for (builtin_alias, is_disabled, _, _) in overrides {
        if is_disabled {
            disabled_aliases.insert(builtin_alias);
        }
    }

    let mut global_bookmarks = Vec::new();
    let mut conflicts = Vec::new();

    for (alias, command) in state.alias_to_bookmark_map.iter() {
        let is_overridden = user_aliases.contains(alias);
        let is_disabled = disabled_aliases.contains(alias);

        // Only show conflict if overridden AND not disabled
        if is_overridden && !is_disabled {
            conflicts.push(alias.clone());
        }

        global_bookmarks.push(GlobalBookmarkDisplay {
            alias: alias.clone(),
            description: command.description().to_string(),
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
    // Validate alias
    if form.alias.is_empty() || form.alias.len() > 50 {
        return Err(AppError::BadRequest("Invalid alias length".to_string()));
    }

    // Validate templated bookmarks have a valid template with {}
    if form.bookmark_type == "templated" {
        let template = form.command_template.as_deref().unwrap_or(&form.url);
        validation::validate_template(template)?;
    }

    let encode_query = form.encode_query.is_some();

    // Create parent bookmark in database
    let bookmark_id = db::create_bookmark(
        &state.db_pool,
        db::BookmarkScope::Personal { user_id: current_user.0.id },
        &form.alias,
        &form.bookmark_type,
        &form.url,
        &form.description,
        form.command_template.as_deref(),
        encode_query,
        Some(current_user.0.id),
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint") {
            AppError::BadRequest("Alias already exists".to_string())
        } else {
            AppError::Internal(format!("Failed to create bookmark: {}", e))
        }
    })?;

    // If nested bookmark, create sub-commands from JSON
    if form.bookmark_type == "nested" {
        if let Some(json_str) = &form.nested_commands_json {
            let nested_commands: Vec<NestedCommandData> = serde_json::from_str(json_str)
                .map_err(|e| AppError::Internal(format!("Failed to parse nested commands: {}", e)))?;

            for (i, nested_cmd) in nested_commands.iter().enumerate() {
                let nested_template = if nested_cmd.cmd_type == "templated" {
                    nested_cmd.template.as_deref()
                } else {
                    None
                };

                // Validate nested templated bookmarks have a valid template with {}
                if nested_cmd.cmd_type == "templated" {
                    let template = nested_template.unwrap_or(&nested_cmd.url);
                    validation::validate_template(template)
                        .map_err(|_| AppError::BadRequest(
                            format!("Nested bookmark '{}' is templated but template doesn't contain {{}} placeholder", nested_cmd.alias)
                        ))?;
                }

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
                .map_err(|e| AppError::Internal(format!("Failed to create nested bookmark: {}", e)))?;
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
    db::delete_bookmark(&state.db_pool, bookmark_id, db::BookmarkScope::Personal { user_id: current_user.0.id })
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
        validation::validate_template(template)?;
    }

    db::update_bookmark(
        &state.db_pool,
        bookmark_id,
        db::BookmarkScope::Personal { user_id: current_user.0.id },
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
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<CreateNestedForm>,
) -> Result<impl IntoResponse, AppError> {
    // Verify parent bookmark belongs to current user
    let parent = db::get_bookmark_by_id(&state.db_pool, form.parent_id)
        .await
        .db_err()?
        .ok_or(AppError::NotFound("Parent bookmark not found".to_string()))?;

    if parent.user_id != Some(current_user.0.id) {
        return Err(AppError::Forbidden("Cannot modify bookmarks you don't own".to_string()));
    }

    let encode_query = form.encode_query.is_some();

    // Validate templated nested bookmarks have a valid template with {}
    if let Some(ref template) = form.command_template {
        validation::validate_template(template)?;
    }

    // Get the next display order
    let existing_nested = db::get_nested_bookmarks(&state.db_pool, form.parent_id)
        .await
        .db_err()?;

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
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Path(nested_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    // Verify nested bookmark's parent belongs to current user
    let nested = db::get_nested_bookmark_by_id(&state.db_pool, nested_id)
        .await
        .db_err()?
        .ok_or(AppError::NotFound("Nested bookmark not found".to_string()))?;

    let parent = db::get_bookmark_by_id(&state.db_pool, nested.parent_bookmark_id)
        .await
        .db_err()?
        .ok_or(AppError::Internal("Parent bookmark not found".to_string()))?;

    if parent.user_id != Some(current_user.0.id) {
        return Err(AppError::Forbidden("Cannot modify bookmarks you don't own".to_string()));
    }

    db::delete_nested_bookmark(&state.db_pool, nested_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete nested bookmark: {}", e)))?;

    Ok(Html(""))
}

// GET /manage/bookmark/:id/nested/list - List nested bookmarks
pub async fn list_nested_bookmarks(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Path(bookmark_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    // Verify bookmark belongs to current user
    let parent = db::get_bookmark_by_id(&state.db_pool, bookmark_id)
        .await
        .db_err()?
        .ok_or(AppError::NotFound("Bookmark not found".to_string()))?;

    if parent.user_id != Some(current_user.0.id) {
        return Err(AppError::Forbidden("Cannot access bookmarks you don't own".to_string()));
    }

    let nested = db::get_nested_bookmarks(&state.db_pool, bookmark_id)
        .await
        .db_err()?;

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

    // Get description from command map
    let description = state.alias_to_bookmark_map
        .get(&form.builtin_alias)
        .map(|cmd| cmd.description())
        .unwrap_or("Built-in bookmark");

    // Return updated table row
    let status_html = if is_disabled {
        "<span style=\"color: #d32f2f;\">✗ Disabled</span>"
    } else {
        "<span style=\"color: #4caf50;\">✓ Active</span>"
    };

    let button_html = if is_disabled {
        format!(
            "<form hx-post=\"/manage/override\" hx-target=\"#global-{}\" hx-swap=\"outerHTML\" style=\"margin: 0;\">
                <input type=\"hidden\" name=\"builtin_alias\" value=\"{}\">
                <button type=\"submit\" class=\"btn-primary\">Enable</button>
            </form>",
            form.builtin_alias, form.builtin_alias
        )
    } else {
        format!(
            "<form hx-post=\"/manage/override\" hx-target=\"#global-{}\" hx-swap=\"outerHTML\" style=\"margin: 0;\">
                <input type=\"hidden\" name=\"builtin_alias\" value=\"{}\">
                <input type=\"hidden\" name=\"is_disabled\" value=\"true\">
                <button type=\"submit\" class=\"btn-secondary\">Disable</button>
            </form>",
            form.builtin_alias, form.builtin_alias
        )
    };

    Ok(Html(format!(
        "<tr id=\"global-{}\">
            <td>
                <input type=\"checkbox\" class=\"global-checkbox\" value=\"{}\" onchange=\"updateGlobalSelection()\">
            </td>
            <td><strong>{}</strong></td>
            <td>{}</td>
            <td id=\"status-{}\">{}</td>
            <td>
                <div style=\"display: flex; gap: 0.5em; align-items: center;\">
                    {}
                    <form hx-post=\"/manage/fork-global\"
                          hx-target=\"#fork-result-{}\"
                          hx-swap=\"innerHTML\"
                          style=\"margin: 0;\">
                        <input type=\"hidden\" name=\"alias\" value=\"{}\">
                        <button type=\"submit\" class=\"btn-secondary\">Fork</button>
                    </form>
                </div>
                <div id=\"fork-result-{}\"></div>
            </td>
        </tr>",
        form.builtin_alias,
        form.builtin_alias,
        form.builtin_alias,
        description,
        form.builtin_alias,
        status_html,
        button_html,
        form.builtin_alias,
        form.builtin_alias,
        form.builtin_alias
    )))
}

// Form structs for import/export
#[derive(Deserialize)]
pub struct ImportForm {
    source: String,          // "paste", "file", or "url"
    content: Option<String>, // For paste
    url: Option<String>,     // For URL
    format: String,          // "yaml" or "json"
    scope: String,           // "personal" or "global"
}

#[derive(Deserialize)]
pub struct ExportParams {
    scope: String,  // "personal" or "global"
    format: String, // "yaml" or "json"
}

#[derive(Deserialize)]
pub struct BulkDeleteForm {
    ids: Vec<i64>,
}

#[derive(Deserialize)]
pub struct BulkDisableForm {
    aliases: Vec<String>,
    is_disabled: bool,
}

#[derive(Deserialize)]
pub struct ForkGlobalForm {
    alias: String,
}

// POST /manage/import - Import bookmarks
pub async fn import_bookmarks(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<ImportForm>,
) -> Result<impl IntoResponse, AppError> {
    use crate::services::bookmark_service::BookmarkScope;
    use crate::services::serializers::{YamlSerializer, JsonSerializer};

    // Determine scope - only admins can import global
    let scope = if form.scope == "global" {
        if !current_user.0.is_admin {
            return Err(AppError::Forbidden("Only admins can import global bookmarks".to_string()));
        }
        BookmarkScope::Global
    } else {
        BookmarkScope::Personal
    };

    // Get content based on source
    let content = match form.source.as_str() {
        "paste" => form.content.ok_or(AppError::BadRequest("No content provided".to_string()))?,
        "url" => {
            // Fetch from URL
            let url = form.url.ok_or(AppError::BadRequest("No URL provided".to_string()))?;

            // Use reqwest to fetch URL content
            let response = reqwest::get(&url)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to fetch URL: {}", e)))?;

            if !response.status().is_success() {
                return Err(AppError::Internal(format!("URL returned status {}", response.status())));
            }

            response.text()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to read URL content: {}", e)))?
        }
        _ => return Err(AppError::BadRequest("Invalid import source".to_string())),
    };

    // Select serializer
    let serializer: Box<dyn crate::services::serializers::BookmarkSerializer> = match form.format.as_str() {
        "json" => Box::new(JsonSerializer),
        _ => Box::new(YamlSerializer), // Default to YAML
    };

    // Import bookmarks
    let result = state.bookmark_service.import_bookmarks(
        &content,
        serializer.as_ref(),
        scope,
        Some(current_user.0.id),
    ).await
    .map_err(|e| AppError::Internal(format!("Import failed: {}", e)))?;

    let message = if result.errors.is_empty() {
        format!(
            "Successfully imported {} bookmarks ({} skipped as duplicates)",
            result.imported, result.skipped
        )
    } else {
        format!(
            "Imported {} bookmarks ({} skipped). Errors: {}",
            result.imported, result.skipped, result.errors.join(", ")
        )
    };

    Ok(Html(format!(
        r#"<div class="success-message">{} <a href="/manage">Refresh to see changes</a></div>"#,
        message
    )))
}

// GET /manage/export - Export bookmarks
pub async fn export_bookmarks(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    axum::extract::Query(params): axum::extract::Query<ExportParams>,
) -> Result<Response, AppError> {
    use crate::services::bookmark_service::BookmarkScope;
    use crate::services::serializers::{YamlSerializer, JsonSerializer};

    // Check permissions for global export
    if params.scope == "global" && !current_user.0.is_admin {
        return Err(AppError::Forbidden("Only admins can export global bookmarks".to_string()));
    }

    let scope = if params.scope == "global" {
        BookmarkScope::Global
    } else {
        BookmarkScope::Personal
    };

    let serializer: Box<dyn crate::services::serializers::BookmarkSerializer> = match params.format.as_str() {
        "json" => Box::new(JsonSerializer),
        _ => Box::new(YamlSerializer),
    };

    let content = state.bookmark_service.export_bookmarks(
        scope,
        Some(current_user.0.id),
        serializer.as_ref(),
    ).await
    .map_err(|e| AppError::Internal(format!("Export failed: {}", e)))?;

    let filename = format!(
        "bookmarks_{}.{}",
        if scope == BookmarkScope::Global { "global" } else { "personal" },
        serializer.file_extension()
    );

    let mut response = Response::new(content.into());
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        serializer.content_type().parse().unwrap(),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename).parse().unwrap(),
    );

    Ok(response)
}

// POST /manage/bookmarks/bulk-delete - Delete multiple personal bookmarks
pub async fn bulk_delete_bookmarks(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    axum::extract::Json(form): axum::extract::Json<BulkDeleteForm>,
) -> Result<impl IntoResponse, AppError> {
    let mut deleted_count = 0;
    let mut errors = Vec::new();

    for id in form.ids {
        match db::delete_bookmark(&state.db_pool, id, db::BookmarkScope::Personal { user_id: current_user.0.id }).await {
            Ok(_) => deleted_count += 1,
            Err(e) => errors.push(format!("ID {}: {}", id, e)),
        }
    }

    if !errors.is_empty() {
        return Err(AppError::Internal(format!("Failed to delete some bookmarks: {}", errors.join(", "))));
    }

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "success": true,
            "deleted": deleted_count
        }))
    ))
}

// POST /manage/overrides/bulk-disable - Bulk disable/enable global bookmarks
pub async fn bulk_toggle_global(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    axum::extract::Json(form): axum::extract::Json<BulkDisableForm>,
) -> Result<impl IntoResponse, AppError> {
    let mut updated_count = 0;
    let mut errors = Vec::new();

    for alias in form.aliases {
        match db::upsert_override(
            &state.db_pool,
            current_user.0.id,
            &alias,
            form.is_disabled,
            None,
            None,
        ).await {
            Ok(_) => updated_count += 1,
            Err(e) => errors.push(format!("Alias '{}': {}", alias, e)),
        }
    }

    if !errors.is_empty() {
        return Err(AppError::Internal(format!("Failed to update some bookmarks: {}", errors.join(", "))));
    }

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "success": true,
            "updated": updated_count
        }))
    ))
}

// POST /manage/fork-global - Fork a global bookmark to personal bookmarks
pub async fn fork_global_bookmark(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<ForkGlobalForm>,
) -> Result<impl IntoResponse, AppError> {
    // Get the global bookmark from database
    let global_bookmarks = db::get_bookmarks(&state.db_pool, db::BookmarkScope::Global)
        .await
        .db_err()?;

    let global_bookmark = global_bookmarks
        .iter()
        .find(|b| b.alias == form.alias)
        .ok_or(AppError::NotFound(format!("Global bookmark '{}' not found", form.alias)))?;

    // Create a copy in user bookmarks
    let bookmark_id = db::create_bookmark(
        &state.db_pool,
        db::BookmarkScope::Personal { user_id: current_user.0.id },
        &global_bookmark.alias,
        &global_bookmark.bookmark_type,
        &global_bookmark.url,
        &global_bookmark.description,
        global_bookmark.command_template.as_deref(),
        global_bookmark.encode_query,
        Some(current_user.0.id), // Track who forked this
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint") {
            AppError::BadRequest(format!("You already have a bookmark with alias '{}'", form.alias))
        } else {
            AppError::Internal(format!("Failed to fork bookmark: {}", e))
        }
    })?;

    // If it's a nested bookmark, copy the nested bookmarks too
    if global_bookmark.bookmark_type == "nested" {
        let global_nested = db::get_nested_bookmarks(&state.db_pool, global_bookmark.id)
            .await
            .db_err()?;

        for (i, nested) in global_nested.iter().enumerate() {
            db::create_nested_bookmark(
                &state.db_pool,
                bookmark_id,
                &nested.alias,
                &nested.url,
                &nested.description,
                nested.command_template.as_deref(),
                nested.encode_query,
                i as i32,
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fork nested bookmark: {}", e)))?;
        }
    }

    Ok(Html(format!(
        r#"<div class="success-message">Forked '{}' to your personal bookmarks! <a href="/manage">Refresh to see changes</a></div>"#,
        form.alias
    )))
}
