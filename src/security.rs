// Security middleware for HTTP security headers

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Middleware that adds security headers to all responses
pub async fn security_headers(request: Request, next: Next) -> Response<Body> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Prevent clickjacking - deny all framing
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );

    // Prevent MIME type sniffing
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // Enable browser XSS filter (legacy, but still useful)
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Referrer policy - don't leak full URL to third parties
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Content Security Policy - restrict content sources
    // Allow self for scripts/styles, and htmx from unpkg CDN
    // Note: unsafe-eval required for HTMX 1.9.12 (uses new Function() internally)
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' https://unpkg.com 'unsafe-eval'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             font-src 'self'; \
             connect-src 'self'; \
             frame-ancestors 'none'; \
             base-uri 'self'; \
             form-action 'self'"
        ),
    );

    // Permissions Policy - restrict browser features
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static(
            "accelerometer=(), camera=(), geolocation=(), gyroscope=(), \
             magnetometer=(), microphone=(), payment=(), usb=()"
        ),
    );

    response
}
