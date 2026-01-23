// Error handling for brunnylol

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::fmt;

/// Application error type
#[derive(Debug)]
pub enum AppError {
    TemplateRender(String),
    #[allow(dead_code)] // Reserved for future use
    NotFound(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::TemplateRender(msg) => write!(f, "Template rendering error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

// Implement IntoResponse so Axum can convert errors to HTTP responses
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::TemplateRender(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", msg),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, format!("Not found: {}", msg)),
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
