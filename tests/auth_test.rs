// Integration tests for authentication and bookmark management

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

async fn create_test_app() -> axum::Router {
    // Use test database
    std::env::set_var("DATABASE_URL", "sqlite::memory:");
    brunnylol::create_router().await
}

#[tokio::test]
async fn test_register_page_loads() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/register").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Register"));
    assert!(body_str.contains("first user"));
}

#[tokio::test]
async fn test_login_page_loads() {
    let app = create_test_app().await;

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
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/manage").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should return Unauthorized without session cookie
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_page_requires_admin() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/admin").body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should return Unauthorized without session cookie
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_navigation_links_present() {
    let app = create_test_app().await;

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
