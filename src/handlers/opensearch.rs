// OpenSearch support: a descriptor document so browsers can register
// Brunnylol as a search engine automatically, plus a suggestions
// endpoint that completes ALIASES (built-in + the signed-in user's
// personal bookmarks) while typing in the address bar.
//
// Suggestions follow the OpenSearch suggestions format:
//   [query, [completions], [descriptions], [query URLs]]
// Alias matching uses the first whitespace-separated token of the
// query — that is the alias being typed; the rest of the query is
// search text, not part of the alias.

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{auth::middleware::OptionalUser, db, AppState};

/// Absolute origin (scheme://host) for this request, honoring the
/// reverse-proxy headers the production deployment sets.
pub fn request_origin(headers: &HeaderMap) -> String {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    format!("{}://{}", scheme, host)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// GET /opensearch.xml — the search-engine descriptor.
pub async fn descriptor(headers: HeaderMap) -> Response {
    let origin = request_origin(&headers);
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>Brunnylol</ShortName>
  <Description>Search and shortcuts with Brunnylol</Description>
  <InputEncoding>UTF-8</InputEncoding>
  <Url type="text/html" template="{origin}/search?q={{searchTerms}}"/>
  <Url type="application/x-suggestions+json" template="{origin}/api/suggest?q={{searchTerms}}"/>
</OpenSearchDescription>
"#,
        origin = xml_escape(&origin),
    );
    (
        StatusCode::OK,
        [("content-type", "application/opensearchdescription+xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct SuggestParams {
    q: Option<String>,
}

/// One suggestible alias.
struct Candidate {
    alias: String,
    description: String,
    personal: bool,
}

/// GET /api/suggest?q=... — alias completions in OpenSearch format.
/// Anonymous requests get built-in aliases; a session cookie adds the
/// user's personal bookmarks (and drops their disabled built-ins).
pub async fn suggest(
    OptionalUser(user): OptionalUser,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<SuggestParams>,
) -> Response {
    let query = params.q.unwrap_or_default();
    let partial = query
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();

    // Gather candidates: globals first, personal overrides/additions win.
    let mut by_alias: HashMap<String, Candidate> = HashMap::new();
    {
        let globals = state.alias_to_bookmark_map.read().await;
        for (alias, command) in globals.iter() {
            by_alias.insert(
                alias.clone(),
                Candidate {
                    alias: alias.clone(),
                    description: command.description().to_string(),
                    personal: false,
                },
            );
        }
    }

    if let Some(ref user) = user {
        let disabled = db::get_disabled_global_aliases(&state.db_pool, user.id).await;
        for alias in &disabled {
            by_alias.remove(alias);
        }
        if let Ok(personal) = db::bookmarks::load_user_bookmarks(&state.db_pool, user.id).await {
            for (alias, command) in personal {
                by_alias.insert(
                    alias.clone(),
                    Candidate {
                        alias,
                        description: command.description().to_string(),
                        personal: true,
                    },
                );
            }
        }
    }

    // Prefix matches rank first, then substring matches; personal
    // bookmarks before built-ins within a rank; alphabetical last.
    let mut prefix: Vec<&Candidate> = Vec::new();
    let mut substring: Vec<&Candidate> = Vec::new();
    for cand in by_alias.values() {
        let lower = cand.alias.to_lowercase();
        if partial.is_empty() || lower.starts_with(&partial) {
            prefix.push(cand);
        } else if lower.contains(&partial) {
            substring.push(cand);
        }
    }
    let rank = |a: &&Candidate, b: &&Candidate| {
        b.personal
            .cmp(&a.personal)
            .then_with(|| a.alias.to_lowercase().cmp(&b.alias.to_lowercase()))
    };
    prefix.sort_by(rank);
    substring.sort_by(rank);

    let origin = request_origin(&headers);
    let mut completions = Vec::new();
    let mut descriptions = Vec::new();
    let mut urls = Vec::new();
    for cand in prefix.into_iter().chain(substring).take(8) {
        completions.push(cand.alias.clone());
        descriptions.push(cand.description.clone());
        urls.push(format!(
            "{}/search?q={}",
            origin,
            urlencoding::encode(&cand.alias)
        ));
    }

    Json(serde_json::json!([query, completions, descriptions, urls])).into_response()
}
