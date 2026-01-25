// Bookmark service - business logic for bookmark management

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::{
    config::yml_settings::YmlSettings,
    db::{self, GlobalBookmark, UserBookmark, NestedBookmark},
    domain::Command,
    services::serializers::BookmarkSerializer,
};

pub struct BookmarkService {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BookmarkScope {
    Personal,
    Global,
}

#[derive(Debug)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

impl BookmarkService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Load all bookmarks for a user (global + personal, with overrides)
    pub async fn load_user_bookmarks(&self, user_id: i64) -> Result<HashMap<String, Box<dyn Command>>> {
        // 1. Load global bookmarks
        let global = self.load_global_bookmarks().await?;

        // 2. Load user bookmarks
        let personal = db::bookmarks::load_user_bookmarks(&self.pool, user_id).await?;

        // 3. Load user overrides (disabled globals)
        let overrides = db::get_user_overrides(&self.pool, user_id).await?;
        let disabled: std::collections::HashSet<String> = overrides
            .iter()
            .filter(|(_, is_disabled, _, _)| *is_disabled)
            .map(|(alias, _, _, _)| alias.clone())
            .collect();

        // 4. Merge: Personal overrides global, disabled globals excluded
        let mut merged = HashMap::new();

        for (alias, command) in global {
            if !disabled.contains(&alias) && !personal.contains_key(&alias) {
                merged.insert(alias, command);
            }
        }

        for (alias, command) in personal {
            merged.insert(alias, command);
        }

        Ok(merged)
    }

    /// Load all global bookmarks as Command objects
    pub async fn load_global_bookmarks(&self) -> Result<HashMap<String, Box<dyn Command>>> {
        let global_bookmarks = db::get_all_global_bookmarks(&self.pool).await?;
        let mut commands = HashMap::new();

        for bookmark in global_bookmarks {
            let nested = if bookmark.bookmark_type == "nested" {
                db::get_global_nested_bookmarks(&self.pool, bookmark.id).await?
            } else {
                vec![]
            };

            // Convert to NestedBookmark type expected by bookmark_to_command
            let nested_converted: Vec<NestedBookmark> = nested
                .into_iter()
                .map(|n| NestedBookmark {
                    id: n.id,
                    parent_bookmark_id: n.parent_bookmark_id,
                    alias: n.alias,
                    url: n.url,
                    description: n.description,
                    command_template: n.command_template,
                    encode_query: n.encode_query,
                    display_order: n.display_order,
                })
                .collect();

            // Reuse existing conversion logic
            let user_bookmark = UserBookmark {
                id: bookmark.id,
                user_id: 0, // Not used for globals
                alias: bookmark.alias.clone(),
                bookmark_type: bookmark.bookmark_type,
                url: bookmark.url,
                description: bookmark.description,
                command_template: bookmark.command_template,
                encode_query: bookmark.encode_query,
            };

            match db::bookmarks::bookmark_to_command(&user_bookmark, nested_converted) {
                Ok(command) => {
                    commands.insert(bookmark.alias.clone(), command);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load global bookmark '{}': {}", bookmark.alias, e);
                }
            }
        }

        Ok(commands)
    }

    /// Import bookmarks from serialized content
    pub async fn import_bookmarks(
        &self,
        content: &str,
        serializer: &dyn BookmarkSerializer,
        scope: BookmarkScope,
        created_by: Option<i64>, // None for system imports, Some(user_id) for user imports
    ) -> Result<ImportResult> {
        let settings: Vec<YmlSettings> = serializer.deserialize(content)
            .context("Failed to parse bookmark data")?;

        let mut imported = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        for setting in settings {
            let result = match scope {
                BookmarkScope::Personal => {
                    let user_id = created_by.context("user_id required for personal bookmark import")?;
                    self.import_personal_bookmark(&setting, user_id).await
                }
                BookmarkScope::Global => {
                    self.import_global_bookmark(&setting, created_by).await
                }
            };

            match result {
                Ok(_) => imported += 1,
                Err(e) => {
                    if e.to_string().contains("UNIQUE constraint") {
                        skipped += 1;
                    } else {
                        errors.push(format!("'{}': {}", setting.alias, e));
                    }
                }
            }
        }

        Ok(ImportResult {
            imported,
            skipped,
            errors,
        })
    }

    /// Export bookmarks to serialized format
    pub async fn export_bookmarks(
        &self,
        scope: BookmarkScope,
        user_id: Option<i64>,
        serializer: &dyn BookmarkSerializer,
    ) -> Result<String> {
        let bookmarks = match scope {
            BookmarkScope::Personal => {
                let user_id = user_id.context("user_id required for personal export")?;
                self.export_personal_bookmarks(user_id).await?
            }
            BookmarkScope::Global => {
                self.export_global_bookmarks().await?
            }
        };

        serializer.serialize(&bookmarks)
    }

    /// Seed global bookmarks from embedded commands.yml if DB is empty
    pub async fn seed_global_bookmarks(&self) -> Result<usize> {
        let is_empty = db::is_global_bookmarks_empty(&self.pool).await?;

        if !is_empty {
            return Ok(0); // Already seeded
        }

        let yaml_content = include_str!("../../commands.yml");
        let serializer = crate::services::serializers::YamlSerializer;

        let result = self.import_bookmarks(
            yaml_content,
            &serializer,
            BookmarkScope::Global,
            None, // System import (no user ID)
        ).await?;

        if !result.errors.is_empty() {
            eprintln!("Errors during global bookmark seeding:");
            for error in &result.errors {
                eprintln!("  - {}", error);
            }
        }

        Ok(result.imported)
    }

    // --- Private helper methods ---

    fn determine_bookmark_type(setting: &YmlSettings) -> &'static str {
        if setting.nested.is_some() {
            "nested"
        } else if setting.command.is_some() {
            "templated"
        } else {
            "simple"
        }
    }

    async fn import_personal_bookmark(&self, setting: &YmlSettings, user_id: i64) -> Result<i64> {
        let bookmark_type = Self::determine_bookmark_type(setting);

        let bookmark_id = db::create_bookmark(
            &self.pool,
            user_id,
            &setting.alias,
            bookmark_type,
            &setting.url,
            &setting.description,
            setting.command.as_deref(),
            setting.encode.unwrap_or(true),
        ).await?;

        // Import nested commands if present
        if let Some(nested) = &setting.nested {
            for (i, nested_setting) in nested.iter().enumerate() {
                db::create_nested_bookmark(
                    &self.pool,
                    bookmark_id,
                    &nested_setting.alias,
                    &nested_setting.url,
                    &nested_setting.description,
                    nested_setting.command.as_deref(),
                    nested_setting.encode.unwrap_or(true),
                    i as i32,
                ).await?;
            }
        }

        Ok(bookmark_id)
    }

    async fn import_global_bookmark(&self, setting: &YmlSettings, created_by: Option<i64>) -> Result<i64> {
        let bookmark_type = Self::determine_bookmark_type(setting);

        let bookmark_id = db::create_global_bookmark(
            &self.pool,
            &setting.alias,
            bookmark_type,
            &setting.url,
            &setting.description,
            setting.command.as_deref(),
            setting.encode.unwrap_or(true),
            created_by,
        ).await?;

        // Import nested commands if present
        if let Some(nested) = &setting.nested {
            for (i, nested_setting) in nested.iter().enumerate() {
                db::create_global_nested_bookmark(
                    &self.pool,
                    bookmark_id,
                    &nested_setting.alias,
                    &nested_setting.url,
                    &nested_setting.description,
                    nested_setting.command.as_deref(),
                    nested_setting.encode.unwrap_or(true),
                    i as i32,
                ).await?;
            }
        }

        Ok(bookmark_id)
    }

    async fn export_personal_bookmarks(&self, user_id: i64) -> Result<Vec<YmlSettings>> {
        let bookmarks = db::get_user_bookmarks(&self.pool, user_id).await?;
        self.bookmarks_to_yml_settings(bookmarks).await
    }

    async fn export_global_bookmarks(&self) -> Result<Vec<YmlSettings>> {
        let bookmarks = db::get_all_global_bookmarks(&self.pool).await?;

        let mut settings = Vec::new();

        for bookmark in bookmarks {
            let nested = if bookmark.bookmark_type == "nested" {
                // Query global nested bookmarks table, not user nested bookmarks
                let nested_bookmarks = db::get_global_nested_bookmarks(&self.pool, bookmark.id).await?;
                Some(
                    nested_bookmarks
                        .into_iter()
                        .map(|n| YmlSettings {
                            alias: n.alias,
                            description: n.description,
                            url: n.url,
                            command: n.command_template,
                            encode: Some(n.encode_query),
                            nested: None,
                        })
                        .collect()
                )
            } else {
                None
            };

            settings.push(YmlSettings {
                alias: bookmark.alias,
                description: bookmark.description,
                url: bookmark.url,
                command: bookmark.command_template,
                encode: Some(bookmark.encode_query),
                nested,
            });
        }

        Ok(settings)
    }

    async fn bookmarks_to_yml_settings(&self, bookmarks: Vec<UserBookmark>) -> Result<Vec<YmlSettings>> {
        let mut settings = Vec::new();

        for bookmark in bookmarks {
            let nested = if bookmark.bookmark_type == "nested" {
                let nested_bookmarks = db::get_nested_bookmarks(&self.pool, bookmark.id).await?;
                Some(
                    nested_bookmarks
                        .into_iter()
                        .map(|n| YmlSettings {
                            alias: n.alias,
                            description: n.description,
                            url: n.url,
                            command: n.command_template,
                            encode: Some(n.encode_query),
                            nested: None,
                        })
                        .collect()
                )
            } else {
                None
            };

            settings.push(YmlSettings {
                alias: bookmark.alias,
                description: bookmark.description,
                url: bookmark.url,
                command: bookmark.command_template,
                encode: Some(bookmark.encode_query),
                nested,
            });
        }

        Ok(settings)
    }
}
