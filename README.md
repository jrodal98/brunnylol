[![Rust](https://github.com/jrodal98/brunnylol/actions/workflows/rust.yml/badge.svg)](https://github.com/jrodal98/brunnylol/actions/workflows/rust.yml) [![Docker](https://github.com/jrodal98/brunnylol/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/jrodal98/brunnylol/actions/workflows/docker-publish.yml)

# brunnylol

Like bunnylol, but written in Rust.

A fast, multi-user bookmark management system with personalized shortcuts for quick web navigation.

## Features

- **Personal Bookmarks**: Create your own custom shortcuts
- **Global Bookmarks**: 40+ built-in shortcuts (Google, YouTube, GitHub, etc.)
- **Smart Search**: Simple, templated, and nested bookmark types
- **Import/Export**: YAML and JSON support with URL fetching
- **Multi-User**: Isolated bookmark collections with admin controls
- **Clean Architecture**: Service layer, strategy pattern, comprehensive testing

## Quick Start

### Running the Application

```bash
# Clone and build
git clone https://github.com/jrodal98/brunnylol.git
cd brunnylol
cargo build --release

# Run with defaults (port 8000, brunnylol.db)
cargo run --release

# Custom configuration
BRUNNYLOL_PORT=3000 BRUNNYLOL_DB=custom.db cargo run --release
```

### First Time Setup

1. Navigate to http://localhost:8000
2. Click "Register" (first user becomes admin automatically)
3. Create your account
4. Start using bookmarks at `/search?q=<alias> <query>`

### Example Usage

```
# Search Google
http://localhost:8000/search?q=g hello world

# Search YouTube
http://localhost:8000/search?q=yt rust programming

# Search GitHub
http://localhost:8000/search?q=gh axum web framework
```

## Usage

### Environment Variables

- `BRUNNYLOL_PORT` - Server port (default: 8000)
- `BRUNNYLOL_DB` - Database file path (default: brunnylol.db)

### Command Line Options

```bash
brunnylol --help

Options:
  -p, --port <PORT>      Port to listen on (default: 8000, env: BRUNNYLOL_PORT)
  -c, --commands <FILE>  Path to YAML commands file (deprecated, use import instead)
  -d, --database <DB>    Path to SQLite database (default: brunnylol.db, env: BRUNNYLOL_DB)
  -h, --help             Print help
```

### Managing Bookmarks

**Personal Bookmarks:**
1. Login to your account
2. Navigate to `/manage`
3. Create new bookmarks manually or import from YAML/JSON

**Global Bookmarks (Admin Only):**
1. Login as admin
2. Navigate to `/manage`
3. Import bookmarks with "Scope: Global Bookmarks"
4. All users will see your global bookmarks

**Import Bookmarks:**
- Paste YAML or JSON content
- Fetch from URL (e.g., https://example.com/bookmarks.yml)
- Choose Personal or Global scope (admin only)

**Export Bookmarks:**
- Download your personal bookmarks as YAML or JSON
- Admins can export global bookmarks
- Use for backup or sharing

## Testing

### Running Tests

```bash
# Run all tests (62 tests)
cargo test

# Run specific test suites
cargo test --test global_bookmarks_test  # Unit tests (16 tests)
cargo test --test e2e_test               # E2E tests (4 tests)
cargo test --test integration_test       # Integration tests (15 tests)
cargo test --test auth_test              # Auth tests (5 tests)

# Run with output
cargo test -- --nocapture

# Run in parallel (default)
cargo test

# Run sequentially (for debugging)
cargo test -- --test-threads=1
```

### Test Coverage

**Unit Tests (16):** `tests/global_bookmarks_test.rs`
- Service layer (import/export logic)
- Repository layer (database operations)
- Serializers (YAML/JSON conversion)
- Business rules (merging, precedence)
- Error handling

**E2E Tests (4):** `tests/e2e_test.rs`
- Complete HTTP workflows
- User registration/login
- Import/export via HTTP
- Permission enforcement
- Search redirects
- Multi-user scenarios

**Integration Tests (15):** `tests/integration_test.rs`
- Search functionality
- URL encoding
- Nested commands
- Help page

**Domain Tests (22):** Unit tests in `src/`
- BookmarkCommand, TemplatedCommand, NestedCommand
- URL encoding/decoding
- Command execution

**Auth Tests (5):** `tests/auth_test.rs`
- Login/registration flows
- Permission controls
- Admin-only pages

### Test Infrastructure

**E2E Test Features:**
- ✅ Automatic port allocation (finds available ports)
- ✅ Unique database per test (no conflicts)
- ✅ Runs in parallel (uses ports 8100+)
- ✅ Automatic binary selection (debug/release)
- ✅ Cookie-based session management
- ✅ Cleanup on test completion

**Example E2E Test:**
```rust
#[tokio::test]
async fn test_e2e_comprehensive() {
    let mut app = TestApp::start().await;  // Starts on unique port
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    // Test auto-seeding
    let global_count = app.db_query_count("SELECT COUNT(*) FROM global_bookmarks;");
    assert!(global_count >= 40);

    // Test import, export, permissions, etc.
    // ...

    app.stop().await;  // Cleanup
}
```

## Contributing

### Adding Features

1. **Understand the architecture** - Read existing code in `src/services/`, `src/handlers/`, `src/db/`
2. **Follow patterns** - Use existing code as templates
3. **Add tests** - Both unit tests and E2E tests required
4. **Update documentation** - Update README if user-facing

### Adding Unit Tests

Location: `tests/global_bookmarks_test.rs` (or create new file)

**When to add unit tests:**
- New service layer methods
- New database operations
- New business logic
- New validators or utilities

**Example:**
```rust
#[tokio::test]
async fn test_my_new_feature() {
    let pool = setup_test_db().await;
    let service = BookmarkService::new(pool.clone());

    // Test your feature
    let result = service.my_new_method().await.unwrap();

    // Assert expected behavior
    assert_eq!(result, expected_value);
}
```

### Adding E2E Tests

Location: `tests/e2e_test.rs`

**When to add E2E tests:**
- New HTTP endpoints
- New user workflows
- New UI features
- Integration between components

**Example:**
```rust
#[tokio::test]
async fn test_my_new_endpoint() {
    let mut app = TestApp::start().await;
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    // Login
    client.post(format!("{}/login", app.base_url))
        .form(&[("username", "admin"), ("password", "admin123")])
        .send()
        .await
        .unwrap();

    // Test your endpoint
    let response = client
        .get(format!("{}/my-endpoint", app.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    app.stop().await;
}
```

### Modifying Existing Tests

**When to modify tests:**
- Changing existing behavior
- Adding new fields to models
- Modifying database schema
- Changing API responses

**Example - Adding a field:**
```rust
// If you add a field to GlobalBookmark:
#[derive(Debug, Clone)]
pub struct GlobalBookmark {
    pub id: i64,
    pub alias: String,
    // ... existing fields ...
    pub new_field: String,  // NEW
}

// Update tests that create GlobalBookmark:
let bookmark = db::create_global_bookmark(
    &pool,
    "test",
    "simple",
    "https://test.com",
    "Test",
    None,
    true,
    None,
    "new_field_value",  // Add new parameter
).await.unwrap();

// Update assertions that check structure:
assert_eq!(bookmark.new_field, "expected_value");
```

### Test Guidelines

**Best Practices:**
1. **Test one thing** - Each test should verify a single behavior
2. **Descriptive names** - `test_import_duplicate_bookmarks_skips_correctly`
3. **Arrange-Act-Assert** - Setup, execute, verify
4. **Clean up** - Use `Drop` trait or cleanup functions
5. **Avoid flakiness** - Don't rely on timing, use proper waits
6. **Document complex tests** - Add comments explaining why

**Running Tests During Development:**
```bash
# Run specific test
cargo test test_import_personal_bookmarks_yaml

# Run tests matching pattern
cargo test import

# Watch mode (requires cargo-watch)
cargo watch -x test

# Show output for failed tests
cargo test -- --nocapture
```

### Database Migrations

When adding database schema changes:

1. Create new migration file: `migrations/00X_description.sql`
2. Add migration execution in `src/db/mod.rs` `init_db()`
3. Update struct definitions in `src/db/mod.rs`
4. Update `setup_test_db()` in all test files
5. Run tests to verify migration works

### Code Style

- Follow Rust conventions (rustfmt)
- Handle errors properly (use `Result<T, E>`)
- Use meaningful variable names
- Add documentation for public APIs
- Keep functions focused and small

### Pull Request Checklist

Before submitting:
- [ ] All tests pass (`cargo test`)
- [ ] No new compiler warnings
- [ ] Code formatted (`cargo fmt`)
- [ ] Added tests for new features
- [ ] Updated tests for modified features
- [ ] Updated README if user-facing changes
- [ ] Commit messages are descriptive

## Project Structure

```
brunnylol/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Router & app state
│   ├── domain/              # Business domain models
│   ├── db/                  # Database layer
│   ├── services/            # Business logic layer
│   │   ├── bookmark_service.rs
│   │   └── serializers.rs
│   ├── handlers/            # HTTP handlers
│   └── auth/                # Authentication
├── templates/               # Askama HTML templates
├── migrations/              # SQL migrations
├── tests/                   # Test files
│   ├── global_bookmarks_test.rs  # Unit tests
│   ├── e2e_test.rs              # E2E tests
│   ├── integration_test.rs       # Integration tests
│   └── auth_test.rs             # Auth tests
└── commands.yml             # Default global bookmarks
```

## License

See LICENSE file for details.

## Links

- Live Demo: [https://brunnylol.jrodal.com](https://brunnylol.jrodal.com)
- Help Page: [https://brunnylol.jrodal.com/help](https://brunnylol.jrodal.com/help)
