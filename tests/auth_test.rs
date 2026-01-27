// Integration tests for authentication and bookmark management

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn test_register_page_blocked_after_first_user() {
    let app = common::create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/register").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should return Forbidden since admin user already exists
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Registration is closed") || body_str.contains("Forbidden"));
}

#[tokio::test]
async fn test_login_page_loads() {
    let app = common::create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/login").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Login"));
}

#[tokio::test]
async fn test_manage_page_requires_auth() {
    let app = common::create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/manage").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should redirect to login with return parameter
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();
    assert!(location.contains("/login"));
    assert!(location.contains("return=%2Fmanage")); // URL-encoded /manage
}

#[tokio::test]
async fn test_admin_page_requires_admin() {
    let app = common::create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/admin").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should redirect to login with return parameter
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();
    assert!(location.contains("/login"));
    assert!(location.contains("return=%2Fadmin")); // URL-encoded /admin
}

#[tokio::test]
async fn test_navigation_links_present() {
    let app = common::create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Check for navigation
    assert!(body_str.contains("Login") || body_str.contains("login"));
    assert!(body_str.contains("Register") || body_str.contains("register"));
    assert!(body_str.contains("Help") || body_str.contains("help"));
}
