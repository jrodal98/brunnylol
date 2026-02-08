// Negative path tests for validation errors
// Tests that ensure proper error handling for invalid inputs

use brunnylol::validation;

#[tokio::test]
async fn test_invalid_template_without_placeholder() {
    let result = validation::validate_template("https://example.com/search");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("placeholder"));
}

#[tokio::test]
async fn test_invalid_template_with_text() {
    let result = validation::validate_template("just some text without placeholder");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_password_mismatch() {
    let result = validation::validate_passwords_match("password123", "different456");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("do not match"));
}

#[tokio::test]
async fn test_password_mismatch_case_sensitive() {
    let result = validation::validate_passwords_match("Password", "password");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_string_rejected() {
    let result = validation::validate_not_empty("Username", "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
}

#[tokio::test]
async fn test_whitespace_only_rejected() {
    let result = validation::validate_not_empty("Field", "   ");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tab_only_rejected() {
    let result = validation::validate_not_empty("Field", "\t\t");
    assert!(result.is_err());
}

// Positive test cases

#[tokio::test]
async fn test_valid_template_with_placeholder() {
    let result = validation::validate_template("https://example.com/search?q={}");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_empty_template_accepted() {
    // Empty template is accepted (for simple bookmarks)
    let result = validation::validate_template("");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_matching_passwords() {
    let result = validation::validate_passwords_match("password123", "password123");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_matching_empty_passwords() {
    let result = validation::validate_passwords_match("", "");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_non_empty_string() {
    let result = validation::validate_not_empty("Username", "john_doe");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_string_with_spaces() {
    let result = validation::validate_not_empty("Name", "John Doe");
    assert!(result.is_ok());
}
