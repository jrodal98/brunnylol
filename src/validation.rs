// Validation functions for brunnylol
// Provides reusable validation logic for forms and user input

use crate::error::AppError;

/// Validate that a template string contains the placeholder "{}"
///
/// Returns Ok(()) if valid, Err(AppError::BadRequest) if invalid
pub fn validate_template(template: &str) -> Result<(), AppError> {
    if !template.is_empty() && !template.contains("{}") {
        return Err(AppError::BadRequest(
            "Template must contain '{}' placeholder for the search query".to_string()
        ));
    }
    Ok(())
}

/// Validate that password and confirm_password match
///
/// Returns Ok(()) if they match, Err(AppError::BadRequest) if they don't
pub fn validate_passwords_match(password: &str, confirm_password: &str) -> Result<(), AppError> {
    if password != confirm_password {
        return Err(AppError::BadRequest("Passwords do not match".to_string()));
    }
    Ok(())
}

/// Validate that a string is not empty
///
/// Returns Ok(()) if non-empty, Err(AppError::BadRequest) if empty
pub fn validate_not_empty(field_name: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::BadRequest(format!("{} cannot be empty", field_name)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_template_valid() {
        assert!(validate_template("https://example.com/search?q={}").is_ok());
        assert!(validate_template("").is_ok()); // Empty is allowed
    }

    #[test]
    fn test_validate_template_invalid() {
        let result = validate_template("https://example.com/search");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must contain '{}'"));
    }

    #[test]
    fn test_validate_passwords_match_valid() {
        assert!(validate_passwords_match("password123", "password123").is_ok());
    }

    #[test]
    fn test_validate_passwords_match_invalid() {
        let result = validate_passwords_match("password123", "password456");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("do not match"));
    }

    #[test]
    fn test_validate_not_empty_valid() {
        assert!(validate_not_empty("Username", "john").is_ok());
    }

    #[test]
    fn test_validate_not_empty_invalid() {
        let result = validate_not_empty("Username", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));

        let result = validate_not_empty("Username", "   ");
        assert!(result.is_err());
    }
}
