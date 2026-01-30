// Validation functions for brunnylol
// Provides reusable validation logic for forms and user input

use crate::error::AppError;
use url::Url;

/// Check if an IPv4 address is private or reserved
fn is_private_ipv4(octets: [u8; 4]) -> Option<&'static str> {
    // Localhost and loopback (127.0.0.0/8)
    if octets[0] == 127 {
        return Some("Cannot fetch from loopback address");
    }
    // 10.0.0.0/8
    if octets[0] == 10 {
        return Some("Cannot fetch from private IP range");
    }
    // 172.16.0.0/12
    if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
        return Some("Cannot fetch from private IP range");
    }
    // 192.168.0.0/16
    if octets[0] == 192 && octets[1] == 168 {
        return Some("Cannot fetch from private IP range");
    }
    // 169.254.0.0/16 (link-local)
    if octets[0] == 169 && octets[1] == 254 {
        return Some("Cannot fetch from link-local address");
    }
    None
}

/// Check if an IPv6 address is private or reserved
fn is_private_ipv6(ipv6: std::net::Ipv6Addr) -> Option<&'static str> {
    // Block IPv6 loopback
    if ipv6.is_loopback() {
        return Some("Cannot fetch from loopback address");
    }

    // Block IPv4-mapped IPv6 addresses (::ffff:x.x.x.x)
    if let Some(ipv4) = ipv6.to_ipv4_mapped() {
        return is_private_ipv4(ipv4.octets());
    }

    // Block IPv6 unique local addresses (fc00::/7)
    let segments = ipv6.segments();
    if segments[0] >= 0xfc00 && segments[0] <= 0xfdff {
        return Some("Cannot fetch from private IP range");
    }
    // Block IPv6 link-local addresses (fe80::/10)
    if segments[0] >= 0xfe80 && segments[0] <= 0xfebf {
        return Some("Cannot fetch from link-local address");
    }
    None
}

/// Validate that a template string contains the placeholder "{}"
///
/// Returns Ok(()) if valid, Err(AppError::BadRequest) if invalid
pub fn validate_template(template: &str) -> Result<(), AppError> {
    if !template.is_empty() && !template.contains("{}") {
        return Err(AppError::BadRequest(
            "Template must contain '{}' placeholder for the search query".to_string()
        ));
    }

    // Validate URL scheme (only allow http, https)
    if !template.is_empty() {
        validate_url_scheme(template)?;
    }

    Ok(())
}

/// Validate variable template syntax (RFC 6570-style)
///
/// Returns Ok(()) if valid, Err(AppError::BadRequest) if invalid
pub fn validate_variable_template(template: &str) -> Result<(), AppError> {
    // Parse template to validate syntax
    let parsed = crate::domain::template::TemplateParser::parse(template)
        .map_err(|e| AppError::BadRequest(format!("Invalid template syntax: {}", e)))?;

    // Validate URL scheme
    validate_url_scheme(template)?;

    // Disallow multiple {} or {query} placeholders
    let query_vars = parsed
        .variables()
        .iter()
        .filter(|v| v.name == "query")
        .count();

    if query_vars > 1 {
        return Err(AppError::BadRequest(
            "Template cannot have multiple {} or {query} placeholders".to_string(),
        ));
    }

    // Validate variable names (alphanumeric + underscore only)
    for var in parsed.variables() {
        if var.name != "query" && !is_valid_variable_name(&var.name) {
            return Err(AppError::BadRequest(format!(
                "Invalid variable name '{}': must contain only letters, numbers, and underscores",
                var.name
            )));
        }
    }

    Ok(())
}

/// Check if a variable name is valid (alphanumeric + underscore)
fn is_valid_variable_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Validate that a URL has a safe scheme (http or https only)
///
/// Returns Ok(()) if valid, Err(AppError::BadRequest) if invalid
pub fn validate_url_scheme(url: &str) -> Result<(), AppError> {
    let url_lower = url.to_lowercase();

    // Check if it starts with a scheme
    if url_lower.contains("://") || url_lower.starts_with("javascript:")
        || url_lower.starts_with("data:") || url_lower.starts_with("file:") {
        // Only allow http and https
        if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
            return Err(AppError::BadRequest(
                "Only http:// and https:// URLs are allowed".to_string()
            ));
        }
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

/// Validate URL for SSRF protection
///
/// Only allows http/https URLs and blocks private IP ranges
pub fn validate_url_for_fetch(url_str: &str) -> Result<(), AppError> {
    // Parse URL
    let url = url_str.parse::<Url>()
        .map_err(|_| AppError::BadRequest("Invalid URL format".to_string()))?;

    // Only allow http and https
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::BadRequest("Only http:// and https:// URLs are allowed".to_string()));
    }

    // Get host
    let host = url.host_str()
        .ok_or(AppError::BadRequest("URL must have a host".to_string()))?;

    // Block localhost variants
    if host == "localhost" || host == "127.0.0.1" || host.starts_with("127.")
        || host == "::1" || host == "0.0.0.0" {
        return Err(AppError::BadRequest("Cannot fetch from localhost".to_string()));
    }

    // Block private IP ranges (IPv4 and IPv6)
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(ipv4) => {
                if let Some(error_msg) = is_private_ipv4(ipv4.octets()) {
                    return Err(AppError::BadRequest(error_msg.to_string()));
                }
            }
            std::net::IpAddr::V6(ipv6) => {
                if let Some(error_msg) = is_private_ipv6(ipv6) {
                    return Err(AppError::BadRequest(error_msg.to_string()));
                }
            }
        }
    }

    Ok(())
}

/// Validate a resolved IP address for SSRF protection
///
/// This provides basic localhost and private IP blocking.
pub fn validate_resolved_ip(ip: std::net::IpAddr) -> Result<(), AppError> {
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            // Check for unspecified address (0.0.0.0)
            if ipv4.is_unspecified() {
                return Err(AppError::BadRequest("Cannot fetch from unspecified address".to_string()));
            }
            if let Some(error_msg) = is_private_ipv4(ipv4.octets()) {
                return Err(AppError::BadRequest(error_msg.to_string()));
            }
        }
        std::net::IpAddr::V6(ipv6) => {
            if let Some(error_msg) = is_private_ipv6(ipv6) {
                return Err(AppError::BadRequest(error_msg.to_string()));
            }
        }
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
