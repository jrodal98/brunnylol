// Common test utilities shared across test files

use sqlx::SqlitePool;

/// Set up an in-memory SQLite database for testing
#[allow(dead_code)]
pub async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory database");

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("Failed to enable foreign keys");

    // Run migrations manually (sqlx::migrate! doesn't work well with in-memory databases in tests)
    let migration_1 = include_str!("../../migrations/001_initial_schema.sql");
    sqlx::query(migration_1)
        .execute(&pool)
        .await
        .expect("Failed to run migration 001");

    let migration_2 = include_str!("../../migrations/002_global_bookmarks.sql");
    sqlx::query(migration_2)
        .execute(&pool)
        .await
        .expect("Failed to run migration 002");

    let migration_3 = include_str!("../../migrations/003_user_default_alias.sql");
    sqlx::query(migration_3)
        .execute(&pool)
        .await
        .expect("Failed to run migration 003");

    let migration_4 = include_str!("../../migrations/004_consolidate_bookmarks.sql");
    sqlx::query(migration_4)
        .execute(&pool)
        .await
        .expect("Failed to run migration 004");

    pool
}

/// Create an admin user for testing
/// Returns (user_id, session_token)
#[allow(dead_code)]
pub async fn create_admin_user(pool: &SqlitePool) -> (i64, String) {
    let password_hash = bcrypt::hash("testpass123", bcrypt::DEFAULT_COST).unwrap();

    // Use the db::create_user function which handles admin logic
    let user = brunnylol::db::create_user(pool, "testadmin", &password_hash)
        .await
        .unwrap();

    // Create session
    let session_id = brunnylol::db::create_session(pool, user.id)
        .await
        .unwrap();

    (user.id, session_id)
}

/// Create a regular (non-admin) user for testing
/// Returns (user_id, session_token)
#[allow(dead_code)]
pub async fn create_regular_user(pool: &SqlitePool, username: &str) -> (i64, String) {
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();

    let user_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO users (username, password_hash, is_admin) VALUES (?, ?, 0) RETURNING id"
    )
    .bind(username)
    .bind(&password_hash)
    .fetch_one(pool)
    .await
    .expect("Failed to create regular user");

    // Create session
    let session_token = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sessions (user_id, session_token, expires_at) VALUES (?, ?, datetime('now', '+7 days'))"
    )
    .bind(user_id)
    .bind(&session_token)
    .fetch_optional(pool)
    .await
    .expect("Failed to create session");

    (user_id, session_token)
}

/// Create a test Axum router for integration tests
#[allow(dead_code)]
pub async fn create_test_app() -> axum::Router {
    // Use in-memory database for testing
    std::env::set_var("BRUNNYLOL_DB", ":memory:");
    brunnylol::create_router().await
}
