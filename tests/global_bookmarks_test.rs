// Integration tests for global bookmarks feature

// Helper to create test database
async fn setup_test_db() -> sqlx::SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("Failed to enable foreign keys");

    // Run migrations
    let migration_1 = include_str!("../migrations/001_initial_schema.sql");
    sqlx::query(migration_1)
        .execute(&pool)
        .await
        .expect("Failed to run migration 001");

    let migration_2 = include_str!("../migrations/002_global_bookmarks.sql");
    sqlx::query(migration_2)
        .execute(&pool)
        .await
        .expect("Failed to run migration 002");

    pool
}

// Helper to create admin user
async fn create_admin_user(pool: &sqlx::SqlitePool) -> (i64, String) {
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

#[tokio::test]
async fn test_global_bookmarks_auto_seed() {
    let pool = setup_test_db().await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    // Verify table is empty
    let is_empty = brunnylol::db::is_global_bookmarks_empty(&pool)
        .await
        .unwrap();
    assert!(is_empty, "Global bookmarks table should be empty initially");

    // Seed from embedded commands.yml
    let count = service.seed_global_bookmarks().await.unwrap();

    assert!(count > 30, "Should seed at least 30 bookmarks from commands.yml");

    // Verify table is no longer empty
    let is_empty_after = brunnylol::db::is_global_bookmarks_empty(&pool)
        .await
        .unwrap();
    assert!(!is_empty_after, "Global bookmarks table should have data after seeding");

    // Verify specific bookmarks exist
    let bookmarks = brunnylol::db::get_all_global_bookmarks(&pool)
        .await
        .unwrap();

    let aliases: Vec<String> = bookmarks.iter().map(|b| b.alias.clone()).collect();
    assert!(aliases.contains(&"g".to_string()), "Should have Google bookmark");
    assert!(aliases.contains(&"ddg".to_string()), "Should have DuckDuckGo bookmark");
    assert!(aliases.contains(&"yt".to_string()), "Should have YouTube bookmark");
}

#[tokio::test]
async fn test_import_personal_bookmarks_yaml() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    let yaml = r#"
- alias: test1
  url: https://test1.com
  description: Test bookmark 1
  command: https://test1.com/search?q={}
- alias: test2
  url: https://test2.com
  description: Test bookmark 2
"#;

    let serializer = brunnylol::services::serializers::YamlSerializer;
    let result = service.import_bookmarks(
        yaml,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await.unwrap();

    assert_eq!(result.imported, 2, "Should import 2 bookmarks");
    assert_eq!(result.skipped, 0, "Should skip 0 duplicates");
    assert_eq!(result.errors.len(), 0, "Should have 0 errors");

    // Verify in database
    let bookmarks = brunnylol::db::get_user_bookmarks(&pool, user_id)
        .await
        .unwrap();

    assert_eq!(bookmarks.len(), 2);
    assert_eq!(bookmarks[0].alias, "test1");
    assert_eq!(bookmarks[0].bookmark_type, "templated");
    assert_eq!(bookmarks[1].alias, "test2");
    assert_eq!(bookmarks[1].bookmark_type, "simple");
}

#[tokio::test]
async fn test_import_personal_bookmarks_json() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    let json = r#"[
  {
    "alias": "jsontest",
    "url": "https://json.test",
    "description": "JSON test",
    "command": null,
    "encode": true,
    "nested": null
  }
]"#;

    let serializer = brunnylol::services::serializers::JsonSerializer;
    let result = service.import_bookmarks(
        json,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await.unwrap();

    assert_eq!(result.imported, 1);
    assert_eq!(result.errors.len(), 0);
}

#[tokio::test]
async fn test_import_global_bookmarks_admin_only() {
    let pool = setup_test_db().await;
    let (admin_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    let yaml = r#"
- alias: global1
  url: https://global1.com
  description: Global bookmark 1
  command: https://global1.com/search?q={}
"#;

    let serializer = brunnylol::services::serializers::YamlSerializer;
    let result = service.import_bookmarks(
        yaml,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Global,
        Some(admin_id),
    ).await.unwrap();

    assert_eq!(result.imported, 1);

    // Verify in global_bookmarks table
    let bookmarks = brunnylol::db::get_all_global_bookmarks(&pool)
        .await
        .unwrap();

    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].alias, "global1");
    assert_eq!(bookmarks[0].created_by, Some(admin_id));
}

#[tokio::test]
async fn test_import_nested_bookmarks() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    let yaml = r#"
- alias: parent
  url: https://parent.com
  description: Parent bookmark
  nested:
    - alias: child1
      url: https://parent.com/child1
      description: Child 1
      command: https://parent.com/child1?q={}
    - alias: child2
      url: https://parent.com/child2
      description: Child 2
"#;

    let serializer = brunnylol::services::serializers::YamlSerializer;
    let result = service.import_bookmarks(
        yaml,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await.unwrap();

    assert_eq!(result.imported, 1);

    // Verify parent bookmark
    let bookmarks = brunnylol::db::get_user_bookmarks(&pool, user_id)
        .await
        .unwrap();
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].alias, "parent");
    assert_eq!(bookmarks[0].bookmark_type, "nested");

    // Verify nested bookmarks
    let nested = brunnylol::db::get_nested_bookmarks(&pool, bookmarks[0].id)
        .await
        .unwrap();
    assert_eq!(nested.len(), 2);
    assert_eq!(nested[0].alias, "child1");
    assert_eq!(nested[1].alias, "child2");
}

#[tokio::test]
async fn test_export_personal_bookmarks_yaml() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;

    // Create some bookmarks
    brunnylol::db::create_bookmark(
        &pool,
        user_id,
        "export1",
        "simple",
        "https://export1.com",
        "Export test 1",
        None,
        true,
    ).await.unwrap();

    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());
    let serializer = brunnylol::services::serializers::YamlSerializer;

    let yaml = service.export_bookmarks(
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
        &serializer,
    ).await.unwrap();

    assert!(yaml.contains("alias: export1"));
    assert!(yaml.contains("url: https://export1.com"));
    assert!(yaml.contains("description: Export test 1"));
}

#[tokio::test]
async fn test_export_global_bookmarks_json() {
    let pool = setup_test_db().await;
    let (admin_id, _) = create_admin_user(&pool).await;

    // Create a global bookmark
    brunnylol::db::create_global_bookmark(
        &pool,
        "gtest",
        "templated",
        "https://gtest.com",
        "Global test",
        Some("https://gtest.com/search?q={}"),
        true,
        Some(admin_id),
    ).await.unwrap();

    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());
    let serializer = brunnylol::services::serializers::JsonSerializer;

    let json = service.export_bookmarks(
        brunnylol::services::bookmark_service::BookmarkScope::Global,
        Some(admin_id),
        &serializer,
    ).await.unwrap();

    assert!(json.contains("\"alias\": \"gtest\""));
    assert!(json.contains("\"url\": \"https://gtest.com\""));
}

#[tokio::test]
async fn test_export_global_nested_bookmarks() {
    let pool = setup_test_db().await;
    let (admin_id, _) = create_admin_user(&pool).await;

    // Create nested global bookmark
    let parent_id = brunnylol::db::create_global_bookmark(
        &pool,
        "gnest",
        "nested",
        "https://gnest.com",
        "Global nested test",
        None,
        true,
        Some(admin_id),
    ).await.unwrap();

    brunnylol::db::create_global_nested_bookmark(
        &pool,
        parent_id,
        "sub",
        "https://gnest.com/sub",
        "Sub command",
        Some("https://gnest.com/sub?q={}"),
        true,
        0,
    ).await.unwrap();

    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());
    let serializer = brunnylol::services::serializers::YamlSerializer;

    let yaml = service.export_bookmarks(
        brunnylol::services::bookmark_service::BookmarkScope::Global,
        Some(admin_id),
        &serializer,
    ).await.unwrap();

    // Verify nested structure is exported
    assert!(yaml.contains("alias: gnest"));
    assert!(yaml.contains("nested:"));
    assert!(yaml.contains("alias: sub"));
}

#[tokio::test]
async fn test_duplicate_detection() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    let yaml = r#"
- alias: dup
  url: https://dup.com
  description: Duplicate test
"#;

    let serializer = brunnylol::services::serializers::YamlSerializer;

    // Import first time
    let result1 = service.import_bookmarks(
        yaml,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await.unwrap();

    assert_eq!(result1.imported, 1);
    assert_eq!(result1.skipped, 0);

    // Import again - should skip duplicate
    let result2 = service.import_bookmarks(
        yaml,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await.unwrap();

    assert_eq!(result2.imported, 0);
    assert_eq!(result2.skipped, 1);
}

#[tokio::test]
async fn test_personal_overrides_global() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    // Create global bookmark
    brunnylol::db::create_global_bookmark(
        &pool,
        "override-test",
        "simple",
        "https://global.com",
        "Global version",
        None,
        true,
        None,
    ).await.unwrap();

    // Create personal bookmark with same alias
    brunnylol::db::create_bookmark(
        &pool,
        user_id,
        "override-test",
        "simple",
        "https://personal.com",
        "Personal version",
        None,
        true,
    ).await.unwrap();

    // Load user bookmarks (should include personal, not global for conflicting alias)
    let bookmarks = service.load_user_bookmarks(user_id).await.unwrap();

    assert!(bookmarks.contains_key("override-test"));

    let command = bookmarks.get("override-test").unwrap();
    let redirect_url = command.get_redirect_url("");

    // Should redirect to personal URL, not global
    assert_eq!(redirect_url, "https://personal.com");
}

#[tokio::test]
async fn test_disabled_global_bookmarks() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    // Create global bookmark
    brunnylol::db::create_global_bookmark(
        &pool,
        "disable-test",
        "simple",
        "https://disabled.com",
        "Should be disabled",
        None,
        true,
        None,
    ).await.unwrap();

    // Disable it for user
    brunnylol::db::upsert_override(
        &pool,
        user_id,
        "disable-test",
        true, // is_disabled
        None,
        None,
    ).await.unwrap();

    // Load user bookmarks - should not include disabled global
    let bookmarks = service.load_user_bookmarks(user_id).await.unwrap();

    assert!(!bookmarks.contains_key("disable-test"),
           "Disabled global bookmark should not appear in user bookmarks");
}

#[tokio::test]
async fn test_round_trip_export_import() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    // Create bookmarks
    let yaml_import = r#"
- alias: roundtrip1
  url: https://rt1.com
  description: Round trip test 1
  command: https://rt1.com/search?q={}
- alias: roundtrip2
  url: https://rt2.com
  description: Round trip test 2
  nested:
    - alias: sub1
      url: https://rt2.com/sub1
      description: Sub 1
"#;

    let serializer = brunnylol::services::serializers::YamlSerializer;

    // Import
    let import_result = service.import_bookmarks(
        yaml_import,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await.unwrap();

    assert_eq!(import_result.imported, 2);

    // Export
    let exported = service.export_bookmarks(
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
        &serializer,
    ).await.unwrap();

    // Verify exported content contains both bookmarks
    assert!(exported.contains("roundtrip1"));
    assert!(exported.contains("roundtrip2"));
    assert!(exported.contains("sub1"));

    // Re-import should skip duplicates
    let reimport_result = service.import_bookmarks(
        &exported,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await.unwrap();

    assert_eq!(reimport_result.imported, 0);
    assert_eq!(reimport_result.skipped, 2);
}

#[tokio::test]
async fn test_serializer_yaml_json_equivalence() {
    use brunnylol::config::yml_settings::YmlSettings;
    use brunnylol::services::serializers::{YamlSerializer, JsonSerializer, BookmarkSerializer};

    let bookmarks = vec![
        YmlSettings {
            alias: "test".to_string(),
            url: "https://test.com".to_string(),
            description: "Test".to_string(),
            command: Some("https://test.com/search?q={}".to_string()),
            encode: Some(true),
            nested: None,
        }
    ];

    let yaml_serializer = YamlSerializer;
    let json_serializer = JsonSerializer;

    // Serialize to YAML
    let yaml = yaml_serializer.serialize(&bookmarks).unwrap();
    assert!(yaml.contains("alias: test"));

    // Serialize to JSON
    let json = json_serializer.serialize(&bookmarks).unwrap();
    assert!(json.contains("\"alias\": \"test\""));

    // Deserialize both
    let from_yaml = yaml_serializer.deserialize(&yaml).unwrap();
    let from_json = json_serializer.deserialize(&json).unwrap();

    assert_eq!(from_yaml.len(), 1);
    assert_eq!(from_json.len(), 1);
    assert_eq!(from_yaml[0].alias, "test");
    assert_eq!(from_json[0].alias, "test");
}

#[tokio::test]
async fn test_load_global_bookmarks_as_commands() {
    let pool = setup_test_db().await;
    let (admin_id, _) = create_admin_user(&pool).await;

    // Create test global bookmarks
    brunnylol::db::create_global_bookmark(
        &pool,
        "loadtest",
        "templated",
        "https://loadtest.com",
        "Load test",
        Some("https://loadtest.com/search?q={}"),
        true,
        Some(admin_id),
    ).await.unwrap();

    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());
    let commands = service.load_global_bookmarks().await.unwrap();

    assert!(commands.contains_key("loadtest"));

    let command = commands.get("loadtest").unwrap();
    let redirect_url = command.get_redirect_url("testquery");

    assert_eq!(redirect_url, "https://loadtest.com/search?q=testquery");
}

#[tokio::test]
async fn test_merge_global_and_personal_bookmarks() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    // Create global bookmark
    brunnylol::db::create_global_bookmark(
        &pool,
        "merge-global",
        "simple",
        "https://global-merge.com",
        "Global merge test",
        None,
        true,
        None,
    ).await.unwrap();

    // Create personal bookmark
    brunnylol::db::create_bookmark(
        &pool,
        user_id,
        "merge-personal",
        "simple",
        "https://personal-merge.com",
        "Personal merge test",
        None,
        true,
    ).await.unwrap();

    // Load merged bookmarks
    let bookmarks = service.load_user_bookmarks(user_id).await.unwrap();

    // Should contain both
    assert!(bookmarks.contains_key("merge-global"));
    assert!(bookmarks.contains_key("merge-personal"));

    assert_eq!(bookmarks.get("merge-global").unwrap().get_redirect_url(""), "https://global-merge.com");
    assert_eq!(bookmarks.get("merge-personal").unwrap().get_redirect_url(""), "https://personal-merge.com");
}

#[tokio::test]
async fn test_import_with_errors() {
    let pool = setup_test_db().await;
    let (user_id, _) = create_admin_user(&pool).await;
    let service = brunnylol::services::bookmark_service::BookmarkService::new(pool.clone());

    // Test with invalid YAML
    let invalid_yaml = r#"
- alias: valid1
  url: https://valid1.com
  description: Valid bookmark
- this is invalid yaml that will fail to parse
"#;

    let serializer = brunnylol::services::serializers::YamlSerializer;
    let result = service.import_bookmarks(
        invalid_yaml,
        &serializer,
        brunnylol::services::bookmark_service::BookmarkScope::Personal,
        Some(user_id),
    ).await;

    // Should fail to parse
    assert!(result.is_err(), "Invalid YAML should return an error");
}
