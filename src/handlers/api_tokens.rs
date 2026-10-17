// Settings-page handlers for personal API tokens: create (shows the
// plaintext exactly once) and revoke. Both return HTMX fragments —
// the create response reveals the new token and refreshes the token
// list out-of-band; revoke returns the refreshed list.

use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{Html, IntoResponse},
};
use serde::Deserialize;

use crate::{auth::middleware::CurrentUser, db, error::AppError, AppState};

/// One row of the token list (display-ready strings).
#[derive(Clone)]
pub struct TokenRow {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub last_used_at: String,
}

impl From<db::tokens::ApiToken> for TokenRow {
    fn from(t: db::tokens::ApiToken) -> Self {
        TokenRow {
            id: t.id,
            name: t.name,
            created_at: t.created_at,
            last_used_at: t.last_used_at.unwrap_or_else(|| "never".to_string()),
        }
    }
}

pub async fn token_rows(pool: &sqlx::SqlitePool, user_id: i64) -> Vec<TokenRow> {
    db::tokens::list_api_tokens(pool, user_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(TokenRow::from)
        .collect()
}

#[derive(Template)]
#[template(path = "partials/api_token_list.html")]
pub struct ApiTokenListTemplate {
    pub tokens: Vec<TokenRow>,
}

async fn render_token_list(pool: &sqlx::SqlitePool, user_id: i64) -> Result<String, AppError> {
    let template = ApiTokenListTemplate {
        tokens: token_rows(pool, user_id).await,
    };
    Ok(template.render()?)
}

#[derive(Deserialize)]
pub struct CreateTokenForm {
    name: String,
}

/// POST /settings/api-tokens — create a token; the response fragment
/// shows the plaintext ONCE (only its hash is stored).
pub async fn create_token(
    current_user: CurrentUser,
    State(state): State<Arc<AppState>>,
    Form(form): Form<CreateTokenForm>,
) -> Result<impl IntoResponse, AppError> {
    let name = form.name.trim();
    if name.is_empty() || name.len() > 60 {
        return Err(AppError::BadRequest(
            "Token name must be 1-60 characters".to_string(),
        ));
    }

    let (_id, plaintext) = db::tokens::create_api_token(&state.db_pool, current_user.0.id, name)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create token: {}", e)))?;

    let list_html = render_token_list(&state.db_pool, current_user.0.id)
        .await?
        // Mark the list as an out-of-band swap alongside the reveal.
        .replacen(
            "id=\"api-token-list\"",
            "id=\"api-token-list\" hx-swap-oob=\"true\"",
            1,
        );

    let html = format!(
        r#"<div class="ai-proposal-card" style="margin-bottom: var(--space-4);">
            <p style="margin-bottom: var(--space-2);"><strong>Your new API token</strong> — copy it now, it will not be shown again:</p>
            <p><code style="word-break: break-all;">{}</code></p>
            <p style="color: var(--text-secondary);">Use it as <code>Authorization: Bearer &lt;token&gt;</code> against the /api/bookmarks endpoints.</p>
        </div>{}"#,
        plaintext, list_html
    );
    Ok(Html(html))
}

/// POST /settings/api-tokens/{id}/revoke — delete a token and return
/// the refreshed list.
pub async fn revoke_token(
    current_user: CurrentUser,
    State(state): State<Arc<AppState>>,
    Path(token_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    db::tokens::revoke_api_token(&state.db_pool, current_user.0.id, token_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to revoke token: {}", e)))?;
    Ok(Html(
        render_token_list(&state.db_pool, current_user.0.id).await?,
    ))
}
