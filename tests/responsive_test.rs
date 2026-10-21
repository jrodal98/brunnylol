// Responsive layout guards: the stylesheets must carry the small-screen
// rules (stacked nav, scrollable tab bar, table scroll wrappers) and the
// data-table pages must actually wrap their tables in .table-scroll, so
// phone-width screens never push the page itself sideways.

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

async fn get(app: &axum::Router, uri: &str, cookie: Option<&str>) -> axum::response::Response {
    let mut builder = Request::builder().uri(uri).method("GET");
    if let Some(c) = cookie {
        builder = builder.header("Cookie", c);
    }
    app.clone()
        .oneshot(common::add_test_socket_addr(
            builder.body(Body::empty()).unwrap(),
        ))
        .await
        .unwrap()
}

/// Register the first user (becomes admin) and return its session cookie.
async fn register_admin(app: &axum::Router) -> String {
    let req = common::add_test_socket_addr(
        Request::builder()
            .uri("/register")
            .method("POST")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::from(
                "username=admin1&password=password123&confirm_password=password123",
            ))
            .unwrap(),
    );
    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(
        resp.status().is_redirection() || resp.status().is_success(),
        "admin registration failed: {}",
        resp.status()
    );
    resp.headers()
        .get("set-cookie")
        .expect("register should set a session cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn test_stylesheets_carry_responsive_rules() {
    let app = common::create_test_app().await;

    let resp = get(&app, "/static/css/main.css", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let css = body_string(resp).await;
    assert!(
        css.contains(".table-scroll"),
        "main.css must define the table scroll wrapper"
    );
    assert!(
        css.contains("@media (max-width: 768px)"),
        "main.css must carry the small-screen media query (stacked nav)"
    );

    let resp = get(&app, "/static/css/manage.css", None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let css = body_string(resp).await;
    assert!(
        css.contains("@media (max-width: 768px)"),
        "manage.css must carry the small-screen media query (scrollable tabs)"
    );
}

#[tokio::test]
async fn test_tables_wrapped_for_horizontal_scroll() {
    let app = common::create_test_app().await;
    let cookie = register_admin(&app).await;

    for uri in ["/manage", "/help", "/admin"] {
        let resp = get(&app, uri, Some(&cookie)).await;
        assert_eq!(resp.status(), StatusCode::OK, "GET {}", uri);
        let html = body_string(resp).await;
        assert!(
            html.contains("class=\"table-scroll\""),
            "{} must wrap its table(s) in .table-scroll",
            uri
        );
    }
}
