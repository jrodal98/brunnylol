// Rate limiting configuration and error handling

use axum::{
    body::Body,
    http::{Response, StatusCode},
};
use tower_governor::GovernorError;

/// Custom error handler for rate limiting errors
pub fn rate_limit_error_handler(error: GovernorError) -> Response<Body> {
    match error {
        GovernorError::TooManyRequests { wait_time, headers } => {
            let message = format!(
                "Too many requests. Please wait {} seconds before trying again.",
                wait_time
            );

            let mut response = Response::new(Body::from(message));
            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;

            if let Some(header_map) = headers {
                response.headers_mut().extend(header_map);
            }

            response
        }
        GovernorError::UnableToExtractKey => {
            let mut response = Response::new(Body::from("Unable to extract client IP address"));
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response
        }
        GovernorError::Other { code, msg, headers } => {
            let message = msg.unwrap_or_else(|| "Rate limiting error".to_string());
            let mut response = Response::new(Body::from(message));
            *response.status_mut() = code;

            if let Some(header_map) = headers {
                response.headers_mut().extend(header_map);
            }

            response
        }
    }
}
