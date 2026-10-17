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

    // First registration should succeed (no users exist yet)
    let first_response = app
        .clone()
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/register")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    // First user registration page should load successfully
    assert_eq!(first_response.status(), StatusCode::OK);

    // Now create a user via the registration endpoint
    let register_form = "username=testadmin&password=testpass123&confirm_password=testpass123";
    let create_user_response = app
        .clone()
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/register")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(register_form))
                .unwrap(),
        ))
        .await
        .unwrap();

    // User creation should succeed
    assert!(
        create_user_response.status().is_redirection()
            || create_user_response.status().is_success()
    );

    // Second attempt to access registration page should be blocked
    let blocked_response = app
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/register")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    // Should return Forbidden since admin user now exists
    assert_eq!(blocked_response.status(), StatusCode::FORBIDDEN);

    let body = axum::body::to_bytes(blocked_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Registration is closed") || body_str.contains("Forbidden"));
}

#[tokio::test]
async fn test_login_page_loads() {
    let app = common::create_test_app().await;

    let response = app
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        ))
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
        .oneshot(
            Request::builder()
                .uri("/manage")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should redirect to login with return parameter
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains("/login"));
    assert!(location.contains("return=%2Fmanage")); // URL-encoded /manage
}

#[tokio::test]
async fn test_admin_page_requires_admin() {
    let app = common::create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should redirect to login with return parameter
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
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

#[tokio::test]
async fn test_login_unknown_user_shows_error_on_page() {
    let app = common::create_test_app().await;

    // Create a first user so the instance is fully set up
    let register_form = "username=someone&password=<redacted>&confirm_password=<redacted>";
    let resp = app
        .clone()
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/register")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(register_form))
                .unwrap(),
        ))
        .await
        .unwrap();
    assert!(resp.status().is_redirection() || resp.status().is_success());

    // Log in with a username that does not exist: the error must be
    // rendered on the returned login page (regression test: it used to
    // redirect to /login?return=<message>, where nothing displayed it).
    let login_form = "username=nosuchuser&password=<redacted>";
    let resp = app
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(login_form))
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        html.contains("Invalid username or password"),
        "login page should show the error inline, got: {}",
        &html[..html.len().min(400)]
    );
}

#[tokio::test]
async fn test_register_links_hidden_once_registration_closes() {
    let app = common::create_test_app().await;

    // Fresh instance (no users): the home page offers Register
    let resp = app
        .clone()
        .oneshot(common::add_test_socket_addr(
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        ))
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(String::from_utf8(body.to_vec())
        .unwrap()
        .contains("href=\"/register\""));

    // Create the first user; registration is now closed
    let register_form = "username=firstuser&password=<redacted>&confirm_password=<redacted>";
    let resp = app
        .clone()
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/register")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(register_form))
                .unwrap(),
        ))
        .await
        .unwrap();
    assert!(resp.status().is_redirection() || resp.status().is_success());

    // Home and login pages must no longer link to /register
    for path in ["/", "/login"] {
        let resp = app
            .clone()
            .oneshot(common::add_test_socket_addr(
                Request::builder().uri(path).body(Body::empty()).unwrap(),
            ))
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            !html.contains("href=\"/register\""),
            "{} should not link to /register once registration is closed",
            path
        );
    }
}

#[tokio::test]
async fn test_about_page_loads() {
    let app = common::create_test_app().await;

    let resp = app
        .oneshot(common::add_test_socket_addr(
            Request::builder()
                .uri("/about")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("About Brunnylol"));
    // Nav About link points at the real page now
    assert!(html.contains("href=\"/about\""));
}
