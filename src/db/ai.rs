// Database access for AI settings.
//
// Two levels of configuration, same shape:
//   * Global settings (singleton) — managed by admins, optionally shared
//     with other users according to `access_mode`.
//   * Per-user settings — managed by each user, applies only to them.
//
// Any field left NULL/empty means "not set at this level" and resolution
// falls back (user -> global DB value -> environment variable). See
// services::ai_service for the resolution and permission logic.

use anyhow::Result;
use sqlx::{Row, SqlitePool};

/// Access modes for the shared (admin-managed) AI configuration.
pub const ACCESS_MODES: [&str; 4] = ["disabled", "admins_only", "all_users", "selected_users"];

pub fn is_valid_access_mode(mode: &str) -> bool {
    ACCESS_MODES.contains(&mode)
}

#[derive(Debug, Clone, Default)]
pub struct GlobalAiSettings {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    /// One of ACCESS_MODES. Defaults to "admins_only" when no row exists.
    pub access_mode: String,
}

#[derive(Debug, Clone, Default)]
pub struct UserAiSettings {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    /// User-level opt-out. Only meaningful when a settings row exists;
    /// users without a row are treated as enabled.
    pub enabled: bool,
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

/// Normalize an optional form/DB value: blank becomes None ("fall back").
pub fn blank_to_none(value: &str) -> Option<String> {
    non_empty(Some(value.to_string()))
}

pub async fn get_global_ai_settings(pool: &SqlitePool) -> Result<GlobalAiSettings> {
    let row = sqlx::query(
        "SELECT base_url, api_key, model, access_mode FROM ai_global_settings WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => Ok(GlobalAiSettings {
            base_url: non_empty(row.get::<Option<String>, _>("base_url")),
            api_key: non_empty(row.get::<Option<String>, _>("api_key")),
            model: non_empty(row.get::<Option<String>, _>("model")),
            access_mode: row.get::<String, _>("access_mode"),
        }),
        None => Ok(GlobalAiSettings {
            access_mode: "admins_only".to_string(),
            ..Default::default()
        }),
    }
}

pub async fn upsert_global_ai_settings(
    pool: &SqlitePool,
    settings: &GlobalAiSettings,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ai_global_settings (id, base_url, api_key, model, access_mode, updated_at)
         VALUES (1, ?, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
            base_url = excluded.base_url,
            api_key = excluded.api_key,
            model = excluded.model,
            access_mode = excluded.access_mode,
            updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&settings.base_url)
    .bind(&settings.api_key)
    .bind(&settings.model)
    .bind(&settings.access_mode)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns None when the user has never saved AI settings.
pub async fn get_user_ai_settings(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Option<UserAiSettings>> {
    let row = sqlx::query(
        "SELECT base_url, api_key, model, enabled FROM ai_user_settings WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| UserAiSettings {
        base_url: non_empty(row.get::<Option<String>, _>("base_url")),
        api_key: non_empty(row.get::<Option<String>, _>("api_key")),
        model: non_empty(row.get::<Option<String>, _>("model")),
        enabled: row.get::<bool, _>("enabled"),
    }))
}

pub async fn upsert_user_ai_settings(
    pool: &SqlitePool,
    user_id: i64,
    settings: &UserAiSettings,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ai_user_settings (user_id, base_url, api_key, model, enabled, updated_at)
         VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(user_id) DO UPDATE SET
            base_url = excluded.base_url,
            api_key = excluded.api_key,
            model = excluded.model,
            enabled = excluded.enabled,
            updated_at = CURRENT_TIMESTAMP",
    )
    .bind(user_id)
    .bind(&settings.base_url)
    .bind(&settings.api_key)
    .bind(&settings.model)
    .bind(settings.enabled)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_allowed_user_ids(pool: &SqlitePool) -> Result<Vec<i64>> {
    let rows = sqlx::query("SELECT user_id FROM ai_allowed_users ORDER BY user_id")
        .fetch_all(pool)
        .await?;
    Ok(rows.iter().map(|r| r.get::<i64, _>("user_id")).collect())
}

pub async fn is_user_allowed(pool: &SqlitePool, user_id: i64) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_allowed_users WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(count > 0)
}

/// Replace the allow-list with exactly the given user ids.
pub async fn set_allowed_users(pool: &SqlitePool, user_ids: &[i64]) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM ai_allowed_users")
        .execute(&mut *tx)
        .await?;
    for user_id in user_ids {
        sqlx::query("INSERT OR IGNORE INTO ai_allowed_users (user_id) VALUES (?)")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
