// Error handling for brunnylol

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use std::fmt;

/// HTML escape a string to prevent XSS
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Application error type
#[derive(Debug)]
pub enum AppError {
    TemplateRender(String),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::TemplateRender(msg) => write!(f, "Template rendering error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

// Implement IntoResponse so Axum can convert errors to HTTP responses
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // For Unauthorized errors, redirect to login page with return URL
        if let AppError::Unauthorized(return_path) = self {
            let redirect_url = if return_path.is_empty() || return_path == "/" {
                "/login".to_string()
            } else {
                format!("/login?return={}", urlencoding::encode(&return_path))
            };
            return Redirect::to(&redirect_url).into_response();
        }

        let (status, message) = match &self {
            AppError::TemplateRender(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", msg),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, format!("Not found: {}", msg)),
            AppError::Unauthorized(_) => unreachable!(), // Handled above
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, format!("Forbidden: {}", msg)),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, format!("Bad request: {}", msg)),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {}", msg),
            ),
        };

        // Return an HTML error page styled to match the rest of the site
        // (same stylesheet, dark theme, and nav as the regular pages),
        // with the message escaped to prevent XSS.
        let error_html = format!(
            r##"<!doctype html>
<html data-theme="dark">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="theme-color" content="#1a1a2e" />
    <title>{} - Brunnylol</title>
    <link rel="stylesheet" href="/static/css/main.css">
    <link rel="icon" type="image/svg+xml" href="/static/favicon.svg">
    <link rel="icon" type="image/png" sizes="32x32" href="/static/favicon-32.png">
    <link rel="apple-touch-icon" sizes="180x180" href="/static/apple-touch-icon.png">
</head>
<body>
    <nav>
      <div>
        <a href="/about">About</a>
        <a href="/help">Help</a>
      </div>
      <div>
        <a href="/login">Login</a>
      </div>
    </nav>
    <div class="container-sm" style="margin-top: var(--space-12); text-align: center;">
        <h1 style="font-size: 3rem; margin-bottom: var(--space-3);">{}</h1>
        <p style="color: var(--text-secondary); margin-bottom: var(--space-6);">{}</p>
        <a href="/" class="btn btn-primary" style="display: inline-block; padding: var(--space-3) var(--space-5); text-decoration: none;">Return to home</a>
    </div>
    <div class="footer">
        <p>Jacob Rodal © 2020</p>
    </div>
</body>
</html>"##,
            html_escape(status.as_str()),
            html_escape(status.as_str()),
            html_escape(&message)
        );

        (status, Html(error_html)).into_response()
    }
}

// Helper to convert template errors
impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::TemplateRender(err.to_string())
    }
}

// Extension trait for database result handling
pub trait DbResultExt<T> {
    /// Convert database errors to AppError::Internal with "Database error: " prefix
    fn db_err(self) -> Result<T, AppError>;
}

impl<T, E: std::fmt::Display> DbResultExt<T> for Result<T, E> {
    fn db_err(self) -> Result<T, AppError> {
        self.map_err(|e| AppError::Internal(format!("Database error: {}", e)))
    }
}
