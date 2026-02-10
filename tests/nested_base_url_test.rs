// Test for nested bookmark base URL navigation
use brunnylol::domain::Command;
use brunnylol::domain::template::TemplateParser;
use brunnylol::db::bookmarks::bookmark_to_command;
use brunnylol::db::{Bookmark, NestedBookmark};

#[test]
fn test_nested_bookmark_stores_base_url() {
    // Create a parent nested bookmark with base URL
    let parent = Bookmark {
        id: 1,
        scope: "global".to_string(),
        user_id: None,
        alias: "dev".to_string(),
        bookmark_type: "nested".to_string(),
        url: "https://example.com".to_string(),  // This should be stored!
        description: "Development shortcuts".to_string(),
        command_template: None,
        created_by: None,
        variable_metadata: None,
    };

    // Create a child bookmark
    let child = NestedBookmark {
        id: 2,
        parent_bookmark_id: 1,
        alias: "frontend".to_string(),
        url: "https://github.com".to_string(),
        description: "Frontend repo".to_string(),
        command_template: Some("{url}/{user}/{repo}".to_string()),
        display_order: 1,
        variable_metadata: None,
    };

    // Convert to Command
    let command = bookmark_to_command(&parent, vec![child]).unwrap();

    // Verify it's a Nested command with base_url
    match command {
        Command::Nested { base_url, .. } => {
            assert_eq!(base_url, "https://example.com");
        }
        _ => panic!("Expected Nested command"),
    }
}

#[test]
fn test_nested_bookmark_base_url_redirect() {
    // Test that accessing just the parent alias returns the base URL
    let empty_template = TemplateParser::parse("").unwrap();

    let child = Command::Variable {
        base_url: "https://github.com".to_string(),
        template: empty_template,
        description: "Child".to_string(),
        metadata: None,
    };

    let mut children = std::collections::HashMap::new();
    children.insert("frontend".to_string(), child);

    let nested = Command::Nested {
        base_url: "https://example.com".to_string(),
        children,
        description: "Development shortcuts".to_string(),
    };

    // Test base_url() method
    assert_eq!(nested.base_url(), "https://example.com");

    // Test get_redirect_url with empty query (just the alias, no args)
    let redirect = nested.get_redirect_url("");
    assert_eq!(redirect, "https://example.com");

    // Test get_redirect_url with whitespace query
    let redirect = nested.get_redirect_url("  ");
    assert_eq!(redirect, "https://example.com");
}

#[test]
fn test_nested_bookmark_child_redirect() {
    // Test that accessing a child still works
    let empty_template = TemplateParser::parse("").unwrap();

    let child = Command::Variable {
        base_url: "https://github.com".to_string(),
        template: empty_template,
        description: "Child".to_string(),
        metadata: None,
    };

    let mut children = std::collections::HashMap::new();
    children.insert("frontend".to_string(), child);

    let nested = Command::Nested {
        base_url: "https://example.com".to_string(),
        children,
        description: "Development shortcuts".to_string(),
    };

    // Test accessing a child command
    let redirect = nested.get_redirect_url("frontend");
    assert_eq!(redirect, "https://github.com");
}
