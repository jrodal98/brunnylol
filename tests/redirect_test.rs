// Test to verify redirect URLs are not URL-encoded

use brunnylol::db::{self, BookmarkScope};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_redirect_url_not_encoded() {
    let pool = common::setup_test_db().await;
    let (user_id, _) = common::create_admin_user(&pool).await;

    // Create a bookmark
    db::create_bookmark(
        &pool,
        BookmarkScope::Personal { user_id },
        "gh",
        "templated",
        "https://github.com",
        "GitHub",
        Some("{url}/{query|!encode}"),
        None,
    )
    .await
    .unwrap();

    // Create router
    let app = brunnylol::create_router().await;

    // Make request
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=gh+jrodal98/brunnylol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Check status
    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    // Check location header
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    eprintln!("DEBUG TEST: Location header = {}", location);

    // Location should NOT be URL-encoded
    assert!(
        location.contains("https://github.com/jrodal98/brunnylol"),
        "Location header should not be URL-encoded, got: {}",
        location
    );
}
