// Authentication module

pub mod middleware;

use anyhow::{Context, Result};
use bcrypt::{hash, verify, DEFAULT_COST};

// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String> {
    hash(password, DEFAULT_COST).context("Failed to hash password")
}

// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    verify(password, hash).context("Failed to verify password")
}

// Validate username format
pub fn validate_username(username: &str) -> Result<()> {
    if username.len() < 3 {
        anyhow::bail!("Username must be at least 3 characters");
    }
    if username.len() > 30 {
        anyhow::bail!("Username must be at most 30 characters");
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        anyhow::bail!("Username can only contain letters, numbers, underscores, and hyphens");
    }
    Ok(())
}

// Validate password strength
pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 8 {
        anyhow::bail!("Password must be at least 8 characters");
    }
    if password.len() > 128 {
        anyhow::bail!("Password must be at most 128 characters");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_username_validation() {
        assert!(validate_username("valid_user").is_ok());
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("ab").is_err()); // too short
        assert!(validate_username("user@example").is_err()); // invalid char
    }

    #[test]
    fn test_password_validation() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("short").is_err()); // too short
    }
}
