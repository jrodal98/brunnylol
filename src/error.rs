// Error handling for brunnylol

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use std::fmt;

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
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal error: {}", msg)),
        };

        // Return a simple HTML error page
        let error_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Error - Brunnylol</title>
    <style>
        body {{
            font-family: Arial, sans-serif;
            max-width: 600px;
            margin: 100px auto;
            text-align: center;
        }}
        h1 {{ color: #d32f2f; }}
        p {{ color: #666; }}
    </style>
</head>
<body>
    <h1>{}</h1>
    <p>{}</p>
    <p><a href="/">Return to home</a></p>
</body>
</html>"#,
            status.as_str(),
            message
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
