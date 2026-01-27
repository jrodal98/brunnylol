// Admin panel handlers

use askama::Template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth::middleware::CurrentUser, db, error::{AppError, DbResultExt}, validation};

// Template struct
#[derive(Template)]
#[template(path = "admin.html")]
struct AdminTemplate {
    user: db::User,
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
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
) -> Result<Html<String>, AppError> {
    // Check if user is admin
    if !current_user.0.is_admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    // Get all users
    let all_users = db::list_all_users(&state.db_pool)
        .await
        .db_err()?;

    // Get bookmark counts for each user
    let mut users_display = Vec::new();
    for user in all_users {
        let bookmarks = db::get_user_bookmarks(&state.db_pool, user.id)
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
        user: current_user.0,
        users: users_display,
    };

    Ok(Html(template.render()?))
}

// POST /admin/cleanup-sessions - Clean up expired sessions
pub async fn cleanup_sessions(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
) -> Result<impl IntoResponse, AppError> {
    if !current_user.0.is_admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let deleted = db::cleanup_expired_sessions(&state.db_pool)
        .await
        .db_err()?;

    Ok(Html(format!("<div>Cleaned up {} expired sessions</div>", deleted)))
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
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<CreateUserForm>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user is admin
    if !current_user.0.is_admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    // Validate passwords match
    if let Err(_) = validation::validate_passwords_match(&form.password, &form.confirm_password) {
        return Ok(Html(
            r#"<div style="color: #d32f2f;">Passwords do not match</div>"#.to_string()
        ));
    }

    // Validate username
    if let Err(e) = crate::auth::validate_username(&form.username) {
        return Ok(Html(format!(r#"<div style="color: #d32f2f;">{}</div>"#, e)));
    }

    // Validate password
    if let Err(e) = crate::auth::validate_password(&form.password) {
        return Ok(Html(format!(r#"<div style="color: #d32f2f;">{}</div>"#, e)));
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

    Ok(Html(format!(
        r#"<div class="success-message">User '{}' created successfully! <a href="/admin">Refresh</a></div>"#,
        form.username
    )))
}
