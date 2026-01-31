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

    let metadata = if let Some(ref json) = variable_metadata {
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
            Ok(Command::Simple {
                url: bookmark.url.clone(),
                description: bookmark.description.clone(),
            })
        }
        "templated" => {
            let template = bookmark.command_template.as_deref().unwrap_or(&bookmark.url);

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
                    Command::Simple {
                        url: nested_bm.url.clone(),
                        description: nested_bm.description.clone(),
                    }
                };
                nested_commands.insert(nested_bm.alias.clone(), nested_cmd);
            }
            Ok(Command::Nested {
                children: nested_commands,
                description: bookmark.description.clone(),
            })
        }
        _ => {
            // Fallback to simple bookmark
            Ok(Command::Simple {
                url: bookmark.url.clone(),
                description: bookmark.description.clone(),
            })
        }
    }
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
