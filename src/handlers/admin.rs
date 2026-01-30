// Admin panel handlers

use askama::Template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth::middleware::AdminUser, db, error::{AppError, DbResultExt}, validation};
use super::common::{ErrorTemplate, SuccessTemplate, SuccessWithLinkTemplate};

// Template struct
#[derive(Template)]
#[template(path = "admin.html")]
struct AdminTemplate {
    users: Vec<UserDisplay>,
}

#[derive(Clone)]
struct UserDisplay {
    id: i64,
    username: String,
    is_admin: bool,
    bookmark_count: usize,
}

// GET /admin - Admin panel
pub async fn admin_page(
    _admin_user: AdminUser,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Html<String>, AppError> {
    // Get all users
    let all_users = db::list_all_users(&state.db_pool)
        .await
        .db_err()?;

    // Get bookmark counts for each user
    let mut users_display = Vec::new();
    for user in all_users {
        let bookmarks = db::get_bookmarks(&state.db_pool, db::BookmarkScope::Personal { user_id: user.id })
            .await
            .db_err()?;

        users_display.push(UserDisplay {
            id: user.id,
            username: user.username.clone(),
            is_admin: user.is_admin,
            bookmark_count: bookmarks.len(),
        });
    }

    let template = AdminTemplate {
        users: users_display,
    };

    Ok(Html(template.render()?))
}

// POST /admin/cleanup-sessions - Clean up expired sessions
pub async fn cleanup_sessions(
    _admin_user: AdminUser,
    State(state): State<Arc<crate::AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = db::cleanup_expired_sessions(&state.db_pool)
        .await
        .db_err()?;

    let message = format!("Cleaned up {} expired sessions", deleted);
    let template = SuccessTemplate { message: &message };
    Ok(Html(template.render()?))
}

// Form struct for creating users
#[derive(Deserialize)]
pub struct CreateUserForm {
    username: String,
    password: String,
    confirm_password: String,
    is_admin: Option<String>, // checkbox
}

// POST /admin/create-user - Create new user
pub async fn create_user(
    _admin_user: AdminUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<CreateUserForm>,
) -> Result<impl IntoResponse, AppError> {
    // Validate passwords match
    if validation::validate_passwords_match(&form.password, &form.confirm_password).is_err() {
        let template = ErrorTemplate { message: "Passwords do not match" };
        return Ok(Html(template.render()?));
    }

    // Validate username
    if let Err(e) = crate::auth::validate_username(&form.username) {
        let template = ErrorTemplate { message: &e.to_string() };
        return Ok(Html(template.render()?));
    }

    // Validate password
    if let Err(e) = crate::auth::validate_password(&form.password) {
        let template = ErrorTemplate { message: &e.to_string() };
        return Ok(Html(template.render()?));
    }

    // Hash password
    let password_hash = crate::auth::hash_password(&form.password)
        .map_err(|e| AppError::Internal(format!("Password hashing error: {}", e)))?;

    let is_admin = form.is_admin.is_some();

    // Create user directly (bypass first-user-admin logic)
    sqlx::query("INSERT INTO users (username, password_hash, is_admin) VALUES (?, ?, ?)")
        .bind(&form.username)
        .bind(&password_hash)
        .bind(is_admin)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                AppError::BadRequest("Username already exists".to_string())
            } else {
                AppError::Internal(format!("User creation error: {}", e))
            }
        })?;

    let message = format!("User '{}' created successfully!", form.username);
    let template = SuccessWithLinkTemplate {
        message: &message,
        link: "/admin",
        link_text: "Refresh",
    };
    Ok(Html(template.render()?))
}

// POST /admin/reload-global - Reload global bookmarks from database
pub async fn reload_global_bookmarks(
    _admin_user: AdminUser,
    State(state): State<Arc<crate::AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // Reload bookmarks from database
    let new_map = state.bookmark_service
        .load_global_bookmarks()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to reload bookmarks: {}", e)))?;

    // Atomic swap with write lock
    let mut write_lock = state.alias_to_bookmark_map.write().await;
    *write_lock = new_map;
    drop(write_lock);

    let template = SuccessTemplate {
        message: "Global bookmarks reloaded successfully",
    };
    Ok(Html(template.render()?))
}
