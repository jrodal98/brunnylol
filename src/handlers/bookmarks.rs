// Bookmark management handlers

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{Html, IntoResponse, Response},
    http::header,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth::middleware::CurrentUser, db, domain::Command, error::{AppError, DbResultExt}, validation, helpers};
use super::common::{SuccessTemplate, SuccessWithLinkTemplate};

// Helper function to convert Template AST back to string for editing
fn render_template_to_string(template: &crate::domain::template::Template) -> String {
    use crate::domain::template::{TemplatePart, PipelineOp};

    let mut result = String::new();

    for part in &template.parts {
        match part {
            TemplatePart::Literal(s) => {
                // Escape braces in literals
                result.push_str(&s.replace("{", "{{").replace("}", "}}"));
            }
            TemplatePart::Variable(var) => {
                result.push('{');
                result.push_str(&var.name);

                if var.is_optional {
                    result.push('?');
                } else if let Some(ref default) = var.default {
                    result.push('=');
                    result.push_str(default);
                }

                // Add pipelines
                for pipeline in &var.pipelines {
                    result.push('|');
                    match pipeline {
                        PipelineOp::Encode => result.push_str("encode"),
                        PipelineOp::NoEncode => result.push_str("!encode"),
                        PipelineOp::Trim => result.push_str("trim"),
                        PipelineOp::Options { values, strict } => {
                            result.push_str("options[");
                            result.push_str(&values.join(","));
                            result.push(']');
                            if *strict {
                                result.push_str("[strict]");
                            }
                        }
                        PipelineOp::Map { mappings } => {
                            result.push_str("map[");
                            let mapping_strs: Vec<String> = mappings
                                .iter()
                                .map(|(k, v)| format!("{}:{}", k, v))
                                .collect();
                            result.push_str(&mapping_strs.join(","));
                            result.push(']');
                        }
                    }
                }

                result.push('}');
            }
        }
    }

    result
}

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
    nested_count: usize,
}

#[derive(Clone)]
struct GlobalBookmarkDisplay {
    alias: String,
    description: String,
    url: String,
    template: String,
    is_overridden: bool,
    is_disabled: bool,
}

#[derive(Template)]
#[template(path = "partials/nested_list.html")]
struct NestedListTemplate {
    nested: Vec<db::NestedBookmark>,
}

#[derive(Template)]
#[template(path = "partials/global_bookmark_row.html")]
struct GlobalBookmarkRowTemplate<'a> {
    alias: &'a str,
    description: &'a str,
    url: &'a str,
    template: &'a str,
    is_disabled: bool,
    is_admin: bool,
}

// Form structs
#[derive(Deserialize, Debug)]
pub struct NestedCommandData {
    alias: String,
    url: String,
    description: String,
    command_template: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateBookmarkForm {
    alias: String,
    bookmark_type: String,  // "simple", "templated", or "nested"
    url: String,
    description: String,
    command_template: Option<String>,
    // Nested commands as JSON string
    nested_commands_json: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateBookmarkForm {
    alias: String,
    url: String,
    description: String,
    command_template: Option<String>,
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
}

// GET /manage - Main bookmark management page
pub async fn manage_page(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Html<String>, AppError> {
    // Get user's personal bookmarks with nested counts (optimized to avoid N+1)
    let bookmarks_with_nested = db::get_bookmarks_with_nested(
        &state.db_pool,
        db::BookmarkScope::Personal { user_id: current_user.0.id }
    ).await.db_err()?;

    // Convert to display format
    let personal_bookmarks: Vec<BookmarkDisplay> = bookmarks_with_nested
        .into_iter()
        .map(|(bookmark, nested)| {
            BookmarkDisplay {
                id: bookmark.id,
                alias: bookmark.alias,
                bookmark_type: bookmark.bookmark_type,
                url: bookmark.url,
                description: bookmark.description,
                command_template: bookmark.command_template.unwrap_or_default(),
                nested_count: nested.len(),
            }
        })
        .collect();

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

    let bookmark_map = state.alias_to_bookmark_map.read().await;
    for (alias, command) in bookmark_map.iter() {
        let is_overridden = user_aliases.contains(alias.as_str());
        let is_disabled = disabled_aliases.contains(alias.as_str());

        // Only show conflict if overridden AND not disabled
        if is_overridden && !is_disabled {
            conflicts.push(alias.clone());
        }

        // Extract URL and template from Command
        let (url, template) = match command {
            Command::Variable { base_url, template, .. } => {
                // Render template back to string for editing
                let template_str = render_template_to_string(template);
                (base_url.clone(), template_str)
            }
            Command::Nested { .. } => {
                // Nested bookmarks don't have a simple template
                (String::new(), String::new())
            }
        };

        global_bookmarks.push(GlobalBookmarkDisplay {
            alias: alias.clone(),
            description: command.description().to_string(),
            url,
            template,
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

    // Validate URL scheme (skip for nested bookmarks since parent URL is not used)
    if form.bookmark_type != "nested" {
        validation::validate_url_scheme(&form.url)?;
    }

    // Validate templated bookmarks have a valid template (if provided)
    if form.bookmark_type == "templated" {
        helpers::validate_optional_template(&form.command_template)?;
    }

    // Create parent bookmark in database
    let bookmark_id = db::create_bookmark(
        &state.db_pool,
        db::BookmarkScope::Personal { user_id: current_user.0.id },
        &form.alias,
        &form.bookmark_type,
        &form.url,
        &form.description,
        form.command_template.as_deref(),
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

            // Prevent DoS: Limit number of nested bookmarks per parent
            if nested_commands.len() > 100 {
                return Err(AppError::BadRequest("Too many nested commands (maximum 100 allowed)".to_string()));
            }

            for (i, nested_cmd) in nested_commands.iter().enumerate() {
                validation::validate_url_scheme(&nested_cmd.url)?;
                helpers::validate_optional_template(&nested_cmd.command_template)?;

                db::create_nested_bookmark(
                    &state.db_pool,
                    bookmark_id,
                    &nested_cmd.alias,
                    &nested_cmd.url,
                    &nested_cmd.description,
                    nested_cmd.command_template.as_deref(),
                    i as i32,
                )
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create nested bookmark '{}': {}", nested_cmd.alias, e)))?;
            }
        }
    }

    // Return success message as HTMX fragment
    let message = format!("Bookmark '{}' created successfully!", form.alias);
    let template = SuccessWithLinkTemplate {
        message: &message,
        link: "/manage",
        link_text: "Refresh to see changes",
    };
    Ok(Html(template.render()?))
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
    // Validate URL scheme
    validation::validate_url_scheme(&form.url)?;

    helpers::validate_optional_template(&form.command_template)?;

    db::update_bookmark(
        &state.db_pool,
        bookmark_id,
        db::BookmarkScope::Personal { user_id: current_user.0.id },
        &form.alias,
        &form.url,
        &form.description,
        form.command_template.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update bookmark: {}", e)))?;

    let message = format!("Bookmark '{}' updated successfully!", form.alias);
    let template = SuccessTemplate { message: &message };
    Ok(Html(template.render()?))
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

    // Validate URL scheme
    validation::validate_url_scheme(&form.url)?;

    helpers::validate_optional_template(&form.command_template)?;

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
        display_order,
    )
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create nested bookmark: {}", e)))?;

    let message = format!("Sub-command '{}' added successfully!", form.alias);
    let template = SuccessTemplate { message: &message };
    Ok(Html(template.render()?))
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

    let template = NestedListTemplate { nested };
    Ok(Html(template.render()?))
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
    let bookmark_map = state.alias_to_bookmark_map.read().await;
    let bookmark = bookmark_map.get(&form.builtin_alias);
    let description = bookmark
        .map(|cmd| cmd.description())
        .unwrap_or("Built-in bookmark");
    let url = bookmark
        .map(|cmd| cmd.base_url())
        .unwrap_or("");
    let template_str = bookmark
        .map(|cmd| match cmd {
            Command::Variable { template, .. } => render_template_to_string(template),
            Command::Nested { .. } => String::new(),
        })
        .unwrap_or_default();

    let template = GlobalBookmarkRowTemplate {
        alias: &form.builtin_alias,
        description,
        url,
        template: &template_str,
        is_disabled,
        is_admin: current_user.0.is_admin,
    };
    Ok(Html(template.render()?))
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
    use crate::services::serializers::{YamlSerializer, JsonSerializer};

    // Determine scope - only admins can import global
    let scope = if form.scope == "global" {
        if !current_user.0.is_admin {
            return Err(AppError::Forbidden("Only admins can import global bookmarks".to_string()));
        }
        db::BookmarkScope::Global
    } else {
        db::BookmarkScope::Personal { user_id: current_user.0.id }
    };

    // Get content based on source
    let content = match form.source.as_str() {
        "paste" => form.content.ok_or(AppError::BadRequest("No content provided".to_string()))?,
        "url" => {
            // Fetch from URL
            let url = form.url.ok_or(AppError::BadRequest("No URL provided".to_string()))?;

            // Validate URL for SSRF protection (first check)
            validation::validate_url_for_fetch(&url)?;

            // Parse URL to get host and port for DNS resolution
            let parsed_url = url.parse::<url::Url>()
                .map_err(|_| AppError::BadRequest("Invalid URL format".to_string()))?;

            let host = parsed_url.host_str()
                .ok_or(AppError::BadRequest("URL must have a host".to_string()))?;

            let port = parsed_url.port_or_known_default()
                .ok_or(AppError::BadRequest("Cannot determine port for URL".to_string()))?;

            // Manually resolve hostname to prevent DNS rebinding attacks
            // This ensures we validate the ACTUAL IP that will be connected to
            let socket_addrs = tokio::net::lookup_host(format!("{}:{}", host, port))
                .await
                .map_err(|e| AppError::Internal(format!("DNS resolution failed: {}", e)))?
                .collect::<Vec<_>>();

            if socket_addrs.is_empty() {
                return Err(AppError::BadRequest("Could not resolve hostname".to_string()));
            }

            // Validate ALL resolved IPs to prevent DNS rebinding
            // An attacker could configure DNS to return multiple IPs (public + private)
            for socket_addr in &socket_addrs {
                validation::validate_resolved_ip(socket_addr.ip())?;
            }

            // Create a client with timeout and redirect disabled
            // Disabling redirects helps mitigate DNS rebinding attacks where
            // the initial request passes validation but redirects to internal IPs
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

            // Use reqwest to fetch URL content
            let response = client.get(&url)
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to fetch URL: {}", e)))?;

            if !response.status().is_success() {
                // Handle redirects explicitly - we don't follow them for security
                if response.status().is_redirection() {
                    return Err(AppError::BadRequest("URL redirects are not allowed for security".to_string()));
                }
                return Err(AppError::Internal(format!("URL returned status {}", response.status())));
            }

            // Stream response and limit actual bytes received (not trusting Content-Length)
            const MAX_SIZE: usize = 1_000_000;
            let bytes = response.bytes()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to read URL content: {}", e)))?;

            if bytes.len() > MAX_SIZE {
                return Err(AppError::BadRequest("URL content too large (max 1MB)".to_string()));
            }

            String::from_utf8(bytes.to_vec())
                .map_err(|_| AppError::BadRequest("URL content is not valid UTF-8".to_string()))?
        }
        _ => return Err(AppError::BadRequest("Invalid import source".to_string())),
    };

    // Validate content size (protect against YAML bombs and large files)
    if content.len() > 1_000_000 {
        return Err(AppError::BadRequest("Content too large (max 1MB)".to_string()));
    }

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

    let template = SuccessWithLinkTemplate {
        message: &message,
        link: "/manage",
        link_text: "Refresh to see changes",
    };
    Ok(Html(template.render()?))
}

// GET /manage/export - Export bookmarks
pub async fn export_bookmarks(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    axum::extract::Query(params): axum::extract::Query<ExportParams>,
) -> Result<Response, AppError> {
    use crate::services::serializers::{YamlSerializer, JsonSerializer};

    // Check permissions for global export
    if params.scope == "global" && !current_user.0.is_admin {
        return Err(AppError::Forbidden("Only admins can export global bookmarks".to_string()));
    }

    let scope = if params.scope == "global" {
        db::BookmarkScope::Global
    } else {
        db::BookmarkScope::Personal { user_id: current_user.0.id }
    };

    let serializer: Box<dyn crate::services::serializers::BookmarkSerializer> = match params.format.as_str() {
        "json" => Box::new(JsonSerializer),
        _ => Box::new(YamlSerializer),
    };

    let content = state.bookmark_service.export_bookmarks(
        scope,
        serializer.as_ref(),
    ).await
    .map_err(|e| AppError::Internal(format!("Export failed: {}", e)))?;

    let is_global = matches!(scope, db::BookmarkScope::Global);
    let filename = format!(
        "bookmarks_{}.{}",
        if is_global { "global" } else { "personal" },
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
                i as i32,
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fork nested bookmark: {}", e)))?;
        }
    }

    let message = format!("Forked '{}' to your personal bookmarks!", form.alias);
    let template = SuccessWithLinkTemplate {
        message: &message,
        link: "/manage",
        link_text: "Refresh to see changes",
    };
    Ok(Html(template.render()?))
}
