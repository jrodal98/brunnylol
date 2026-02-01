// Database seeding for development/testing

use anyhow::Result;
use sqlx::SqlitePool;

use crate::auth;
use crate::db;

pub async fn seed_test_user(pool: &SqlitePool) -> Result<()> {
    // Check if admin user already exists
    if db::get_user_by_username(pool, "admin").await?.is_some() {
        println!("Test admin user already exists");
        return Ok(());
    }

    // Create test admin user
    let password_hash = auth::hash_password("admin123")?;
    let user = db::create_user(pool, "admin", &password_hash).await?;

    println!("Created test admin user: {} (id: {})", user.username, user.id);

    Ok(())
}
