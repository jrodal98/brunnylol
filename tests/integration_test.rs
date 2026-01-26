// Integration tests for brunnylol with Axum
// These tests verify the migrated Axum implementation

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt; // for `oneshot`

async fn create_test_app() -> axum::Router {
    // Temporarily override CLI args for testing
    // For now, use the default commands.yml
    brunnylol::create_router().await
}

#[tokio::test]
async fn test_index_page_renders() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Brunnylol"));
    assert!(body_str.contains("Smart Bookmarking"));
}

#[tokio::test]
async fn test_help_page_renders() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/help").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Brunnylol"));
    assert!(body_str.contains("Alias"));
    assert!(body_str.contains("Description"));
}

#[tokio::test]
async fn test_help_page_contains_google_alias() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/help").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should contain the google alias from commands.yml
    assert!(body_str.contains(">g<") || body_str.contains("google"));
}

#[tokio::test]
async fn test_redirect_with_valid_alias() {
    let app = create_test_app().await;

    // Test Google search: "g hello world"
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=g%20hello%20world")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();

    // Should redirect to Google with encoded query
    assert!(location.contains("google.com"));
    assert!(location.contains("hello%20world"));
}

#[tokio::test]
async fn test_redirect_url_encoding_spaces() {
    let app = create_test_app().await;

    // Test that spaces are encoded as %20
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=g%20rust%20programming")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let location = response.headers().get("Location").unwrap().to_str().unwrap();
    assert!(location.contains("rust%20programming"));
}

#[tokio::test]
async fn test_redirect_with_alias_only() {
    let app = create_test_app().await;

    // Test just alias without query (should go to base URL)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=g")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();

    // Should redirect to Google homepage (not search template)
    assert!(location.contains("google.com"));
    assert!(!location.contains("search?q="));
}

#[tokio::test]
async fn test_redirect_default_fallback() {
    let app = create_test_app().await;

    // Test unknown alias returns 404 (new default behavior - no fallback)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=unknownalias%20hello%20world")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 404 for unknown alias (new behavior)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_redirect_custom_default() {
    let app = create_test_app().await;

    // Test custom default parameter
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=unknownalias%20test&default=g")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();

    // Should use google as default
    assert!(location.contains("google.com"));
}

#[tokio::test]
async fn test_redirect_no_encoding_github() {
    let app = create_test_app().await;

    // Test GitHub alias with encode=false (forward slash should NOT be encoded)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=gh%20jrodal98/brunnylol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();

    // Forward slash should be preserved (not %2F)
    assert!(location.contains("github.com"));
    assert!(location.contains("jrodal98/brunnylol"));
    assert!(!location.contains("jrodal98%2Fbrunnylol"));
}

// There is no nested command in commands.yml right now
// #[tokio::test]
// async fn test_redirect_nested_command() {
//     let app = create_test_app().await;
//
//     // Test nested command structure (if aoc alias exists in commands.yml)
//     let response = app
//         .oneshot(
//             Request::builder()
//                 .uri("/search?q=aoc%20j%205")
//                 .body(Body::empty())
//                 .unwrap(),
//         )
//         .await
//         .unwrap();
//
//     assert_eq!(response.status(), StatusCode::SEE_OTHER);
//
//     let location = response.headers().get("Location").unwrap().to_str().unwrap();
//
//     // Should route through nested command structure
//     assert!(location.contains("github.com") || location.contains("advent-of-code"));
// }

#[tokio::test]
async fn test_redirect_special_characters() {
    let app = create_test_app().await;

    // Test special characters in query
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=g%20c%2B%2B%20tutorial")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();

    // Plus signs should be encoded
    assert!(location.contains("c%2B%2B"));
}

#[tokio::test]
async fn test_redirect_ampersand_encoding() {
    let app = create_test_app().await;

    // Test ampersand encoding
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=g%20rock%20%26%20roll")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();

    // Ampersand should be encoded
    assert!(location.contains("rock%20%26%20roll") || location.contains("rock+%26+roll"));
}

#[tokio::test]
async fn test_redirect_multiple_spaces() {
    let app = create_test_app().await;

    // Test multiple consecutive spaces
    let response = app
        .oneshot(
            Request::builder()
                .uri("/search?q=g%20hello%20%20%20world")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);

    let location = response.headers().get("Location").unwrap().to_str().unwrap();

    // Multiple spaces should be preserved and encoded
    assert!(location.contains("google.com"));
}

#[tokio::test]
async fn test_index_contains_help_link() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should contain link to help page
    assert!(body_str.contains("help") || body_str.contains("/help"));
}

#[tokio::test]
async fn test_help_page_searchable() {
    let app = create_test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/help").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should contain search input for filtering aliases
    assert!(body_str.contains("input") || body_str.contains("search"));
}
