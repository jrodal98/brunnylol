// JSON API for personal bookmarks, authenticated with a personal API
// token (Authorization: Bearer brn_...). Built for scripts and AI
// agents: predictable shapes, JSON errors, no HTML, no redirects.
//
// Shapes:
//   Bookmark: { alias, type, url, description, command|null,
//               use_count, last_used_at|null,
//               nested: [{ alias, url, description, command|null }] }
//   Create/update input: { alias (create only), url, description,
//               command?, nested? } — the bookmark type is inferred:
//               nested present -> "nested", command present ->
//               "templated", otherwise "simple".

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{auth::middleware::ApiUser, db, helpers, validation, AppState};

#[derive(Serialize)]
struct NestedJson {
    alias: String,
    url: String,
    description: String,
    command: Option<String>,
}

#[derive(Serialize)]
struct BookmarkJson {
    alias: String,
    #[serde(rename = "type")]
    bookmark_type: String,
    url: String,
    description: String,
    command: Option<String>,
    use_count: i64,
    last_used_at: Option<String>,
    nested: Vec<NestedJson>,
}

#[derive(Deserialize)]
pub struct NestedInput {
    alias: String,
    url: String,
    description: String,
    command: Option<String>,
}

#[derive(Deserialize)]
pub struct BookmarkInput {
    alias: Option<String>,
    url: Option<String>,
    description: Option<String>,
    command: Option<String>,
    nested: Option<Vec<NestedInput>>,
}

/// JSON API error: a status plus a message, rendered as
/// `{"error": message}`. A small payload type (instead of a full
/// `Response`, which is 128+ bytes) keeps `Result<T, ApiError>`
/// return values cheap in the handlers' internal helpers.
struct ApiError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

fn api_error(status: StatusCode, message: &str) -> ApiError {
    ApiError {
        status,
        message: message.to_string(),
    }
}

fn bad_request(message: &str) -> ApiError {
    api_error(StatusCode::BAD_REQUEST, message)
}

fn not_found(alias: &str) -> ApiError {
    api_error(
        StatusCode::NOT_FOUND,
        &format!("no personal bookmark with alias '{}'", alias),
    )
}

async fn load_personal(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<(db::Bookmark, Vec<db::NestedBookmark>)>, ApiError> {
    db::get_bookmarks_with_nested(&state.db_pool, db::BookmarkScope::Personal { user_id })
        .await
        .map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("database error: {}", e),
            )
        })
}

fn to_json(
    bookmark: db::Bookmark,
    nested: Vec<db::NestedBookmark>,
    usage: (i64, Option<String>),
) -> BookmarkJson {
    let (use_count, last_used_at) = usage;
    BookmarkJson {
        alias: bookmark.alias,
        bookmark_type: bookmark.bookmark_type,
        url: bookmark.url,
        description: bookmark.description,
        command: bookmark.command_template,
        use_count,
        last_used_at,
        nested: nested
            .into_iter()
            .map(|n| NestedJson {
                alias: n.alias,
                url: n.url,
                description: n.description,
                command: n.command_template,
            })
            .collect(),
    }
}

/// Validate shared create/update input; returns (url, description,
/// bookmark_type). Mirrors the web form's validation rules.
fn validate_input(input: &BookmarkInput) -> Result<(String, String, String), ApiError> {
    let url = input.url.clone().unwrap_or_default();
    let description = input.description.clone().unwrap_or_default();
    if description.is_empty() {
        return Err(bad_request("description is required"));
    }
    let bookmark_type = if input.nested.is_some() {
        "nested"
    } else if input
        .command
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false)
    {
        "templated"
    } else {
        "simple"
    }
    .to_string();

    if bookmark_type != "nested" {
        if url.is_empty() {
            return Err(bad_request("url is required"));
        }
        validation::validate_url_scheme(&url).map_err(|e| bad_request(&e.to_string()))?;
    }
    if let Some(command) = &input.command {
        let opt = if command.is_empty() {
            None
        } else {
            Some(command.clone())
        };
        helpers::validate_optional_template(&opt).map_err(|e| bad_request(&e.to_string()))?;
    }
    if let Some(nested) = &input.nested {
        if nested.len() > 100 {
            return Err(bad_request("too many nested commands (maximum 100)"));
        }
        for child in nested {
            validation::validate_url_scheme(&child.url).map_err(|e| bad_request(&e.to_string()))?;
            let opt = child.command.clone().filter(|c| !c.is_empty());
            helpers::validate_optional_template(&opt).map_err(|e| bad_request(&e.to_string()))?;
        }
    }
    Ok((url, description, bookmark_type))
}

async fn insert_nested(
    state: &AppState,
    parent_id: i64,
    nested: &[NestedInput],
) -> Result<(), ApiError> {
    for (i, child) in nested.iter().enumerate() {
        db::create_nested_bookmark(
            &state.db_pool,
            parent_id,
            &child.alias,
            &child.url,
            &child.description,
            child.command.as_deref().filter(|c| !c.is_empty()),
            i as i32,
        )
        .await
        .map_err(|e| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to create nested bookmark: {}", e),
            )
        })?;
    }
    Ok(())
}

/// GET /api — this API's documentation as markdown, so scripts and
/// agents can fetch the reference from the deployment itself (and
/// therefore always get the version matching the server).
pub async fn docs() -> Response {
    (
        StatusCode::OK,
        [("content-type", "text/markdown; charset=utf-8")],
        include_str!("../../docs/api.md"),
    )
        .into_response()
}

/// GET /api/whoami — verify a token and see whose it is.
pub async fn whoami(ApiUser(user): ApiUser) -> Response {
    Json(serde_json::json!({
        "username": user.username,
        "is_admin": user.is_admin,
    }))
    .into_response()
}

/// GET /api/bookmarks — list the token owner's personal bookmarks.
pub async fn list_bookmarks(
    ApiUser(user): ApiUser,
    State(state): State<Arc<AppState>>,
) -> Response {
    let all = match load_personal(&state, user.id).await {
        Ok(v) => v,
        Err(r) => return r.into_response(),
    };
    let usage_map = db::bookmarks::get_usage_map(&state.db_pool, user.id)
        .await
        .unwrap_or_default();
    let out: Vec<BookmarkJson> = all
        .into_iter()
        .map(|(bookmark, nested)| {
            let usage = usage_map.get(&bookmark.id).cloned().unwrap_or((0, None));
            to_json(bookmark, nested, usage)
        })
        .collect();
    Json(serde_json::json!({ "bookmarks": out })).into_response()
}

/// GET /api/bookmarks/{alias} — one bookmark.
pub async fn get_bookmark(
    ApiUser(user): ApiUser,
    State(state): State<Arc<AppState>>,
    Path(alias): Path<String>,
) -> Response {
    let all = match load_personal(&state, user.id).await {
        Ok(v) => v,
        Err(r) => return r.into_response(),
    };
    let usage_map = db::bookmarks::get_usage_map(&state.db_pool, user.id)
        .await
        .unwrap_or_default();
    for (bookmark, nested) in all {
        if bookmark.alias == alias {
            let usage = usage_map.get(&bookmark.id).cloned().unwrap_or((0, None));
            return Json(to_json(bookmark, nested, usage)).into_response();
        }
    }
    not_found(&alias).into_response()
}

/// POST /api/bookmarks — create a bookmark. 201 + the bookmark.
pub async fn create_bookmark(
    ApiUser(user): ApiUser,
    State(state): State<Arc<AppState>>,
    Json(input): Json<BookmarkInput>,
) -> Response {
    let alias = match &input.alias {
        Some(a) if !a.is_empty() && a.len() <= 50 => a.clone(),
        _ => return bad_request("alias is required (1-50 characters)").into_response(),
    };
    let (url, description, bookmark_type) = match validate_input(&input) {
        Ok(v) => v,
        Err(r) => return r.into_response(),
    };
    let command = input.command.clone().filter(|c| !c.is_empty());

    let bookmark_id = match db::create_bookmark(
        &state.db_pool,
        db::NewBookmark {
            scope: db::BookmarkScope::Personal { user_id: user.id },
            alias: &alias,
            bookmark_type: &bookmark_type,
            url: &url,
            description: &description,
            command_template: command.as_deref(),
            created_by: Some(user.id),
        },
    )
    .await
    {
        Ok(id) => id,
        Err(e) if e.to_string().contains("UNIQUE constraint") => {
            return api_error(
                StatusCode::CONFLICT,
                &format!("alias '{}' already exists", alias),
            )
            .into_response();
        }
        Err(e) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to create bookmark: {}", e),
            )
            .into_response();
        }
    };

    if let Some(nested) = &input.nested {
        if let Err(r) = insert_nested(&state, bookmark_id, nested).await {
            return r.into_response();
        }
    }

    // Re-fetch for a canonical representation of what was stored.
    let all = match load_personal(&state, user.id).await {
        Ok(v) => v,
        Err(r) => return r.into_response(),
    };
    for (bookmark, nested) in all {
        if bookmark.id == bookmark_id {
            let body = to_json(bookmark, nested, (0, None));
            return (StatusCode::CREATED, Json(body)).into_response();
        }
    }
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "created bookmark could not be re-read",
    )
    .into_response()
}

/// PUT /api/bookmarks/{alias} — replace url/description/command;
/// when `nested` is present it replaces all sub-commands.
pub async fn update_bookmark(
    ApiUser(user): ApiUser,
    State(state): State<Arc<AppState>>,
    Path(alias): Path<String>,
    Json(input): Json<BookmarkInput>,
) -> Response {
    let all = match load_personal(&state, user.id).await {
        Ok(v) => v,
        Err(r) => return r.into_response(),
    };
    let existing = all.into_iter().find(|(b, _)| b.alias == alias);
    let Some((bookmark, _nested)) = existing else {
        return not_found(&alias).into_response();
    };

    let (url, description, _type) = match validate_input(&input) {
        Ok(v) => v,
        Err(r) => return r.into_response(),
    };
    let command = input.command.clone().filter(|c| !c.is_empty());

    if let Err(e) = db::update_bookmark(
        &state.db_pool,
        bookmark.id,
        db::BookmarkScope::Personal { user_id: user.id },
        &alias,
        &url,
        &description,
        command.as_deref(),
    )
    .await
    {
        return api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to update bookmark: {}", e),
        )
        .into_response();
    }

    if let Some(nested) = &input.nested {
        // Replace children wholesale.
        if let Ok(existing_children) = db::get_nested_bookmarks(&state.db_pool, bookmark.id).await {
            for child in existing_children {
                let _ = db::delete_nested_bookmark(&state.db_pool, child.id).await;
            }
        }
        if let Err(r) = insert_nested(&state, bookmark.id, nested).await {
            return r.into_response();
        }
    }

    get_bookmark(ApiUser(user), State(state), Path(alias)).await
}

/// DELETE /api/bookmarks/{alias} — 204 on success.
pub async fn delete_bookmark(
    ApiUser(user): ApiUser,
    State(state): State<Arc<AppState>>,
    Path(alias): Path<String>,
) -> Response {
    let all = match load_personal(&state, user.id).await {
        Ok(v) => v,
        Err(r) => return r.into_response(),
    };
    let existing = all.into_iter().find(|(b, _)| b.alias == alias);
    let Some((bookmark, _)) = existing else {
        return not_found(&alias).into_response();
    };
    match db::delete_bookmark(
        &state.db_pool,
        bookmark.id,
        db::BookmarkScope::Personal { user_id: user.id },
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to delete bookmark: {}", e),
        )
        .into_response(),
    }
}
