// Bookmark loading and conversion to Command objects

use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::domain::Command;
use crate::db::{self, Bookmark, NestedBookmark};

// Detect if a template uses variable syntax (beyond simple {query})
fn is_variable_template(template: &str) -> bool {
    if !template.contains('{') {
        return false;
    }

    // Check for advanced variable syntax:
    // - Multiple variables: {var1}/{var2}
    // - Optional marker: {var?}
    // - Default value: {var=default}
    // - Pipeline operations: {var|encode}
    template.matches('{').count() > 1
        || template.contains('?')
        || template.contains('=')
        || template.contains('|')
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

            // Check if this is a variable template
            if is_variable_template(template) {
                // Parse template
                let parsed_template = crate::domain::template::TemplateParser::parse(template)?;

                // Deserialize metadata if present
                let metadata = if let Some(ref json) = bookmark.variable_metadata {
                    serde_json::from_str(json).ok()
                } else {
                    Some(crate::domain::template::TemplateMetadata::from_template(&parsed_template))
                };

                Ok(Command::Variable {
                    base_url: bookmark.url.clone(),
                    template: parsed_template,
                    description: bookmark.description.clone(),
                    metadata,
                })
            } else {
                // Simple template with just {query}
                Ok(Command::Templated {
                    base_url: bookmark.url.clone(),
                    template: template.to_string(),
                    description: bookmark.description.clone(),
                    encode_query: bookmark.encode_query,
                })
            }
        }
        "nested" => {
            // Build nested commands HashMap
            let mut nested_commands = HashMap::new();
            for nested_bm in nested {
                let nested_cmd = if let Some(template) = &nested_bm.command_template {
                    // Check if variable template
                    if is_variable_template(template) {
                        let parsed_template = crate::domain::template::TemplateParser::parse(template)?;
                        let metadata = if let Some(ref json) = nested_bm.variable_metadata {
                            serde_json::from_str(json).ok()
                        } else {
                            Some(crate::domain::template::TemplateMetadata::from_template(&parsed_template))
                        };

                        Command::Variable {
                            base_url: nested_bm.url.clone(),
                            template: parsed_template,
                            description: nested_bm.description.clone(),
                            metadata,
                        }
                    } else {
                        Command::Templated {
                            base_url: nested_bm.url.clone(),
                            template: template.clone(),
                            description: nested_bm.description.clone(),
                            encode_query: nested_bm.encode_query,
                        }
                    }
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
