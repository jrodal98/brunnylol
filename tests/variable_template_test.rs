// Integration tests for variable template functionality

use brunnylol::db::{self, BookmarkScope};
use std::collections::HashMap;

mod common;

#[tokio::test]
async fn test_create_variable_template_bookmark() {
    let pool = common::setup_test_db().await;
    let (user_id, _) = common::create_admin_user(&pool).await;

    // Create a bookmark with variable template
    let result = db::create_bookmark(
        &pool,
        BookmarkScope::Personal { user_id },
        "test",
        "templated",
        "https://example.com",
        "Test variable template",
        Some("https://example.com/{page}/{repo?}"),
        true,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_load_variable_template_as_command() {
    let pool = common::setup_test_db().await;
    let (user_id, _) = common::create_admin_user(&pool).await;

    // Create bookmark with variable syntax
    db::create_bookmark(
        &pool,
        BookmarkScope::Personal { user_id },
        "gh",
        "templated",
        "https://github.com",
        "GitHub with variables",
        Some("https://github.com/{org}/{repo}"),
        true,
        None,
    )
    .await
    .unwrap();

    // Load bookmarks
    let commands = brunnylol::db::bookmarks::load_user_bookmarks(&pool, user_id)
        .await
        .unwrap();

    // Should create Command::Variable (not Templated) due to multiple variables
    assert!(commands.contains_key("gh"));
}

#[tokio::test]
async fn test_variable_resolution_with_query() {
    use brunnylol::domain::template::{TemplateParser, TemplateResolver};

    let template = TemplateParser::parse("https://example.com/search?q={query}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "rust templates".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://example.com/search?q=rust%20templates");
}

#[tokio::test]
async fn test_variable_resolution_with_multiple_vars() {
    use brunnylol::domain::template::{TemplateParser, TemplateResolver};

    let template = TemplateParser::parse("https://github.com/{org}/{repo}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("org".to_string(), "rust-lang".to_string());
    vars.insert("repo".to_string(), "rust".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://github.com/rust-lang/rust");
}

#[tokio::test]
async fn test_optional_variable_omission() {
    use brunnylol::domain::template::{TemplateParser, TemplateResolver};

    let template = TemplateParser::parse("/api/{version?}/search").unwrap();
    let resolver = TemplateResolver::new();

    let vars = HashMap::new(); // No version provided

    let result = resolver.resolve(&template, &vars).unwrap();
    // Optional variable omitted leaves empty string (TODO: smart segment omission)
    assert_eq!(result, "/api//search");
}

#[tokio::test]
async fn test_default_value_used() {
    use brunnylol::domain::template::{TemplateParser, TemplateResolver};

    let template = TemplateParser::parse("/api/{version=v1}/search").unwrap();
    let resolver = TemplateResolver::new();

    let vars = HashMap::new(); // No version provided

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "/api/v1/search");
}

#[tokio::test]
async fn test_pipeline_encode() {
    use brunnylol::domain::template::{TemplateParser, TemplateResolver};

    let template = TemplateParser::parse("/search?q={query|encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "hello world".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "/search?q=hello%20world");
}

#[tokio::test]
async fn test_pipeline_noencode() {
    use brunnylol::domain::template::{TemplateParser, TemplateResolver};

    let template = TemplateParser::parse("/path/{segment|!encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("segment".to_string(), "foo/bar".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "/path/foo/bar");
}

#[tokio::test]
async fn test_pipeline_trim_and_encode() {
    use brunnylol::domain::template::{TemplateParser, TemplateResolver};

    let template = TemplateParser::parse("/search?q={query|trim|encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "  hello world  ".to_string());

    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "/search?q=hello%20world");
}

#[tokio::test]
async fn test_backward_compatibility_with_query() {
    let pool = common::setup_test_db().await;
    let (user_id, _) = common::create_admin_user(&pool).await;

    // Create bookmark with simple {query} template (migrated from {})
    db::create_bookmark(
        &pool,
        BookmarkScope::Personal { user_id },
        "search",
        "templated",
        "https://example.com",
        "Simple search",
        Some("https://example.com/search?q={query}"),
        true,
        None,
    )
    .await
    .unwrap();

    // Should still work with existing Templated variant
    let commands = brunnylol::db::bookmarks::load_user_bookmarks(&pool, user_id)
        .await
        .unwrap();

    assert!(commands.contains_key("search"));
}
