// Admin panel handlers

use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use std::sync::Arc;

use crate::{auth::middleware::CurrentUser, db, error::AppError};

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
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    // Get bookmark counts for each user
    let mut users_display = Vec::new();
    for user in all_users {
        let bookmarks = db::get_user_bookmarks(&state.db_pool, user.id)
            .await
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

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
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    Ok(Html(format!("<div>Cleaned up {} expired sessions</div>", deleted)))
}
