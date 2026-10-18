// Bookmark loading and conversion to Command objects

use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::domain::Command;
use crate::db::{self, Bookmark, NestedBookmark};

// Create a Command::Variable from template string
fn create_variable_command(
    template_str: &str,
    base_url: String,
    description: String,
    variable_metadata: Option<&String>,
) -> Result<Command> {
    let parsed_template = crate::domain::template::TemplateParser::parse(template_str)?;

    let metadata = if let Some(json) = variable_metadata {
        serde_json::from_str(json).ok()
    } else {
        Some(crate::domain::template::TemplateMetadata::from_template(&parsed_template))
    };

    Ok(Command::Variable {
        base_url,
        template: parsed_template,
        description,
        metadata,
    })
}

// Convert a unified database bookmark to a Command enum (v2 API)
pub fn bookmark_to_command(bookmark: &Bookmark, nested: Vec<NestedBookmark>) -> Result<Command> {
    match bookmark.bookmark_type.as_str() {
        "simple" => {
            // Simple bookmarks become Variable commands with empty template (no variables)
            let empty_template = crate::domain::template::TemplateParser::parse("")?;
            Ok(Command::Variable {
                base_url: bookmark.url.clone(),
                template: empty_template,
                description: bookmark.description.clone(),
                metadata: None,
            })
        }
        "templated" => {
            let template = bookmark.command_template.as_deref().unwrap_or("");

            // All templates become Variable commands
            create_variable_command(
                template,
                bookmark.url.clone(),
                bookmark.description.clone(),
                bookmark.variable_metadata.as_ref(),
            )
        }
        "nested" => {
            // Build nested commands HashMap
            let mut nested_commands = HashMap::new();
            for nested_bm in nested {
                let nested_cmd = if let Some(template) = &nested_bm.command_template {
                    create_variable_command(
                        template,
                        nested_bm.url.clone(),
                        nested_bm.description.clone(),
                        nested_bm.variable_metadata.as_ref(),
                    )?
                } else {
                    // No template - create empty Variable command
                    let empty_template = crate::domain::template::TemplateParser::parse("")?;
                    Command::Variable {
                        base_url: nested_bm.url.clone(),
                        template: empty_template,
                        description: nested_bm.description.clone(),
                        metadata: None,
                    }
                };
                nested_commands.insert(nested_bm.alias.clone(), nested_cmd);
            }
            Ok(Command::Nested {
                base_url: bookmark.url.clone(),
                children: nested_commands,
                description: bookmark.description.clone(),
            })
        }
        _ => {
            // Fallback to empty Variable command
            let empty_template = crate::domain::template::TemplateParser::parse("")?;
            Ok(Command::Variable {
                base_url: bookmark.url.clone(),
                template: empty_template,
                description: bookmark.description.clone(),
                metadata: None,
            })
        }
    }
}


/// Record one use of a personal bookmark (upsert into bookmark_usage).
/// Errors are the caller's choice to ignore: usage metrics must never
/// break the redirect they describe.
pub async fn record_bookmark_use(pool: &SqlitePool, user_id: i64, alias: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO bookmark_usage (bookmark_id, use_count, last_used_at)
         SELECT id, 1, CURRENT_TIMESTAMP FROM bookmarks
         WHERE scope = 'personal' AND user_id = ? AND alias = ?
         ON CONFLICT(bookmark_id) DO UPDATE SET
             use_count = use_count + 1,
             last_used_at = CURRENT_TIMESTAMP",
    )
    .bind(user_id)
    .bind(alias)
    .execute(pool)
    .await?;
    Ok(())
}

/// Record one use of a global (built-in) bookmark. Global counts are
/// instance-wide: they accumulate across all users, signed in or not.
pub async fn record_global_bookmark_use(pool: &SqlitePool, alias: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO bookmark_usage (bookmark_id, use_count, last_used_at)
         SELECT id, 1, CURRENT_TIMESTAMP FROM bookmarks
         WHERE scope = 'global' AND alias = ?
         ON CONFLICT(bookmark_id) DO UPDATE SET
             use_count = use_count + 1,
             last_used_at = CURRENT_TIMESTAMP",
    )
    .bind(alias)
    .execute(pool)
    .await?;
    Ok(())
}

/// Usage stats for global bookmarks:
/// alias -> (use_count, last_used_at as display string).
pub async fn get_global_usage_by_alias(
    pool: &SqlitePool,
) -> Result<HashMap<String, (i64, Option<String>)>> {
    let rows = sqlx::query(
        "SELECT b.alias AS alias, u.use_count AS use_count,
                CAST(u.last_used_at AS TEXT) AS last_used_at
         FROM bookmark_usage u
         JOIN bookmarks b ON b.id = u.bookmark_id
         WHERE b.scope = 'global'",
    )
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::new();
    for row in rows {
        use sqlx::Row;
        let alias: String = row.get("alias");
        let use_count: i64 = row.get("use_count");
        let last_used_at: Option<String> = row.get("last_used_at");
        map.insert(alias, (use_count, last_used_at));
    }
    Ok(map)
}

/// Usage stats for a user's personal bookmarks:
/// bookmark_id -> (use_count, last_used_at as display string).
pub async fn get_usage_map(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<HashMap<i64, (i64, Option<String>)>> {
    let rows = sqlx::query(
        "SELECT u.bookmark_id AS bookmark_id, u.use_count AS use_count,
                CAST(u.last_used_at AS TEXT) AS last_used_at
         FROM bookmark_usage u
         JOIN bookmarks b ON b.id = u.bookmark_id
         WHERE b.scope = 'personal' AND b.user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::new();
    for row in rows {
        use sqlx::Row;
        let bookmark_id: i64 = row.get("bookmark_id");
        let use_count: i64 = row.get("use_count");
        let last_used_at: Option<String> = row.get("last_used_at");
        map.insert(bookmark_id, (use_count, last_used_at));
    }
    Ok(map)
}

// Load all user bookmarks and convert to Commands
pub async fn load_user_bookmarks(pool: &SqlitePool, user_id: i64) -> Result<HashMap<String, Command>> {
    // Use optimized v2 API that fetches bookmarks + nested in single query
    let bookmarks_with_nested = db::get_bookmarks_with_nested(pool, db::BookmarkScope::Personal { user_id }).await?;
    let mut commands = HashMap::new();

    for (bookmark, nested) in bookmarks_with_nested {
        match bookmark_to_command(&bookmark, nested) {
            Ok(command) => {
                commands.insert(bookmark.alias.clone(), command);
            }
            Err(e) => {
                eprintln!("Warning: Failed to load bookmark '{}': {}. Skipping.", bookmark.alias, e);
            }
        }
    }

    Ok(commands)
}
