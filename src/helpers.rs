// Shared helper functions used across handlers

use std::sync::Arc;
use crate::{AppState, db, domain::Command, error::AppError, validation};

/// Load a bookmark for the given alias, checking user bookmarks first, then global bookmarks
/// Respects user-disabled global bookmarks
pub async fn load_bookmark_for_alias(
    alias: &str,
    user: Option<&db::User>,
    state: &Arc<AppState>,
) -> Option<Command> {
    if let Some(user) = user {
        // Check user's personal bookmarks first
        if let Ok(user_bookmarks) = db::bookmarks::load_user_bookmarks(&state.db_pool, user.id).await {
            if let Some(cmd) = user_bookmarks.get(alias).cloned() {
                return Some(cmd);
            }
        }

        // Check if user has disabled this global bookmark
        let disabled_globals = db::get_disabled_global_aliases(&state.db_pool, user.id).await;
        if disabled_globals.contains(alias) {
            return None;
        }
    }

    let bookmark_map = state.alias_to_bookmark_map.read().await;
    bookmark_map.get(alias).cloned()
}

/// Validate an optional template if it's non-empty
pub fn validate_optional_template(template: &Option<String>) -> Result<(), AppError> {
    if let Some(ref t) = template {
        if !t.is_empty() {
            validation::validate_variable_template(t)?;
        }
    }
    Ok(())
}
