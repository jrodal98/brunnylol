// Database module for user management and bookmarks

pub mod bookmarks;
pub mod seed;

use sqlx::{sqlite::SqlitePool, Row};
use anyhow::{Context, Result};
use std::collections::HashMap;

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

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .context("Failed to enable foreign keys")?;

    // Run migrations
    let migration_sql = include_str!("../../migrations/001_initial_schema.sql");
    sqlx::query(migration_sql)
        .execute(&pool)
        .await
        .context("Failed to run migrations")?;

    let migration_sql_2 = include_str!("../../migrations/002_global_bookmarks.sql");
    sqlx::query(migration_sql_2)
        .execute(&pool)
        .await
        .context("Failed to run migration 002")?;

    let migration_sql_3 = include_str!("../../migrations/003_user_default_alias.sql");
    // Ignore "duplicate column" errors (migration may have already run)
    let _ = sqlx::query(migration_sql_3)
        .execute(&pool)
        .await;

    // Run migration 004 (consolidate bookmarks schema)
    let migration_sql_4 = include_str!("../../migrations/004_consolidate_bookmarks.sql");
    sqlx::query(migration_sql_4)
        .execute(&pool)
        .await
        .context("Failed to run migration 004")?;

    Ok(pool)
}

// User models
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
    pub default_alias: Option<String>,
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
        default_alias: None,
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
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, is_admin, default_alias FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

// List all users (admin only)
pub async fn list_all_users(pool: &SqlitePool) -> Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, is_admin, default_alias FROM users ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}

// Session management
pub async fn create_session(pool: &SqlitePool, user_id: i64) -> Result<String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    // Set expiration to 10 years (essentially permanent until logout/password change)
    let expires_at = chrono::Utc::now() + chrono::Duration::days(3650);

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

// Unified bookmark scope - represents whether a bookmark is personal or global
#[derive(Debug, Clone, PartialEq)]
pub enum BookmarkScope {
    Personal { user_id: i64 },
    Global,
}

impl BookmarkScope {
    pub fn to_db_string(&self) -> &'static str {
        match self {
            BookmarkScope::Personal { .. } => "personal",
            BookmarkScope::Global => "global",
        }
    }

    pub fn user_id(&self) -> Option<i64> {
        match self {
            BookmarkScope::Personal { user_id } => Some(*user_id),
            BookmarkScope::Global => None,
        }
    }
}

// Unified bookmark struct (replaces UserBookmark and GlobalBookmark)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Bookmark {
    pub id: i64,
    pub scope: String,  // "personal" or "global"
    pub user_id: Option<i64>,  // NULL for global bookmarks
    pub alias: String,
    pub bookmark_type: String,
    pub url: String,
    pub description: String,
    pub command_template: Option<String>,
    pub encode_query: bool,
    pub created_by: Option<i64>,  // Track who created global bookmarks
}

impl Bookmark {
    pub fn scope_enum(&self) -> BookmarkScope {
        match self.scope.as_str() {
            "personal" => BookmarkScope::Personal {
                user_id: self.user_id.expect("Personal bookmark must have user_id"),
            },
            "global" => BookmarkScope::Global,
            _ => unreachable!("Invalid scope in database: {}", self.scope),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
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

pub async fn get_nested_bookmark_by_id(pool: &SqlitePool, nested_id: i64) -> Result<Option<NestedBookmark>> {
    let nested = sqlx::query_as::<_, NestedBookmark>(
        "SELECT id, parent_bookmark_id, alias, url, description, command_template, encode_query, display_order
         FROM nested_bookmarks
         WHERE id = ?"
    )
    .bind(nested_id)
    .fetch_optional(pool)
    .await?;

    Ok(nested)
}

// Update a bookmark
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

// Update user's default alias preference
pub async fn update_user_default_alias(pool: &SqlitePool, user_id: i64, default_alias: Option<&str>) -> Result<()> {
    sqlx::query(
        "UPDATE users SET default_alias = ? WHERE id = ?"
    )
    .bind(default_alias)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

// Global bookmark functions

// Check if global bookmarks table is empty
// ============================================================================
// UNIFIED BOOKMARK API
// ============================================================================

// Create a bookmark in the unified schema
pub async fn create_bookmark(
    pool: &SqlitePool,
    scope: BookmarkScope,
    alias: &str,
    bookmark_type: &str,
    url: &str,
    description: &str,
    command_template: Option<&str>,
    encode_query: bool,
    created_by: Option<i64>,
) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO bookmarks
         (scope, user_id, alias, bookmark_type, url, description, command_template, encode_query, created_by)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(scope.to_db_string())
    .bind(scope.user_id())
    .bind(alias)
    .bind(bookmark_type)
    .bind(url)
    .bind(description)
    .bind(command_template)
    .bind(encode_query)
    .bind(created_by)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

// Get all bookmarks for a given scope
pub async fn get_bookmarks(
    pool: &SqlitePool,
    scope: BookmarkScope,
) -> Result<Vec<Bookmark>> {
    let bookmarks = sqlx::query_as::<_, Bookmark>(
        "SELECT id, scope, user_id, alias, bookmark_type, url, description, command_template, encode_query, created_by
         FROM bookmarks
         WHERE scope = ? AND (user_id = ? OR user_id IS NULL)
         ORDER BY alias"
    )
    .bind(scope.to_db_string())
    .bind(scope.user_id())
    .fetch_all(pool)
    .await?;

    Ok(bookmarks)
}

// Get a single bookmark by ID
pub async fn get_bookmark_by_id(
    pool: &SqlitePool,
    bookmark_id: i64,
) -> Result<Option<Bookmark>> {
    let bookmark = sqlx::query_as::<_, Bookmark>(
        "SELECT id, scope, user_id, alias, bookmark_type, url, description, command_template, encode_query, created_by
         FROM bookmarks
         WHERE id = ?"
    )
    .bind(bookmark_id)
    .fetch_optional(pool)
    .await?;

    Ok(bookmark)
}

// Update a bookmark
pub async fn update_bookmark(
    pool: &SqlitePool,
    bookmark_id: i64,
    scope: BookmarkScope,
    alias: &str,
    url: &str,
    description: &str,
    command_template: Option<&str>,
    encode_query: bool,
) -> Result<()> {
    sqlx::query(
        "UPDATE bookmarks
         SET alias = ?, url = ?, description = ?, command_template = ?, encode_query = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND scope = ? AND (user_id = ? OR user_id IS NULL)"
    )
    .bind(alias)
    .bind(url)
    .bind(description)
    .bind(command_template)
    .bind(encode_query)
    .bind(bookmark_id)
    .bind(scope.to_db_string())
    .bind(scope.user_id())
    .execute(pool)
    .await?;

    Ok(())
}

// Delete a bookmark
pub async fn delete_bookmark(
    pool: &SqlitePool,
    bookmark_id: i64,
    scope: BookmarkScope,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM bookmarks
         WHERE id = ? AND scope = ? AND (user_id = ? OR user_id IS NULL)"
    )
    .bind(bookmark_id)
    .bind(scope.to_db_string())
    .bind(scope.user_id())
    .execute(pool)
    .await?;

    Ok(())
}

// Get nested bookmarks for a parent bookmark (unified - works for both personal and global)
pub async fn get_nested_bookmarks(
    pool: &SqlitePool,
    parent_bookmark_id: i64,
) -> Result<Vec<NestedBookmark>> {
    let nested = sqlx::query_as::<_, NestedBookmark>(
        "SELECT id, parent_bookmark_id, alias, url, description, command_template, encode_query, display_order
         FROM nested_bookmarks
         WHERE parent_bookmark_id = ?
         ORDER BY display_order, alias"
    )
    .bind(parent_bookmark_id)
    .fetch_all(pool)
    .await?;

    Ok(nested)
}

// Create nested bookmark (unified)
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

// Delete nested bookmark (unified)
pub async fn delete_nested_bookmark(
    pool: &SqlitePool,
    nested_id: i64,
) -> Result<()> {
    sqlx::query("DELETE FROM nested_bookmarks WHERE id = ?")
        .bind(nested_id)
        .execute(pool)
        .await?;

    Ok(())
}

// Check if bookmarks table is empty (for a given scope)
pub async fn is_bookmarks_empty(
    pool: &SqlitePool,
    scope: BookmarkScope,
) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bookmarks WHERE scope = ?"
    )
    .bind(scope.to_db_string())
    .fetch_one(pool)
    .await?;
    Ok(count == 0)
}

// Get bookmarks with nested bookmarks in a single query (fixes N+1 problem)
pub async fn get_bookmarks_with_nested(
    pool: &SqlitePool,
    scope: BookmarkScope,
) -> Result<Vec<(Bookmark, Vec<NestedBookmark>)>> {
    // Fetch all bookmarks for scope
    let bookmarks = get_bookmarks(pool, scope).await?;

    if bookmarks.is_empty() {
        return Ok(vec![]);
    }

    // Collect bookmark IDs
    let bookmark_ids: Vec<i64> = bookmarks.iter().map(|b| b.id).collect();

    // Fetch ALL nested bookmarks in one query
    let all_nested = if !bookmark_ids.is_empty() {
        // Build query with IN clause
        let placeholders = bookmark_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query_str = format!(
            "SELECT id, parent_bookmark_id, alias, url, description, command_template, encode_query, display_order
             FROM nested_bookmarks
             WHERE parent_bookmark_id IN ({})
             ORDER BY parent_bookmark_id, display_order, alias",
            placeholders
        );

        let mut query = sqlx::query_as::<_, NestedBookmark>(&query_str);
        for id in &bookmark_ids {
            query = query.bind(id);
        }

        query.fetch_all(pool).await?
    } else {
        vec![]
    };

    // Group nested bookmarks by parent_id
    let mut nested_map: HashMap<i64, Vec<NestedBookmark>> = HashMap::new();
    for nested in all_nested {
        nested_map.entry(nested.parent_bookmark_id)
            .or_insert_with(Vec::new)
            .push(nested);
    }

    // Combine bookmarks with their nested bookmarks
    let result = bookmarks
        .into_iter()
        .map(|bookmark| {
            let nested = nested_map.remove(&bookmark.id).unwrap_or_default();
            (bookmark, nested)
        })
        .collect();

    Ok(result)
}
