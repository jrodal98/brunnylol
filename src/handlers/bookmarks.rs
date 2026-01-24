// Bookmark management handlers

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{Html, IntoResponse, Redirect},
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
}

// Form structs
#[derive(Deserialize)]
pub struct CreateBookmarkForm {
    alias: String,
    bookmark_type: String,  // "simple", "templated", or "nested"
    url: String,
    description: String,
    command_template: Option<String>,
    encode_query: Option<String>, // checkbox value
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

    // Get global alias list for conflict detection
    let user_aliases: std::collections::HashSet<String> = personal_bookmarks
        .iter()
        .map(|b| b.alias.clone())
        .collect();

    let mut global_bookmarks = Vec::new();
    let mut conflicts = Vec::new();

    for alias in state.alias_to_bookmark_map.keys() {
        let is_overridden = user_aliases.contains(alias);
        if is_overridden {
            conflicts.push(alias.clone());
        }
        global_bookmarks.push(GlobalBookmarkDisplay {
            alias: alias.clone(),
            is_overridden,
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

    let encode_query = form.encode_query.is_some();

    // Create bookmark in database
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

// POST /manage/bookmark/:id/nested - Add nested bookmark
pub async fn create_nested_bookmark(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<CreateNestedForm>,
) -> Result<impl IntoResponse, AppError> {
    let encode_query = form.encode_query.is_some();

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

    Ok(Html("<div>Nested bookmark added successfully!</div>"))
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
