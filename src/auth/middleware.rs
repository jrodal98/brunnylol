// Authentication middleware and extractors

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::CookieJar;
use std::sync::Arc;

use crate::db::{self, User};
use crate::error::AppError;

// Extractor for current authenticated user
// Usage: async fn handler(current_user: CurrentUser) { ... }
pub struct CurrentUser(pub User);

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    Arc<crate::AppState>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract app state
        let app_state = Arc::<crate::AppState>::from_ref(state);

        // Extract cookies using CookieJar extractor
        let jar = CookieJar::from_headers(&parts.headers);

        // Get session cookie
        let session_cookie = jar.get("session_id")
            .ok_or(AppError::Unauthorized("Not logged in".to_string()))?;

        let session_id = session_cookie.value();

        // Validate session
        let user_id = db::validate_session(&app_state.db_pool, session_id)
            .await
            .map_err(|e| AppError::Internal(format!("Session validation error: {}", e)))?
            .ok_or(AppError::Unauthorized("Invalid or expired session".to_string()))?;

        // Get user
        let user = db::get_user_by_id(&app_state.db_pool, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?
            .ok_or(AppError::Unauthorized("User not found".to_string()))?;

        Ok(CurrentUser(user))
    }
}

// Optional user extractor (returns None if not logged in)
pub struct OptionalUser(pub Option<User>);

impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
    Arc<crate::AppState>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract app state
        let app_state = Arc::<crate::AppState>::from_ref(state);

        // Extract cookies using CookieJar extractor
        let jar = CookieJar::from_headers(&parts.headers);

        let session_cookie = match jar.get("session_id") {
            Some(cookie) => cookie,
            None => return Ok(OptionalUser(None)),
        };

        let user_id = match db::validate_session(&app_state.db_pool, session_cookie.value()).await {
            Ok(Some(id)) => id,
            _ => return Ok(OptionalUser(None)),
        };

        let user = db::get_user_by_id(&app_state.db_pool, user_id).await.ok().flatten();
        Ok(OptionalUser(user))
    }
}
