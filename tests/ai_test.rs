// Tests for AI configuration resolution, permissions, proposal parsing,
// and the propose flow against a mock OpenAI-compatible provider.

mod common;

use brunnylol::db;
use brunnylol::services::ai_service::{
    chat_completions_url, parse_proposal, resolve_effective_config_with_env, AiConfigSource,
    AiService, BookmarkProposal, EnvAiConfig, ProposeRequest, ResolvedAiConfig,
};

fn env(base: Option<&str>, key: Option<&str>, model: Option<&str>) -> EnvAiConfig {
    EnvAiConfig {
        base_url: base.map(|s| s.to_string()),
        api_key: key.map(|s| s.to_string()),
        model: model.map(|s| s.to_string()),
    }
}

async fn make_users(pool: &sqlx::SqlitePool) -> (db::User, db::User) {
    // First user created is admin (mirrors production registration).
    let admin = db::create_user(pool, "aiadmin", "hash").await.unwrap();
    let regular = db::create_user(pool, "airegular", "hash").await.unwrap();
    assert!(admin.is_admin);
    assert!(!regular.is_admin);
    (admin, regular)
}

async fn set_global(
    pool: &sqlx::SqlitePool,
    base: Option<&str>,
    key: Option<&str>,
    model: Option<&str>,
    mode: &str,
) {
    db::ai::upsert_global_ai_settings(
        pool,
        &db::ai::GlobalAiSettings {
            base_url: base.map(|s| s.to_string()),
            api_key: key.map(|s| s.to_string()),
            model: model.map(|s| s.to_string()),
            access_mode: mode.to_string(),
        },
    )
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------
// URL building
// ---------------------------------------------------------------------------

#[test]
fn test_chat_completions_url_shapes() {
    assert_eq!(
        chat_completions_url("https://llm.example.com"),
        "https://llm.example.com/chat/completions"
    );
    assert_eq!(
        chat_completions_url("https://api.openai.com/v1"),
        "https://api.openai.com/v1/chat/completions"
    );
    assert_eq!(
        chat_completions_url("https://llm.example.com/chat/completions"),
        "https://llm.example.com/chat/completions"
    );
    assert_eq!(
        chat_completions_url("https://llm.example.com/v1/"),
        "https://llm.example.com/v1/chat/completions"
    );
}

// ---------------------------------------------------------------------------
// Proposal parsing / validation
// ---------------------------------------------------------------------------

#[test]
fn test_parse_proposal_plain_json() {
    let proposal = parse_proposal(
        r#"{"alias":"hn","bookmark_type":"templated","url":"https://news.ycombinator.com","description":"Search Hacker News","command_template":"{url}/search?q={query}"}"#,
    )
    .unwrap();
    assert_eq!(proposal.alias, "hn");
    assert_eq!(
        proposal.command_template.as_deref(),
        Some("{url}/search?q={query}")
    );
}

#[test]
fn test_parse_proposal_fenced_json_with_prose() {
    let proposal = parse_proposal(
        "Here you go:\n```json\n{\"alias\":\"gh\",\"bookmark_type\":\"templated\",\"url\":\"https://github.com\",\"description\":\"GitHub repo\",\"command_template\":\"{url}/{user}/{repo}\"}\n```",
    )
    .unwrap();
    assert_eq!(proposal.alias, "gh");
}

#[test]
fn test_parse_proposal_rejects_bad_url_and_template() {
    let bad_url = parse_proposal(
        r#"{"alias":"x","bookmark_type":"templated","url":"javascript:alert(1)","description":"bad"}"#,
    );
    assert!(bad_url.is_err());

    let bad_template = parse_proposal(
        r#"{"alias":"x","bookmark_type":"templated","url":"https://example.com","description":"bad","command_template":"{url}/{bad name}"}"#,
    );
    assert!(bad_template.is_err());

    let spaced_alias = parse_proposal(
        r#"{"alias":"a b","bookmark_type":"templated","url":"https://example.com","description":"bad"}"#,
    );
    assert!(spaced_alias.is_err());
}

#[test]
fn test_parse_proposal_nested() {
    let proposal = parse_proposal(
        r#"{"alias":"dev","bookmark_type":"nested","url":"https://github.com","description":"Dev repos","command_template":null,"nested_commands":[{"alias":"fe","url":"https://github.com","description":"Frontend","command_template":"{url}/{user}/{repo}"}]}"#,
    )
    .unwrap();
    assert_eq!(proposal.bookmark_type, "nested");
    assert_eq!(proposal.nested_commands.len(), 1);

    let empty_nested = parse_proposal(
        r#"{"alias":"dev","bookmark_type":"nested","url":"","description":"Dev","nested_commands":[]}"#,
    );
    assert!(empty_nested.is_err());
}

// ---------------------------------------------------------------------------
// Config resolution + permissions
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_shared_config_admins_only() {
    let pool = common::setup_test_db().await;
    let (admin, regular) = make_users(&pool).await;
    set_global(
        &pool,
        Some("https://llm.example.com"),
        Some("sk-shared"),
        Some("m1"),
        "admins_only",
    )
    .await;

    let admin_cfg = resolve_effective_config_with_env(&pool, &admin, &env(None, None, None)).await;
    assert!(admin_cfg.is_some());
    assert_eq!(admin_cfg.unwrap().source, AiConfigSource::Shared);

    let regular_cfg =
        resolve_effective_config_with_env(&pool, &regular, &env(None, None, None)).await;
    assert!(regular_cfg.is_none());
}

#[tokio::test]
async fn test_shared_config_all_users_and_disabled() {
    let pool = common::setup_test_db().await;
    let (_admin, regular) = make_users(&pool).await;
    set_global(
        &pool,
        Some("https://llm.example.com"),
        Some("sk-shared"),
        Some("m1"),
        "all_users",
    )
    .await;
    assert!(
        resolve_effective_config_with_env(&pool, &regular, &env(None, None, None))
            .await
            .is_some()
    );

    set_global(
        &pool,
        Some("https://llm.example.com"),
        Some("sk-shared"),
        Some("m1"),
        "disabled",
    )
    .await;
    assert!(
        resolve_effective_config_with_env(&pool, &regular, &env(None, None, None))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn test_shared_config_selected_users() {
    let pool = common::setup_test_db().await;
    let (_admin, regular) = make_users(&pool).await;
    let other = db::create_user(&pool, "aiother", "hash").await.unwrap();
    set_global(
        &pool,
        Some("https://llm.example.com"),
        Some("sk-shared"),
        Some("m1"),
        "selected_users",
    )
    .await;

    assert!(
        resolve_effective_config_with_env(&pool, &regular, &env(None, None, None))
            .await
            .is_none()
    );

    db::ai::set_allowed_users(&pool, &[regular.id])
        .await
        .unwrap();
    assert!(
        resolve_effective_config_with_env(&pool, &regular, &env(None, None, None))
            .await
            .is_some()
    );
    assert!(
        resolve_effective_config_with_env(&pool, &other, &env(None, None, None))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn test_env_fallback_fills_blank_global_fields() {
    let pool = common::setup_test_db().await;
    let (admin, _regular) = make_users(&pool).await;
    // Global row exists but all provider fields blank -> env fills them.
    set_global(&pool, None, None, None, "admins_only").await;

    let cfg = resolve_effective_config_with_env(
        &pool,
        &admin,
        &env(
            Some("https://env.example.com"),
            Some("sk-env"),
            Some("env-model"),
        ),
    )
    .await
    .expect("env fallback should produce a config");
    assert_eq!(cfg.base_url, "https://env.example.com");
    assert_eq!(cfg.api_key, "sk-env");
    assert_eq!(cfg.model, "env-model");

    // Partial env is not a usable config.
    let none_cfg = resolve_effective_config_with_env(
        &pool,
        &admin,
        &env(Some("https://env.example.com"), None, None),
    )
    .await;
    assert!(none_cfg.is_none());
}

#[tokio::test]
async fn test_personal_config_wins_and_opt_out() {
    let pool = common::setup_test_db().await;
    let (_admin, regular) = make_users(&pool).await;
    // Shared is disabled...
    set_global(
        &pool,
        Some("https://shared.example.com"),
        Some("sk-shared"),
        Some("shared-model"),
        "disabled",
    )
    .await;

    // ...but a complete personal config still works for its owner.
    db::ai::upsert_user_ai_settings(
        &pool,
        regular.id,
        &db::ai::UserAiSettings {
            base_url: Some("https://personal.example.com".to_string()),
            api_key: Some("sk-personal".to_string()),
            model: Some("personal-model".to_string()),
            enabled: true,
        },
    )
    .await
    .unwrap();
    let cfg = resolve_effective_config_with_env(&pool, &regular, &env(None, None, None))
        .await
        .expect("personal config should resolve");
    assert_eq!(cfg.source, AiConfigSource::Personal);
    assert_eq!(cfg.model, "personal-model");

    // Partial personal config falls through to shared (which is disabled -> none).
    db::ai::upsert_user_ai_settings(
        &pool,
        regular.id,
        &db::ai::UserAiSettings {
            base_url: Some("https://personal.example.com".to_string()),
            api_key: None,
            model: None,
            enabled: true,
        },
    )
    .await
    .unwrap();
    assert!(
        resolve_effective_config_with_env(&pool, &regular, &env(None, None, None))
            .await
            .is_none()
    );

    // Explicit opt-out disables AI even with a complete personal config
    // and an all-users shared config.
    set_global(
        &pool,
        Some("https://shared.example.com"),
        Some("sk-shared"),
        Some("shared-model"),
        "all_users",
    )
    .await;
    db::ai::upsert_user_ai_settings(
        &pool,
        regular.id,
        &db::ai::UserAiSettings {
            base_url: Some("https://personal.example.com".to_string()),
            api_key: Some("sk-personal".to_string()),
            model: Some("personal-model".to_string()),
            enabled: false,
        },
    )
    .await
    .unwrap();
    assert!(
        resolve_effective_config_with_env(&pool, &regular, &env(None, None, None))
            .await
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// Propose against a mock OpenAI-compatible provider
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_propose_with_mock_provider_and_follow_up() {
    use axum::{routing::post, Json, Router};

    // Mock provider: echoes a fixed proposal as chat completion content.
    let proposal_json = r#"{"alias":"hn","bookmark_type":"templated","url":"https://news.ycombinator.com","description":"Search Hacker News","command_template":"{url}/search?q={query}","nested_commands":[]}"#;
    let content = proposal_json.to_string();
    let mock = Router::new().route(
        "/chat/completions",
        post(move || {
            let content = content.clone();
            async move {
                Json(serde_json::json!({
                    "id": "chatcmpl-test",
                    "object": "chat.completion",
                    "choices": [{
                        "index": 0,
                        "message": {"role": "assistant", "content": content},
                        "finish_reason": "stop"
                    }]
                }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, mock).await.unwrap();
    });

    let config = ResolvedAiConfig {
        base_url: format!("http://{}", addr),
        api_key: "sk-test".to_string(),
        model: "mock-model".to_string(),
        source: AiConfigSource::Personal,
    };
    let service = AiService::new();

    let first = service
        .propose_with_config(
            &config,
            &ProposeRequest {
                message: "a shortcut to search Hacker News".to_string(),
                history: vec![],
                previous_proposal: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(first.proposal.alias, "hn");
    assert_eq!(first.model, "mock-model");
    assert_eq!(
        first.attempts, 1,
        "valid first proposal should take one attempt"
    );

    // Follow-up shape: history + previous proposal are accepted.
    let previous: BookmarkProposal = serde_json::from_str(proposal_json).unwrap();
    let follow_up = service
        .propose_with_config(
            &config,
            &ProposeRequest {
                message: "actually keep the same alias".to_string(),
                history: vec![
                    brunnylol::services::ai_service::ChatTurn {
                        role: "user".to_string(),
                        content: "a shortcut to search Hacker News".to_string(),
                    },
                    brunnylol::services::ai_service::ChatTurn {
                        role: "assistant".to_string(),
                        content: proposal_json.to_string(),
                    },
                ],
                previous_proposal: Some(previous),
            },
        )
        .await
        .unwrap();
    assert_eq!(follow_up.proposal.alias, "hn");
}

// ---------------------------------------------------------------------------
// Validation retry loop: invalid proposals are fed back to the model
// ---------------------------------------------------------------------------

/// Spawn a mock provider that returns `contents` in order (repeating the
/// last one), counting how many times it was called.
async fn spawn_sequence_mock(
    contents: Vec<String>,
) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use axum::{extract::State as AxumState, routing::post, Json, Router};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Clone)]
    struct MockState {
        calls: Arc<AtomicUsize>,
        contents: Arc<Vec<String>>,
    }

    async fn handler(AxumState(st): AxumState<MockState>) -> Json<serde_json::Value> {
        let i = st.calls.fetch_add(1, Ordering::SeqCst);
        let idx = i.min(st.contents.len() - 1);
        let content = st.contents[idx].clone();
        Json(serde_json::json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": content},
                "finish_reason": "stop"
            }]
        }))
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let state = MockState {
        calls: calls.clone(),
        contents: Arc::new(contents),
    };
    let mock = Router::new()
        .route("/chat/completions", post(handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, mock).await.unwrap();
    });
    (format!("http://{}", addr), calls)
}

fn test_config(base_url: String) -> ResolvedAiConfig {
    ResolvedAiConfig {
        base_url,
        api_key: "sk-test".to_string(),
        model: "mock-model".to_string(),
        source: AiConfigSource::Personal,
    }
}

const VALID_PROPOSAL: &str = r#"{"alias":"hn","bookmark_type":"templated","url":"https://news.ycombinator.com","description":"Search Hacker News","command_template":"{url}/search?q={query}","nested_commands":[]}"#;
const INVALID_PROPOSAL: &str = r#"{"alias":"hn","bookmark_type":"templated","url":"javascript:alert(1)","description":"Search Hacker News","command_template":"{url}/search?q={query}","nested_commands":[]}"#;

#[tokio::test]
async fn test_propose_retries_after_invalid_proposal() {
    use std::sync::atomic::Ordering;

    // First response fails validation (bad URL scheme), second is valid.
    let (base_url, calls) = spawn_sequence_mock(vec![
        INVALID_PROPOSAL.to_string(),
        VALID_PROPOSAL.to_string(),
    ])
    .await;

    let outcome = AiService::new()
        .propose_with_config(
            &test_config(base_url),
            &ProposeRequest {
                message: "a shortcut to search Hacker News".to_string(),
                history: vec![],
                previous_proposal: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(outcome.proposal.alias, "hn");
    assert_eq!(outcome.attempts, 2);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "model should be called twice"
    );
}

#[tokio::test]
async fn test_propose_retries_after_unparseable_response() {
    use std::sync::atomic::Ordering;

    // First response is not JSON at all, second is a valid proposal.
    let (base_url, calls) = spawn_sequence_mock(vec![
        "Sure! Here is your bookmark: hn -> Hacker News".to_string(),
        VALID_PROPOSAL.to_string(),
    ])
    .await;

    let outcome = AiService::new()
        .propose_with_config(
            &test_config(base_url),
            &ProposeRequest {
                message: "a shortcut to search Hacker News".to_string(),
                history: vec![],
                previous_proposal: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(outcome.attempts, 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_propose_gives_up_after_three_invalid_attempts() {
    use std::sync::atomic::Ordering;

    let (base_url, calls) = spawn_sequence_mock(vec![INVALID_PROPOSAL.to_string()]).await;

    let result = AiService::new()
        .propose_with_config(
            &test_config(base_url),
            &ProposeRequest {
                message: "a shortcut to search Hacker News".to_string(),
                history: vec![],
                previous_proposal: None,
            },
        )
        .await;

    let err = result.expect_err("always-invalid proposals must fail");
    assert!(
        err.to_string().contains("after 3 attempts"),
        "error should mention the attempt limit, got: {}",
        err
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "model should be called exactly 3 times"
    );
}
