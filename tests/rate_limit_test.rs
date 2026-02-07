// E2E tests for rate limiting

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Helper to create request with specific IP
fn request_with_ip(uri: &str, ip: [u8; 4]) -> Request<Body> {
    let body = if uri == "/register" {
        "username=test&password=testpass123&confirm_password=testpass123"
    } else {
        "username=test&password=wrong"
    };

    let mut req = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();

    req.extensions_mut().insert(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])),
        8000
    ));

    req
}

#[tokio::test]
async fn test_login_rate_limit_blocks_after_5_attempts() {
    let app = common::create_test_app().await;

    // First 5 attempts should succeed (get redirected)
    for i in 1..=5 {
        let response = app
            .clone()
            .oneshot(request_with_ip("/login", [192, 168, 1, 1]))
            .await
            .unwrap();

        assert!(
            response.status().is_redirection() || response.status().is_success(),
            "Attempt {} should not be rate limited", i
        );
    }

    // 6th attempt should be rate limited
    let response = app
        .clone()
        .oneshot(request_with_ip("/login", [192, 168, 1, 1]))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Too many requests"));
    assert!(body_str.contains("wait"));
}

#[tokio::test]
async fn test_different_ips_have_separate_quotas() {
    let app = common::create_test_app().await;

    // Exhaust quota for first IP (5 attempts)
    for _ in 1..=5 {
        let _ = app
            .clone()
            .oneshot(request_with_ip("/login", [192, 168, 2, 1]))
            .await
            .unwrap();
    }

    // 6th attempt from first IP should be blocked
    let response = app
        .clone()
        .oneshot(request_with_ip("/login", [192, 168, 2, 1]))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Request from different IP should still work
    let response = app
        .clone()
        .oneshot(request_with_ip("/login", [192, 168, 2, 2]))
        .await
        .unwrap();

    assert!(
        response.status().is_redirection() || response.status().is_success(),
        "Different IP should not be rate limited"
    );
}

#[tokio::test]
async fn test_register_route_also_rate_limited() {
    let app = common::create_test_app().await;

    // First 5 attempts should work
    for i in 1..=5 {
        let response = app
            .clone()
            .oneshot(request_with_ip("/register", [192, 168, 3, 1]))
            .await
            .unwrap();

        let status = response.status();

        // Register returns either success/redirect or forbidden (if user exists)
        // Debug: print actual status if test fails
        assert!(
            status.is_redirection()
                || status.is_success()
                || status == StatusCode::FORBIDDEN,
            "Attempt {} should not be rate limited, got status: {}", i, status
        );
    }

    // 6th attempt should be rate limited
    let response = app
        .oneshot(request_with_ip("/register", [192, 168, 3, 1]))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_x_forwarded_for_header_respected() {
    let app = common::create_test_app().await;

    // Make requests with X-Forwarded-For header
    for i in 1..=5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/login")
                    .method("POST")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("X-Forwarded-For", "203.0.113.25")
                    .body(Body::from("username=test&password=wrong"))
                    .unwrap()
            )
            .await
            .unwrap();

        assert!(
            response.status().is_redirection() || response.status().is_success(),
            "Attempt {} should not be rate limited", i
        );
    }

    // 6th attempt should be blocked
    let response = app
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("X-Forwarded-For", "203.0.113.25")
                .body(Body::from("username=test&password=wrong"))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
