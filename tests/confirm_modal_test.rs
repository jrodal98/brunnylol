// The themed confirmation modal: pages must carry the modal
// markup and load confirm.js, which intercepts htmx's "htmx:confirm"
// event so hx-confirm guards (bookmark delete, token revoke, user
// delete, ...) present the site's own dialog instead of the
// browser's native confirm(). The end-to-end click behavior is
// covered by browser testing; this suite pins the served pieces.

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
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn test_pages_carry_confirm_modal() {
    let app = common::create_test_app().await;

    let resp = get(&app, "/").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(
        body.contains(r#"id="confirm-modal""#),
        "pages should include the confirmation modal markup"
    );
    assert!(
        body.contains(r#"id="confirm-modal-message""#),
        "modal should include a message element"
    );
    assert!(
        body.contains(r#"id="confirm-modal-ok""#) && body.contains(r#"id="confirm-modal-cancel""#),
        "modal should include confirm and cancel buttons"
    );
    assert!(
        body.contains(r#"src="/static/js/confirm.js""#),
        "pages should load confirm.js"
    );
}

#[tokio::test]
async fn test_confirm_js_and_styles_served() {
    let app = common::create_test_app().await;

    let resp = get(&app, "/static/js/confirm.js").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(
        body.contains("htmx:confirm"),
        "confirm.js should intercept the htmx:confirm event"
    );
    assert!(
        body.contains("issueRequest"),
        "confirm.js should issue the held request on confirm"
    );

    let resp = get(&app, "/static/css/main.css").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(
        body.contains(".modal-backdrop") && body.contains(".modal-card"),
        "main.css should style the confirmation modal"
    );
}
