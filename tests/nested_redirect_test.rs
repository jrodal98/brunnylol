// Regression tests for two bugs reported against a personal nested
// bookmark ("bl" with children s/h/m/a, base URL "/"):
//   1. A bare nested alias failed outright — the parent's own URL
//      was discarded when the bookmark was loaded into a Command,
//      so a bare alias resolved to an empty redirect target.
//   2. The parent's Uses count was inflated — usage was recorded
//      before resolution, so failed attempts counted too.

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

fn req(
    method: &str,
    uri: &str,
    session: Option<&str>,
    bearer: Option<&str>,
    content_type: Option<&str>,
    body: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder().uri(uri).method(method);
    if let Some(c) = session {
        builder = builder.header("Cookie", c);
    }
    if let Some(t) = bearer {
        builder = builder.header("Authorization", format!("Bearer {}", t));
    }
    if let Some(ct) = content_type {
        builder = builder.header("Content-Type", ct);
    }
    common::add_test_socket_addr(
        builder
            .body(Body::from(body.unwrap_or("").to_string()))
            .unwrap(),
    )
}

fn form_req(method: &str, uri: &str, cookie: Option<&str>, form: &str) -> Request<Body> {
    req(
        method,
        uri,
        cookie,
        None,
        Some("application/x-www-form-urlencoded"),
        Some(form),
    )
}

async fn register_admin(app: &axum::Router) -> String {
    let resp = app
        .clone()
        .oneshot(form_req(
            "POST",
            "/register",
            None,
            "username=admin1&password=<redacted>&confirm_password=<redacted>",
        ))
        .await
        .unwrap();
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

/// Create an API token through the settings form; returns the
/// plaintext shown exactly once in the response fragment.
async fn create_api_token(app: &axum::Router, cookie: &str) -> String {
    let resp = app
        .clone()
        .oneshot(form_req(
            "POST",
            "/settings/api-tokens",
            Some(cookie),
            "name=test",
        ))
        .await
        .unwrap();
    assert!(resp.status().is_success(), "token create: {}", resp.status());
    let body = body_string(resp).await;
    let start = body.find("brn_").expect("token fragment shows plaintext");
    body[start..].chars().take_while(|c| *c != '<' && *c != '"' && !c.is_whitespace()).collect()
}

/// Build the fixture from the bug report: nested "bl" (base "/")
/// with children m -> /manage and s -> /search?q={query}.
async fn setup_bl(app: &axum::Router) -> (String, String) {
    let cookie = register_admin(app).await;
    let token = create_api_token(app, &cookie).await;
    let resp = app
        .clone()
        .oneshot(req(
            "POST",
            "/api/bookmarks",
            None,
            Some(&token),
            Some("application/json"),
            Some(
                r#"{"alias":"bl","url":"/","description":"brunnylol","nested":[
                    {"alias":"m","url":"/manage","description":"manage"},
                    {"alias":"s","url":"/search","description":"Brunnylol search","command":"/search?q={query}"}
                ]}"#,
            ),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "create bl failed");
    (cookie, token)
}

async fn search_location(app: &axum::Router, cookie: &str, q: &str) -> (StatusCode, Option<String>) {
    let uri = format!("/search?q={}", urlencoding::encode(q));
    let resp = app
        .clone()
        .oneshot(req("GET", &uri, Some(cookie), None, None, None))
        .await
        .unwrap();
    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .map(|v| v.to_str().unwrap().to_string());
    (status, location)
}

#[tokio::test]
async fn test_bare_nested_alias_redirects_to_parent_url() {
    let app = common::create_test_app().await;
    let (cookie, _) = setup_bl(&app).await;

    // Bare alias -> the parent's own URL (this used to produce an
    // empty Location and fail).
    let (status, location) = search_location(&app, &cookie, "bl").await;
    assert!(status.is_redirection(), "bare bl should redirect, got {}", status);
    assert_eq!(location.as_deref(), Some("/"), "bare bl should land on the parent URL");

    // Known subcommand still resolves to the child.
    let (status, location) = search_location(&app, &cookie, "bl m").await;
    assert!(status.is_redirection(), "bl m should redirect, got {}", status);
    assert_eq!(location.as_deref(), Some("/manage"));

    // Templated subcommand still expands its query.
    let (status, location) = search_location(&app, &cookie, "bl s hello world").await;
    assert!(status.is_redirection(), "bl s should redirect, got {}", status);
    assert_eq!(location.as_deref(), Some("/search?q=hello%20world"));

    // Unknown subcommand lands on the parent URL instead of failing.
    let (status, location) = search_location(&app, &cookie, "bl zzz").await;
    assert!(status.is_redirection(), "bl zzz should redirect, got {}", status);
    assert_eq!(location.as_deref(), Some("/"), "unknown child should fall back to the parent URL");
}

#[tokio::test]
async fn test_usage_counts_only_successful_resolutions() {
    let app = common::create_test_app().await;
    let (cookie, _) = setup_bl(&app).await;

    // Two successful uses of the bl family: bare + subcommand
    // (subcommand uses roll up to the parent).
    let _ = search_location(&app, &cookie, "bl").await;
    let _ = search_location(&app, &cookie, "bl m").await;

    // Failed resolutions must NOT count: an erroring nested mode
    // (Named mode is unsupported for nested commands) and a fully
    // unknown alias.
    let (status, _) = search_location(&app, &cookie, "bl$").await;
    assert!(!status.is_redirection(), "bl$ should not redirect, got {}", status);
    let _ = search_location(&app, &cookie, "zznope").await;

    // The count also must not move for the OpenSearch suggestions
    // endpoint — it only reads aliases.
    let resp = app
        .clone()
        .oneshot(req("GET", "/api/suggest?q=bl", Some(&cookie), None, None, None))
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // Manage page renders bl's Uses as exactly 2.
    let resp = app
        .oneshot(req("GET", "/manage", Some(&cookie), None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    assert!(
        body.contains("data-uses=\"2\""),
        "bl should show exactly 2 uses (successful resolutions only)"
    );
}
