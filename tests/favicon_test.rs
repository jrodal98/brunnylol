// The site favicon (concept #24 from the design rounds: the search
// lens IS the bunny's head, solid, with the full violet face) must
// be served, referenced by the pages, and advertised in the
// OpenSearch descriptor so browser search-engine entries get the
// icon too.

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

async fn get(app: &axum::Router, uri: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn test_favicon_assets_served() {
    let app = common::create_test_app().await;

    let resp = get(&app, "/static/favicon.svg").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap().to_string();
    assert!(ct.contains("image/svg+xml"), "svg content type: {}", ct);
    let body = body_string(resp).await;
    assert!(body.contains("#764ba2"), "favicon should carry the brand violet face");
    assert!(body.contains("#667eea"), "favicon should carry the brand gradient");

    for path in ["/static/favicon-32.png", "/static/favicon-64.png", "/static/apple-touch-icon.png"] {
        let resp = get(&app, path).await;
        assert_eq!(resp.status(), StatusCode::OK, "{} should be served", path);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap().to_string();
        assert!(ct.contains("image/png"), "{} content type: {}", path, ct);
    }
}

#[tokio::test]
async fn test_favicon_referenced_by_pages_and_descriptor() {
    let app = common::create_test_app().await;

    // Pages (rendered from base.html) link the icon set.
    let resp = get(&app, "/").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains(r#"rel="icon" type="image/svg+xml" href="/static/favicon.svg""#), "home page should link the svg favicon");
    assert!(body.contains(r#"rel="apple-touch-icon""#), "home page should link the touch icon");

    // The OpenSearch descriptor advertises the icon for
    // browser search-engine entries.
    let resp = get(&app, "/opensearch.xml").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("<Image"), "descriptor should carry an Image element");
    assert!(body.contains("/static/favicon-64.png"), "descriptor Image should point at the 64px png");
}
