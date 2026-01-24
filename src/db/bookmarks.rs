// Bookmark loading and conversion to Command objects

use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::domain::{
    bookmark_command::BookmarkCommand,
    nested_command::NestedCommand,
    templated_command::TemplatedCommand,
    Command,
};
use crate::db::{self, UserBookmark, NestedBookmark};

// Convert a database bookmark to a Command trait object
pub fn bookmark_to_command(bookmark: &UserBookmark, nested: Vec<NestedBookmark>) -> Box<dyn Command> {
    match bookmark.bookmark_type.as_str() {
        "simple" => {
            Box::new(BookmarkCommand::new(&bookmark.url, &bookmark.description))
        }
        "templated" => {
            let template = bookmark.command_template.as_deref().unwrap_or(&bookmark.url);
            let cmd = TemplatedCommand::new(&bookmark.url, template, &bookmark.description);
            if bookmark.encode_query {
                Box::new(cmd)
            } else {
                Box::new(cmd.with_no_query_encode())
            }
        }
        "nested" => {
            // Build nested commands HashMap
            let mut nested_commands = HashMap::new();
            for nested_bm in nested {
                let nested_cmd: Box<dyn Command> = if nested_bm.command_template.is_some() {
                    let template = nested_bm.command_template.as_ref().unwrap();
                    let cmd = TemplatedCommand::new(&nested_bm.url, template, &nested_bm.description);
                    if nested_bm.encode_query {
                        Box::new(cmd)
                    } else {
                        Box::new(cmd.with_no_query_encode())
                    }
                } else {
                    Box::new(BookmarkCommand::new(&nested_bm.url, &nested_bm.description))
                };
                nested_commands.insert(nested_bm.alias.clone(), nested_cmd);
            }
            Box::new(NestedCommand::new(&bookmark.url, nested_commands, &bookmark.description))
        }
        _ => {
            // Fallback to simple bookmark
            Box::new(BookmarkCommand::new(&bookmark.url, &bookmark.description))
        }
    }
}

// Load all user bookmarks and convert to Commands
pub async fn load_user_bookmarks(pool: &SqlitePool, user_id: i64) -> Result<HashMap<String, Box<dyn Command>>> {
    let bookmarks = db::get_user_bookmarks(pool, user_id).await?;
    let mut commands = HashMap::new();

    for bookmark in bookmarks {
        // Load nested bookmarks if this is a nested command
        let nested = if bookmark.bookmark_type == "nested" {
            db::get_nested_bookmarks(pool, bookmark.id).await?
        } else {
            vec![]
        };

        let command = bookmark_to_command(&bookmark, nested);
        commands.insert(bookmark.alias.clone(), command);
    }

    Ok(commands)
}

// Merge user bookmarks with global bookmarks (user bookmarks take precedence)
pub fn merge_bookmarks(
    global: &HashMap<String, Box<dyn Command>>,
    user: HashMap<String, Box<dyn Command>>,
) -> HashMap<String, Box<dyn Command>> {
    let mut merged = HashMap::new();

    // Start with global bookmarks
    for (alias, command) in global.iter() {
        // Clone the command (we need a way to clone Commands)
        // For now, we'll reference global bookmarks and only add user ones
        // This is a limitation we'll need to address
    }

    // Add/override with user bookmarks
    for (alias, command) in user {
        merged.insert(alias, command);
    }

    merged
}

// Check for alias conflicts between user bookmarks and global bookmarks
pub fn find_conflicts(
    global: &HashMap<String, Box<dyn Command>>,
    user: &HashMap<String, Box<dyn Command>>,
) -> Vec<String> {
    let mut conflicts = Vec::new();

    for alias in user.keys() {
        if global.contains_key(alias) {
            conflicts.push(alias.clone());
        }
    }

    conflicts.sort();
    conflicts
}
