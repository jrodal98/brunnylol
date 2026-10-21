// Personal API tokens: creation, lookup, revocation.
//
// Tokens are bearer credentials for the JSON API. Only a SHA-256 hash
// is stored; the plaintext (prefix "brn_") is returned to the caller
// exactly once, at creation.

use anyhow::Result;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiToken {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Hash a plaintext token for storage / lookup.
pub fn hash_token(plaintext: &str) -> String {
    let digest = Sha256::digest(plaintext.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Generate a new random token plaintext: "brn_" + 48 hex chars
/// (24 bytes from the OS CSPRNG via /dev/urandom).
pub fn generate_token() -> std::io::Result<String> {
    use std::io::Read;
    let mut bytes = [0u8; 24];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    Ok(format!("brn_{}", hex))
}

/// Create a token for a user; returns (row id, plaintext shown once).
pub async fn create_api_token(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
) -> Result<(i64, String)> {
    let plaintext =
        generate_token().map_err(|e| anyhow::anyhow!("failed to generate token: {}", e))?;
    let token_hash = hash_token(&plaintext);
    let result = sqlx::query("INSERT INTO api_tokens (user_id, name, token_hash) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(name)
        .bind(&token_hash)
        .execute(pool)
        .await?;
    Ok((result.last_insert_rowid(), plaintext))
}

/// List a user's tokens (never includes hashes or plaintext).
pub async fn list_api_tokens(pool: &SqlitePool, user_id: i64) -> Result<Vec<ApiToken>> {
    let tokens = sqlx::query_as::<_, ApiToken>(
        "SELECT id, user_id, name, CAST(created_at AS TEXT) AS created_at,
                CAST(last_used_at AS TEXT) AS last_used_at
         FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC, id DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(tokens)
}

/// Revoke one of a user's tokens. Returns true if a row was deleted.
pub async fn revoke_api_token(pool: &SqlitePool, user_id: i64, token_id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM api_tokens WHERE id = ? AND user_id = ?")
        .bind(token_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Resolve a plaintext bearer token to its owning user, stamping
/// last_used_at. Returns None for unknown/revoked tokens.
pub async fn user_for_token(pool: &SqlitePool, plaintext: &str) -> Result<Option<super::User>> {
    let token_hash = hash_token(plaintext);
    let row = sqlx::query("SELECT user_id FROM api_tokens WHERE token_hash = ?")
        .bind(&token_hash)
        .fetch_optional(pool)
        .await?;

    let Some(row) = row else { return Ok(None) };
    use sqlx::Row;
    let user_id: i64 = row.get("user_id");

    let _ =
        sqlx::query("UPDATE api_tokens SET last_used_at = CURRENT_TIMESTAMP WHERE token_hash = ?")
            .bind(&token_hash)
            .execute(pool)
            .await;

    let user = sqlx::query_as::<_, super::User>("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}
