// Integration tests for the Oct 2026 feature batch:
// OpenSearch descriptor + alias suggestions, bookmark usage counts,
// API tokens + the JSON bookmarks API, and the quick-add bookmarklet.

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
    cookie: Option<&str>,
    bearer: Option<&str>,
    content_type: Option<&str>,
    body: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder().uri(uri).method(method);
    if let Some(c) = cookie {
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

fn json_req(method: &str, uri: &str, bearer: &str, json: &str) -> Request<Body> {
    req(method, uri, None, Some(bearer), Some("application/json"), Some(json))
}

/// Register the first user (becomes admin) and return its session cookie.
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
async fn create_token(app: &axum::Router, cookie: &str) -> String {
    let resp = app
        .clone()
        .oneshot(form_req("POST", "/settings/api-tokens", Some(cookie), "name=test+agent"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let start = body.find("brn_").expect("token plaintext shown once");
    body[start..start + 52].to_string()
}

#[tokio::test]
async fn test_api_docs_endpoint() {
    let app = common::create_test_app().await;
    let resp = app
        .oneshot(req("GET", "/api", None, None, None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.contains("text/markdown"), "ct: {}", ct);
    let body = body_string(resp).await;
    assert!(body.contains("Authorization: Bearer"), "docs must cover auth");
    assert!(body.contains("/api/bookmarks/{alias}"), "docs must cover endpoints");
    assert!(body.contains("/api/whoami"), "docs must cover whoami");
}

#[tokio::test]
async fn test_opensearch_descriptor() {
    let app = common::create_test_app().await;
    let resp = app
        .clone()
        .oneshot(req("GET", "/opensearch.xml", None, None, None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.contains("application/opensearchdescription+xml"), "ct: {}", ct);
    let body = body_string(resp).await;
    assert!(body.contains("/search?q={searchTerms}"), "search template missing");
    assert!(body.contains("/api/suggest?q={searchTerms}"), "suggest template missing");

    // Pages advertise the descriptor for browser auto-discovery.
    let resp = app
        .oneshot(req("GET", "/", None, None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    assert!(body.contains("rel=\"search\""), "discovery link missing");
}

#[tokio::test]
async fn test_suggest_anonymous_returns_global_aliases() {
    let app = common::create_test_app().await;
    let resp = app
        .oneshot(req("GET", "/api/suggest?q=g", None, None, None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let arr: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(arr[0], "g");
    let completions = arr[1].as_array().unwrap();
    assert!(
        completions.iter().any(|c| c == "g"),
        "expected alias g in {:?}",
        completions
    );
    assert_eq!(arr[2].as_array().unwrap().len(), completions.len());
    assert_eq!(arr[3].as_array().unwrap().len(), completions.len());
}

#[tokio::test]
async fn test_suggest_personal_and_usage_counts() {
    let app = common::create_test_app().await;
    let cookie = register_admin(&app).await;

    // Create a personal bookmark through the web form.
    let resp = app
        .clone()
        .oneshot(form_req(
            "POST",
            "/manage/bookmark",
            Some(&cookie),
            "alias=zzsuggest&bookmark_type=simple&url=https://example.com&description=Suggest+Me",
        ))
        .await
        .unwrap();
    assert!(
        resp.status().is_success() || resp.status().is_redirection(),
        "create bookmark failed: {}",
        resp.status()
    );

    // Suggestions include the personal alias for the signed-in user.
    let resp = app
        .clone()
        .oneshot(req("GET", "/api/suggest?q=zzs", Some(&cookie), None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    let arr: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(
        arr[1].as_array().unwrap().iter().any(|c| c == "zzsuggest"),
        "personal alias missing from {:?}",
        arr[1]
    );

    // Anonymous suggestions do NOT include it.
    let resp = app
        .clone()
        .oneshot(req("GET", "/api/suggest?q=zzs", None, None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    let arr: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(
        !arr[1].as_array().unwrap().iter().any(|c| c == "zzsuggest"),
        "personal alias leaked to anonymous suggest"
    );

    // Two searches -> use_count 2 in the manage table.
    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(req("GET", "/search?q=zzsuggest", Some(&cookie), None, None, None))
            .await
            .unwrap();
        assert!(resp.status().is_redirection(), "search should redirect");
    }
    let resp = app
        .oneshot(req("GET", "/manage", Some(&cookie), None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    assert!(body.contains("data-uses=\"2\""), "usage count not rendered");
    assert!(body.contains("data-sort-key=\"uses\""), "sortable Uses header missing");
}

#[tokio::test]
async fn test_api_token_lifecycle_and_bookmark_crud() {
    let app = common::create_test_app().await;
    let cookie = register_admin(&app).await;
    let token = create_token(&app, &cookie).await;
    assert!(token.starts_with("brn_"));

    // whoami with and without the token
    let resp = app
        .clone()
        .oneshot(req("GET", "/api/whoami", None, Some(&token), None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("\"admin1\""), "whoami body: {}", body);

    let resp = app
        .clone()
        .oneshot(req("GET", "/api/whoami", None, None, None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Create + duplicate + get + update + delete
    let payload = "{\"alias\":\"api1\",\"url\":\"https://example.com\",\"description\":\"API One\"}";
    let resp = app
        .clone()
        .oneshot(json_req("POST", "/api/bookmarks", &token, payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(json_req("POST", "/api/bookmarks", &token, payload))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    let resp = app
        .clone()
        .oneshot(req("GET", "/api/bookmarks/api1", None, Some(&token), None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("\"use_count\":0"), "body: {}", body);

    let resp = app
        .clone()
        .oneshot(json_req(
            "PUT",
            "/api/bookmarks/api1",
            &token,
            "{\"url\":\"https://example.com/2\",\"description\":\"API Updated\"}",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("API Updated"));

    let resp = app
        .clone()
        .oneshot(req("DELETE", "/api/bookmarks/api1", None, Some(&token), None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(req("GET", "/api/bookmarks/api1", None, Some(&token), None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Revoke -> token stops working
    let resp = app
        .clone()
        .oneshot(req("GET", "/settings", Some(&cookie), None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    let marker = "/settings/api-tokens/";
    let start = body.find(marker).expect("revoke URL in settings") + marker.len();
    let rest = &body[start..];
    let token_id: i64 = rest[..rest.find('/').unwrap()].parse().unwrap();

    let resp = app
        .clone()
        .oneshot(form_req(
            "POST",
            &format!("/settings/api-tokens/{}/revoke", token_id),
            Some(&cookie),
            "",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(req("GET", "/api/whoami", None, Some(&token), None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_bookmarklet_prefill_and_settings_sections() {
    let app = common::create_test_app().await;
    let cookie = register_admin(&app).await;

    let resp = app
        .clone()
        .oneshot(req(
            "GET",
            "/manage?prefill_url=https%3A%2F%2Fwww.example.com%2Fpage&prefill_title=An+Example",
            Some(&cookie),
            None,
            None,
            None,
        ))
        .await
        .unwrap();
    let body = body_string(resp).await;
    assert!(body.contains("value=\"https://www.example.com/page\""), "prefill url missing");
    assert!(body.contains("value=\"An Example\""), "prefill title missing");
    assert!(body.contains("value=\"example\""), "suggested alias missing");

    let resp = app
        .oneshot(req("GET", "/settings", Some(&cookie), None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    assert!(body.contains("javascript:(function(){"), "bookmarklet missing");
    assert!(body.contains("window.open("), "bookmarklet must open a tab");
    assert!(body.contains("opensearch"), "bookmarklet must probe for OpenSearch");
    assert!(body.contains("prefill_url"), "bookmarklet must carry prefill params");
    assert!(body.contains("API Tokens"), "token section missing");
}

#[tokio::test]
async fn test_global_usage_counts_and_opensearch_prefill() {
    let app = common::create_test_app().await;
    let cookie = register_admin(&app).await;

    // Two ANONYMOUS searches through the built-in "g" alias count
    // against the global bookmark (instance-wide stats).
    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(req("GET", "/search?q=g", None, None, None, None))
            .await
            .unwrap();
        assert!(resp.status().is_redirection(), "search should redirect");
    }

    let resp = app
        .clone()
        .oneshot(req("GET", "/manage", Some(&cookie), None, None, None))
        .await
        .unwrap();
    let body = body_string(resp).await;
    let start = body.find("id=\"global-g\"").expect("global g row present");
    let row = &body[start..usize::min(start + 2500, body.len())];
    assert!(row.contains(">2</span>"), "global use count 2 not shown: {}", &row[..200]);

    // OpenSearch-derived prefill: command lands in the template field.
    let resp = app
        .oneshot(req(
            "GET",
            "/manage?prefill_url=https%3A%2F%2Fexample.com%2Fsearch&prefill_title=Ex&prefill_command=%7Burl%7D%3Fq%3D%7Bquery%7D",
            Some(&cookie),
            None,
            None,
            None,
        ))
        .await
        .unwrap();
    let body = body_string(resp).await;
    assert!(
        body.contains("value=\"{url}?q={query}\""),
        "prefill_command not in the template field"
    );
}
