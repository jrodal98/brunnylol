// Integration tests for admin user management:
// promoting/demoting users and deleting users from the admin panel.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

async fn body_string(resp: axum::response::Response) -> String {
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

fn req(method: &str, uri: &str, cookie: Option<&str>, form: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri(uri).method(method);
    if let Some(c) = cookie {
        builder = builder.header("Cookie", c);
    }
    if form.is_some() {
        builder = builder.header("Content-Type", "application/x-www-form-urlencoded");
    }
    common::add_test_socket_addr(
        builder
            .body(Body::from(form.unwrap_or("").to_string()))
            .unwrap(),
    )
}

/// Register the first user (becomes admin) and return its session cookie.
async fn register_admin(app: &axum::Router) -> String {
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            "/register",
            None,
            Some("username=admin1&password=<redacted>&confirm_password=<redacted>"),
        ))
        .await
        .unwrap();
    assert!(
        resp.status().is_redirection() || resp.status().is_success(),
        "admin registration failed: {}",
        resp.status()
    );
    let cookie = resp
        .headers()
        .get("set-cookie")
        .expect("register should set a session cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    cookie
}

/// Extract the user id from the (only) toggle-admin URL on the admin page.
fn parse_other_user_id(admin_html: &str) -> i64 {
    let marker = "/admin/users/";
    let start = admin_html
        .find(marker)
        .expect("admin page should contain a toggle-admin URL for the other user")
        + marker.len();
    let rest = &admin_html[start..];
    let end = rest.find('/').unwrap();
    rest[..end].parse().unwrap()
}

#[tokio::test]
async fn test_admin_can_promote_demote_and_delete_user() {
    let app = common::create_test_app().await;
    let admin_cookie = register_admin(&app).await;

    // Admin creates a second, regular user
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            "/admin/create-user",
            Some(&admin_cookie),
            Some("username=second&password=<redacted>&confirm_password=<redacted>"),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Find the second user's id from the admin page
    let resp = app
        .clone()
        .oneshot(req("GET", "/admin", Some(&admin_cookie), None))
        .await
        .unwrap();
    let html = body_string(resp).await;
    assert!(html.contains("<strong>second</strong>"));
    let second_id = parse_other_user_id(&html);
    assert_ne!(second_id, 1, "first registered user should be the admin");

    // Promote: response is the updated row showing ADMIN + Remove admin
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            &format!("/admin/users/{}/toggle-admin", second_id),
            Some(&admin_cookie),
            Some(""),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let row = body_string(resp).await;
    assert!(
        row.contains("ADMIN"),
        "promoted row should show ADMIN: {}",
        row
    );
    assert!(row.contains("Remove admin"));

    // Demote back
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            &format!("/admin/users/{}/toggle-admin", second_id),
            Some(&admin_cookie),
            Some(""),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let row = body_string(resp).await;
    assert!(row.contains("Make admin"));

    // Guards: the admin cannot demote or delete themselves
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            "/admin/users/1/toggle-admin",
            Some(&admin_cookie),
            Some(""),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            "/admin/users/1/delete",
            Some(&admin_cookie),
            Some(""),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Delete the second user
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            &format!("/admin/users/{}/delete", second_id),
            Some(&admin_cookie),
            Some(""),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Gone from the admin list, and can no longer log in
    let resp = app
        .clone()
        .oneshot(req("GET", "/admin", Some(&admin_cookie), None))
        .await
        .unwrap();
    let html = body_string(resp).await;
    assert!(
        !html.contains("<strong>second</strong>"),
        "deleted user should not be listed"
    );

    let resp = app
        .oneshot(req(
            "POST",
            "/login",
            None,
            Some("username=second&password=<redacted>"),
        ))
        .await
        .unwrap();
    let html = body_string(resp).await;
    assert!(html.contains("Invalid username or password"));
}

#[tokio::test]
async fn test_user_management_requires_admin() {
    let app = common::create_test_app().await;
    let _admin_cookie = register_admin(&app).await;

    // Unauthenticated calls redirect to login
    let resp = app
        .oneshot(req("POST", "/admin/users/1/delete", None, Some("")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
}
