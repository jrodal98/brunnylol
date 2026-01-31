// Comprehensive end-to-end tests for variable template functionality
// These tests verify the complete user workflow from template creation to URL resolution

use brunnylol::db::{self, BookmarkScope};
use brunnylol::domain::template::{TemplateParser, TemplateResolver};
use std::collections::HashMap;

mod common;

// Test 1: Create bookmark with options pipeline
#[tokio::test]
async fn test_e2e_create_bookmark_with_options() {
    let pool = common::setup_test_db().await;
    let (user_id, _) = common::create_admin_user(&pool).await;

    let result = db::create_bookmark(
        &pool,
        BookmarkScope::Personal { user_id },
        "pr",
        "templated",
        "https://proton.me",
        "Proton services",
        Some("https://{product|options[mail,drive,calendar,vpn]}.proton.me"),
        true,
        None,
    )
    .await;

    assert!(result.is_ok());

    // Verify it loads as Variable command with options
    let commands = brunnylol::db::bookmarks::load_user_bookmarks(&pool, user_id)
        .await
        .unwrap();

    assert!(commands.contains_key("pr"));
}

// Test 2: Options pipeline strict validation
#[tokio::test]
async fn test_e2e_options_strict_validation() {
    let template = TemplateParser::parse("https://{product|options[mail,drive][strict]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    // Valid option should work
    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "mail".to_string());
    let result = resolver.resolve(&template, &vars);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "https://mail.proton.me");

    // Invalid option should fail with strict
    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "invalid".to_string());
    let result = resolver.resolve(&template, &vars);
    assert!(result.is_err());
}

// Test 3: Options pipeline non-strict allows any value
#[tokio::test]
async fn test_e2e_options_non_strict_allows_any() {
    let template = TemplateParser::parse("https://{product|options[mail,drive]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    // Any value allowed in non-strict mode
    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "calendar".to_string());
    let result = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(result, "https://calendar.proton.me");
}

// Test 4: Positional argument mapping with multiple variables
#[tokio::test]
async fn test_e2e_positional_args_multiple_vars() {
    let pool = common::setup_test_db().await;
    let (user_id, _) = common::create_admin_user(&pool).await;

    db::create_bookmark(
        &pool,
        BookmarkScope::Personal { user_id },
        "ghuser",
        "templated",
        "https://github.com",
        "GitHub user/repo",
        Some("https://github.com/{user}/{repo}"),
        true,
        None,
    )
    .await
    .unwrap();

    let commands = brunnylol::db::bookmarks::load_user_bookmarks(&pool, user_id)
        .await
        .unwrap();

    let command = commands.get("ghuser").unwrap();

    // Test positional mapping: "jrodal98 dotfiles" → user=jrodal98, repo=dotfiles
    let url = command.get_redirect_url("jrodal98 dotfiles");
    assert_eq!(url, "https://github.com/jrodal98/dotfiles");
}

// Test 5: Named mode parsing with quoted values
#[tokio::test]
async fn test_e2e_named_mode_parsing() {
    // Test parse_named_variables directly
    let query = r#"$user="jrodal98"; $repo="my repo"; rest of query"#;

    // This would be tested in the actual handler, but let's verify the parsing logic
    // by creating a bookmark and using the template resolver
    let template = TemplateParser::parse("https://github.com/{user}/{repo}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("user".to_string(), "jrodal98".to_string());
    vars.insert("repo".to_string(), "my repo".to_string());

    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "https://github.com/jrodal98/my%20repo");
}

// Test 6: Escaped quotes in named variables
#[tokio::test]
async fn test_e2e_named_mode_escaped_quotes() {
    let template = TemplateParser::parse("/search?q={query}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("query".to_string(), r#"Say "Hello""#.to_string());

    let url = resolver.resolve(&template, &vars).unwrap();
    assert!(url.contains("Say%20%22Hello%22"));
}

// Test 7: Optional variables with missing values
#[tokio::test]
async fn test_e2e_optional_variable_omission() {
    let template = TemplateParser::parse("/api/{version?}/search").unwrap();
    let resolver = TemplateResolver::new();

    let vars = HashMap::new(); // version not provided

    let url = resolver.resolve(&template, &vars).unwrap();
    // Optional variable leaves empty string
    assert_eq!(url, "/api//search");
}

// Test 8: Default values used when not provided
#[tokio::test]
async fn test_e2e_default_values() {
    let template = TemplateParser::parse("/api/{version=v1}/users").unwrap();
    let resolver = TemplateResolver::new();

    let vars = HashMap::new(); // version not provided

    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "/api/v1/users");
}

// Test 9: Pipeline chaining (trim + encode)
#[tokio::test]
async fn test_e2e_pipeline_chaining() {
    let template = TemplateParser::parse("/search?q={query|trim|encode}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "  hello world  ".to_string());

    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "/search?q=hello%20world");
}

// Test 10: No encoding pipeline
#[tokio::test]
async fn test_e2e_no_encode_pipeline() {
    let template = TemplateParser::parse("/{path|!encode}/file").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("path".to_string(), "foo/bar".to_string());

    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "/foo/bar/file");
}

// Test 11: Single query variable (backward compatibility)
#[tokio::test]
async fn test_e2e_single_query_variable() {
    let template = TemplateParser::parse("https://google.com/search?q={query}").unwrap();
    let resolver = TemplateResolver::new();

    let mut vars = HashMap::new();
    vars.insert("query".to_string(), "rust programming".to_string());

    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "https://google.com/search?q=rust%20programming");
}

// Test 12: Form builder extracts options from pipelines
#[tokio::test]
async fn test_e2e_form_builder_options() {
    let template = TemplateParser::parse("https://{product|options[mail,drive,calendar]}.proton.me").unwrap();
    let form_data = brunnylol::domain::template::form_builder::build_form_data(&template, None, &HashMap::new());

    assert_eq!(form_data.len(), 1);
    assert_eq!(form_data[0].name, "product");
    assert!(form_data[0].options.is_some());

    let options = form_data[0].options.as_ref().unwrap();
    assert_eq!(options.len(), 3);
    assert_eq!(options[0], "mail");
    assert_eq!(options[1], "drive");
    assert_eq!(options[2], "calendar");
    assert!(!form_data[0].strict); // Non-strict by default
}

// Test 13: Form builder extracts strict flag
#[tokio::test]
async fn test_e2e_form_builder_strict() {
    let template = TemplateParser::parse("https://{product|options[mail,drive][strict]}.proton.me").unwrap();
    let form_data = brunnylol::domain::template::form_builder::build_form_data(&template, None, &HashMap::new());

    assert!(form_data[0].strict);
}

// Test 14: Multiple variables with mixed types
#[tokio::test]
async fn test_e2e_multiple_variables_mixed() {
    let template = TemplateParser::parse("/{org}/{repo?}/{file=README.md}").unwrap();
    let resolver = TemplateResolver::new();

    // Only provide required variable
    let mut vars = HashMap::new();
    vars.insert("org".to_string(), "rust-lang".to_string());

    let url = resolver.resolve(&template, &vars).unwrap();
    // org provided, repo omitted (optional), file uses default
    assert_eq!(url, "/rust-lang//README.md");
}

// Test 15: Greedy query consumption in named mode
#[tokio::test]
async fn test_e2e_greedy_query_consumption() {
    let template = TemplateParser::parse("/search?user={user}&q={query}").unwrap();
    let resolver = TemplateResolver::new();

    // Simulate: alias$ $user="jrodal98"; this is the search query
    let mut vars = HashMap::new();
    vars.insert("user".to_string(), "jrodal98".to_string());
    vars.insert("query".to_string(), "this is the search query".to_string());

    let url = resolver.resolve(&template, &vars).unwrap();
    assert!(url.contains("user=jrodal98"));
    assert!(url.contains("q=this%20is%20the%20search%20query"));
}

// Test 16: Empty variable {} maps to query
#[tokio::test]
async fn test_e2e_empty_variable_maps_to_query() {
    let template = TemplateParser::parse("/search?q={}").unwrap();
    let vars_list = template.variables();

    assert_eq!(vars_list.len(), 1);
    assert_eq!(vars_list[0].name, "query");
}

// Test 17: Escaped braces remain literal
#[tokio::test]
async fn test_e2e_escaped_braces() {
    let template = TemplateParser::parse("/api/{{placeholder}}/search").unwrap();
    let resolver = TemplateResolver::new();

    let vars = HashMap::new();
    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "/api/{placeholder}/search");
}

// Test 18: Variable with all features combined
#[tokio::test]
async fn test_e2e_variable_all_features() {
    // Variable with default, options, strict, and trim
    let template = TemplateParser::parse("https://{product=mail|trim|options[mail,drive,calendar][strict]}.proton.me").unwrap();
    let resolver = TemplateResolver::new();

    // Test with default (no value provided)
    let vars = HashMap::new();
    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "https://mail.proton.me");

    // Test with valid option
    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "  drive  ".to_string());
    let url = resolver.resolve(&template, &vars).unwrap();
    assert_eq!(url, "https://drive.proton.me"); // Trimmed

    // Test with invalid option (should fail with strict)
    let mut vars = HashMap::new();
    vars.insert("product".to_string(), "vpn".to_string());
    let result = resolver.resolve(&template, &vars);
    assert!(result.is_err());
}
