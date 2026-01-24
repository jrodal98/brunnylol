// Authentication handlers

use askama::Template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth, auth::middleware::CurrentUser, db, error::AppError};

// Template structs
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: String,
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    error: String,
    is_first_user: bool,
}

// Form structs
#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RegisterForm {
    username: String,
    password: String,
    confirm_password: String,
}

// GET /login - Show login page
pub async fn login_page() -> Result<Html<String>, AppError> {
    let template = LoginTemplate { error: String::new() };
    Ok(Html(template.render()?))
}

// POST /login - Process login
pub async fn login_submit(
    jar: CookieJar,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    // Get user from database
    let user_data = db::get_user_by_username(&state.db_pool, &form.username)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let (user_id, password_hash, _is_admin) = user_data
        .ok_or(AppError::Unauthorized("Invalid username or password".to_string()))?;

    // Verify password
    let valid = auth::verify_password(&form.password, &password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification error: {}", e)))?;

    if !valid {
        // Return login page with error
        let template = LoginTemplate {
            error: "Invalid username or password".to_string(),
        };
        return Ok(Html(template.render()?).into_response());
    }

    // Create session
    let session_id = db::create_session(&state.db_pool, user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Session creation error: {}", e)))?;

    // Set secure cookie and return with redirect
    let cookie = Cookie::build(("session_id", session_id.clone()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::hours(24))
        .build();

    Ok((jar.add(cookie), Redirect::to("/manage")).into_response())
}

// GET /register - Show registration page (only if no users exist)
pub async fn register_page(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Html<String>, AppError> {
    // Check if any users exist
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    // If users exist, registration is closed
    if count > 0 {
        return Err(AppError::Forbidden(
            "Registration is closed. Please contact an administrator to create an account.".to_string()
        ));
    }

    let template = RegisterTemplate {
        error: String::new(),
        is_first_user: count == 0,
    };
    Ok(Html(template.render()?))
}

// POST /register - Process registration (only if no users exist)
pub async fn register_submit(
    jar: CookieJar,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<RegisterForm>,
) -> Result<impl IntoResponse, AppError> {
    // Check if any users exist
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    // If users exist, registration is closed
    if count > 0 {
        return Err(AppError::Forbidden(
            "Registration is closed. Please contact an administrator.".to_string()
        ));
    }

    // Validate passwords match
    if form.password != form.confirm_password {
        let template = RegisterTemplate {
            error: "Passwords do not match".to_string(),
            is_first_user: count == 0,
        };
        return Ok(Html(template.render()?).into_response());
    }

    // Validate username
    if let Err(e) = auth::validate_username(&form.username) {
        let template = RegisterTemplate {
            error: e.to_string(),
            is_first_user: count == 0,
        };
        return Ok(Html(template.render()?).into_response());
    }

    // Validate password
    if let Err(e) = auth::validate_password(&form.password) {
        let template = RegisterTemplate {
            error: e.to_string(),
            is_first_user: count == 0,
        };
        return Ok(Html(template.render()?).into_response());
    }

    // Hash password
    let password_hash = auth::hash_password(&form.password)
        .map_err(|e| AppError::Internal(format!("Password hashing error: {}", e)))?;

    // Create user
    let user = db::create_user(&state.db_pool, &form.username, &password_hash)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                AppError::BadRequest("Username already exists".to_string())
            } else {
                AppError::Internal(format!("User creation error: {}", e))
            }
        })?;

    // Create session
    let session_id = db::create_session(&state.db_pool, user.id)
        .await
        .map_err(|e| AppError::Internal(format!("Session creation error: {}", e)))?;

    // Set secure cookie and return with redirect
    let cookie = Cookie::build(("session_id", session_id.clone()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::hours(24))
        .build();

    Ok((jar.add(cookie), Redirect::to("/manage")).into_response())
}

// POST /logout - Logout user
pub async fn logout(
    jar: CookieJar,
    State(state): State<Arc<crate::AppState>>,
) -> impl IntoResponse {
    // Delete session if exists
    if let Some(session_cookie) = jar.get("session_id") {
        let _ = db::delete_session(&state.db_pool, session_cookie.value()).await;
    }

    // Remove cookie and return redirect
    let jar = jar.remove(Cookie::from("session_id"));

    (jar, Redirect::to("/")).into_response()
}

// Template for settings page
#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
    user: db::User,
}

// GET /settings - User settings page
pub async fn settings_page(
    current_user: CurrentUser,
) -> Result<Html<String>, AppError> {
    let template = SettingsTemplate {
        user: current_user.0,
    };
    Ok(Html(template.render()?))
}

// Form structs for settings
#[derive(Deserialize)]
pub struct ChangeUsernameForm {
    new_username: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    current_password: String,
    new_password: String,
    confirm_password: String,
}

// POST /settings/username - Change username
pub async fn change_username(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<ChangeUsernameForm>,
) -> Result<impl IntoResponse, AppError> {
    // Validate new username
    if let Err(e) = auth::validate_username(&form.new_username) {
        return Ok(Html(format!(r#"<div style="color: #d32f2f;">{}</div>"#, e)));
    }

    // Update username in database
    sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(&form.new_username)
        .bind(current_user.0.id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                AppError::BadRequest("Username already taken".to_string())
            } else {
                AppError::Internal(format!("Database error: {}", e))
            }
        })?;

    Ok(Html(format!(
        r#"<div class="success-message">Username updated to '{}'! Please <a href="/logout">log out</a> and log back in.</div>"#,
        form.new_username
    )))
}

// POST /settings/password - Change password
pub async fn change_password(
    current_user: CurrentUser,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<ChangePasswordForm>,
) -> Result<impl IntoResponse, AppError> {
    // Get current password hash from database
    let user_data = db::get_user_by_username(&state.db_pool, &current_user.0.username)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?
        .ok_or(AppError::Internal("User not found".to_string()))?;

    let (_user_id, password_hash, _is_admin) = user_data;

    // Verify current password
    let valid = auth::verify_password(&form.current_password, &password_hash)
        .map_err(|e| AppError::Internal(format!("Password verification error: {}", e)))?;

    if !valid {
        return Ok(Html(
            r#"<div style="color: #d32f2f;">Current password is incorrect</div>"#.to_string()
        ));
    }

    // Validate new passwords match
    if form.new_password != form.confirm_password {
        return Ok(Html(
            r#"<div style="color: #d32f2f;">New passwords do not match</div>"#.to_string()
        ));
    }

    // Validate new password strength
    if let Err(e) = auth::validate_password(&form.new_password) {
        return Ok(Html(format!(r#"<div style="color: #d32f2f;">{}</div>"#, e)));
    }

    // Hash new password
    let new_hash = auth::hash_password(&form.new_password)
        .map_err(|e| AppError::Internal(format!("Password hashing error: {}", e)))?;

    // Update password in database
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&new_hash)
        .bind(current_user.0.id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    Ok(Html(
        r#"<div class="success-message">Password updated successfully!</div>"#.to_string()
    ))
}
