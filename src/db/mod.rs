// Database module for user management and bookmarks

pub mod bookmarks;
pub mod seed;

use sqlx::{sqlite::SqlitePool, Row};
use anyhow::{Context, Result};

// Initialize database and run migrations
pub async fn init_db(db_path: &str) -> Result<SqlitePool> {
    let database_url = format!("sqlite:{}", db_path);

    // Create the database file if it doesn't exist
    if !std::path::Path::new(db_path).exists() {
        std::fs::File::create(db_path)?;
    }

    // Connect to the database
    let pool = SqlitePool::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    // Run migrations
    let migration_sql = include_str!("../../migrations/001_initial_schema.sql");
    sqlx::query(migration_sql)
        .execute(&pool)
        .await
        .context("Failed to run migrations")?;

    Ok(pool)
}

// User models
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
}

// Create a new user (first user becomes admin automatically)
pub async fn create_user(pool: &SqlitePool, username: &str, password_hash: &str) -> Result<User> {
    // Check if this is the first user
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    let is_admin = count == 0;

    // Insert user
    let result = sqlx::query(
        "INSERT INTO users (username, password_hash, is_admin) VALUES (?, ?, ?)"
    )
    .bind(username)
    .bind(password_hash)
    .bind(is_admin)
    .execute(pool)
    .await?;

    Ok(User {
        id: result.last_insert_rowid(),
        username: username.to_string(),
        is_admin,
    })
}

// Find user by username
pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<(i64, String, bool)>> {
    let result = sqlx::query(
        "SELECT id, password_hash, is_admin FROM users WHERE username = ? COLLATE NOCASE"
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|row| (
        row.get::<i64, _>("id"),
        row.get::<String, _>("password_hash"),
        row.get::<bool, _>("is_admin"),
    )))
}

// Get user by ID
pub async fn get_user_by_id(pool: &SqlitePool, user_id: i64) -> Result<Option<User>> {
    let result = sqlx::query(
        "SELECT id, username, is_admin FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|row| User {
        id: row.get("id"),
        username: row.get("username"),
        is_admin: row.get("is_admin"),
    }))
}

// List all users (admin only)
pub async fn list_all_users(pool: &SqlitePool) -> Result<Vec<User>> {
    let rows = sqlx::query(
        "SELECT id, username, is_admin FROM users ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| User {
        id: row.get("id"),
        username: row.get("username"),
        is_admin: row.get("is_admin"),
    }).collect())
}

// Session management
pub async fn create_session(pool: &SqlitePool, user_id: i64) -> Result<String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, ?)"
    )
    .bind(&session_id)
    .bind(user_id)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(session_id)
}

// Validate session and return user_id
pub async fn validate_session(pool: &SqlitePool, session_id: &str) -> Result<Option<i64>> {
    let result = sqlx::query(
        "SELECT user_id FROM sessions WHERE id = ? AND expires_at > datetime('now')"
    )
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|row| row.get("user_id")))
}

// Delete session (logout)
pub async fn delete_session(pool: &SqlitePool, session_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;

    Ok(())
}

// Delete all sessions for a user (for password changes)
pub async fn delete_all_user_sessions(pool: &SqlitePool, user_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

// Cleanup expired sessions
pub async fn cleanup_expired_sessions(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < datetime('now')")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

// Bookmark models
#[derive(Debug, Clone)]
pub struct UserBookmark {
    pub id: i64,
    pub user_id: i64,
    pub alias: String,
    pub bookmark_type: String,
    pub url: String,
    pub description: String,
    pub command_template: Option<String>,
    pub encode_query: bool,
}

#[derive(Debug, Clone)]
pub struct NestedBookmark {
    pub id: i64,
    pub parent_bookmark_id: i64,
    pub alias: String,
    pub url: String,
    pub description: String,
    pub command_template: Option<String>,
    pub encode_query: bool,
    pub display_order: i32,
}

// Create a new bookmark
pub async fn create_bookmark(
    pool: &SqlitePool,
    user_id: i64,
    alias: &str,
    bookmark_type: &str,
    url: &str,
    description: &str,
    command_template: Option<&str>,
    encode_query: bool,
) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO user_bookmarks
         (user_id, alias, bookmark_type, url, description, command_template, encode_query)
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(user_id)
    .bind(alias)
    .bind(bookmark_type)
    .bind(url)
    .bind(description)
    .bind(command_template)
    .bind(encode_query)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

// Get all bookmarks for a user
pub async fn get_user_bookmarks(pool: &SqlitePool, user_id: i64) -> Result<Vec<UserBookmark>> {
    let rows = sqlx::query(
        "SELECT id, user_id, alias, bookmark_type, url, description, command_template, encode_query
         FROM user_bookmarks
         WHERE user_id = ?
         ORDER BY alias"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| UserBookmark {
        id: row.get("id"),
        user_id: row.get("user_id"),
        alias: row.get("alias"),
        bookmark_type: row.get("bookmark_type"),
        url: row.get("url"),
        description: row.get("description"),
        command_template: row.get("command_template"),
        encode_query: row.get("encode_query"),
    }).collect())
}

// Get a single bookmark by ID
pub async fn get_bookmark_by_id(pool: &SqlitePool, bookmark_id: i64) -> Result<Option<UserBookmark>> {
    let row = sqlx::query(
        "SELECT id, user_id, alias, bookmark_type, url, description, command_template, encode_query
         FROM user_bookmarks
         WHERE id = ?"
    )
    .bind(bookmark_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| UserBookmark {
        id: row.get("id"),
        user_id: row.get("user_id"),
        alias: row.get("alias"),
        bookmark_type: row.get("bookmark_type"),
        url: row.get("url"),
        description: row.get("description"),
        command_template: row.get("command_template"),
        encode_query: row.get("encode_query"),
    }))
}

// Get nested bookmarks for a parent bookmark
pub async fn get_nested_bookmarks(pool: &SqlitePool, parent_bookmark_id: i64) -> Result<Vec<NestedBookmark>> {
    let rows = sqlx::query(
        "SELECT id, parent_bookmark_id, alias, url, description, command_template, encode_query, display_order
         FROM nested_bookmarks
         WHERE parent_bookmark_id = ?
         ORDER BY display_order, alias"
    )
    .bind(parent_bookmark_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| NestedBookmark {
        id: row.get("id"),
        parent_bookmark_id: row.get("parent_bookmark_id"),
        alias: row.get("alias"),
        url: row.get("url"),
        description: row.get("description"),
        command_template: row.get("command_template"),
        encode_query: row.get("encode_query"),
        display_order: row.get("display_order"),
    }).collect())
}

// Get a single nested bookmark by ID
pub async fn get_nested_bookmark_by_id(pool: &SqlitePool, nested_id: i64) -> Result<Option<NestedBookmark>> {
    let row = sqlx::query(
        "SELECT id, parent_bookmark_id, alias, url, description, command_template, encode_query, display_order
         FROM nested_bookmarks
         WHERE id = ?"
    )
    .bind(nested_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| NestedBookmark {
        id: row.get("id"),
        parent_bookmark_id: row.get("parent_bookmark_id"),
        alias: row.get("alias"),
        url: row.get("url"),
        description: row.get("description"),
        command_template: row.get("command_template"),
        encode_query: row.get("encode_query"),
        display_order: row.get("display_order"),
    }))
}

// Update a bookmark
pub async fn update_bookmark(
    pool: &SqlitePool,
    bookmark_id: i64,
    user_id: i64,
    alias: &str,
    url: &str,
    description: &str,
    command_template: Option<&str>,
    encode_query: bool,
) -> Result<()> {
    sqlx::query(
        "UPDATE user_bookmarks
         SET alias = ?, url = ?, description = ?, command_template = ?, encode_query = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND user_id = ?"
    )
    .bind(alias)
    .bind(url)
    .bind(description)
    .bind(command_template)
    .bind(encode_query)
    .bind(bookmark_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

// Delete a bookmark (and all its nested bookmarks)
pub async fn delete_bookmark(pool: &SqlitePool, bookmark_id: i64, user_id: i64) -> Result<()> {
    sqlx::query(
        "DELETE FROM user_bookmarks WHERE id = ? AND user_id = ?"
    )
    .bind(bookmark_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

// Create nested bookmark
pub async fn create_nested_bookmark(
    pool: &SqlitePool,
    parent_bookmark_id: i64,
    alias: &str,
    url: &str,
    description: &str,
    command_template: Option<&str>,
    encode_query: bool,
    display_order: i32,
) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO nested_bookmarks
         (parent_bookmark_id, alias, url, description, command_template, encode_query, display_order)
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(parent_bookmark_id)
    .bind(alias)
    .bind(url)
    .bind(description)
    .bind(command_template)
    .bind(encode_query)
    .bind(display_order)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

// Delete nested bookmark
pub async fn delete_nested_bookmark(pool: &SqlitePool, nested_id: i64) -> Result<()> {
    sqlx::query("DELETE FROM nested_bookmarks WHERE id = ?")
        .bind(nested_id)
        .execute(pool)
        .await?;

    Ok(())
}

// Get user overrides for built-in bookmarks
pub async fn get_user_overrides(pool: &SqlitePool, user_id: i64) -> Result<Vec<(String, bool, Option<String>, Option<String>)>> {
    let rows = sqlx::query(
        "SELECT builtin_alias, is_disabled, custom_alias, additional_aliases
         FROM user_bookmark_overrides
         WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| (
        row.get("builtin_alias"),
        row.get("is_disabled"),
        row.get("custom_alias"),
        row.get("additional_aliases"),
    )).collect())
}

// Create or update override for built-in bookmark
pub async fn upsert_override(
    pool: &SqlitePool,
    user_id: i64,
    builtin_alias: &str,
    is_disabled: bool,
    custom_alias: Option<&str>,
    additional_aliases: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO user_bookmark_overrides (user_id, builtin_alias, is_disabled, custom_alias, additional_aliases)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(user_id, builtin_alias)
         DO UPDATE SET is_disabled = ?, custom_alias = ?, additional_aliases = ?, updated_at = CURRENT_TIMESTAMP"
    )
    .bind(user_id)
    .bind(builtin_alias)
    .bind(is_disabled)
    .bind(custom_alias)
    .bind(additional_aliases)
    .bind(is_disabled)
    .bind(custom_alias)
    .bind(additional_aliases)
    .execute(pool)
    .await?;

    Ok(())
}

// Delete override (reset to built-in default)
pub async fn delete_override(pool: &SqlitePool, user_id: i64, builtin_alias: &str) -> Result<()> {
    sqlx::query(
        "DELETE FROM user_bookmark_overrides WHERE user_id = ? AND builtin_alias = ?"
    )
    .bind(user_id)
    .bind(builtin_alias)
    .execute(pool)
    .await?;

    Ok(())
}
