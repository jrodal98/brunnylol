// Authentication handlers

use askama::Template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use std::sync::Arc;

use crate::{auth, db, error::AppError};

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

// GET /register - Show registration page
pub async fn register_page(
    State(state): State<Arc<crate::AppState>>,
) -> Result<Html<String>, AppError> {
    // Check if this would be the first user
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    let template = RegisterTemplate {
        error: String::new(),
        is_first_user: count == 0,
    };
    Ok(Html(template.render()?))
}

// POST /register - Process registration
pub async fn register_submit(
    jar: CookieJar,
    State(state): State<Arc<crate::AppState>>,
    Form(form): Form<RegisterForm>,
) -> Result<impl IntoResponse, AppError> {
    // Check if this would be the first user
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

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
