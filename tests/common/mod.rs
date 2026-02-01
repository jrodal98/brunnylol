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

    // Run single consolidated migration
    let migration = include_str!("../../migrations/001_initial_schema.sql");
    sqlx::query(migration)
        .execute(&pool)
        .await
        .expect("Failed to run migration 001");

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

/// Create a test Axum router for integration tests
#[allow(dead_code)]
pub async fn create_test_app() -> axum::Router {
    // Use in-memory database for testing
    std::env::set_var("BRUNNYLOL_DB", ":memory:");
    brunnylol::create_router().await
}
