-- Initial schema for multi-user bookmark management
-- Migration version: 001

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- Sessions table (for cookie-based authentication)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- User custom bookmarks
CREATE TABLE IF NOT EXISTS user_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    alias TEXT NOT NULL,
    bookmark_type TEXT NOT NULL CHECK(bookmark_type IN ('simple', 'templated', 'nested')),
    url TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    command_template TEXT,  -- For templated bookmarks (contains {})
    encode_query BOOLEAN DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, alias)
);

CREATE INDEX IF NOT EXISTS idx_user_bookmarks_user_id ON user_bookmarks(user_id);
CREATE INDEX IF NOT EXISTS idx_user_bookmarks_alias ON user_bookmarks(user_id, alias);

-- Nested bookmark sub-commands
CREATE TABLE IF NOT EXISTS nested_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_bookmark_id INTEGER NOT NULL,
    alias TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    command_template TEXT,
    encode_query BOOLEAN DEFAULT 1,
    display_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_bookmark_id) REFERENCES user_bookmarks(id) ON DELETE CASCADE,
    UNIQUE(parent_bookmark_id, alias)
);

CREATE INDEX IF NOT EXISTS idx_nested_bookmarks_parent ON nested_bookmarks(parent_bookmark_id);

-- User overrides for built-in bookmarks
-- Allows users to disable, rename, or add aliases to built-in bookmarks
CREATE TABLE IF NOT EXISTS user_bookmark_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    builtin_alias TEXT NOT NULL,  -- Original alias from commands.yml
    is_disabled BOOLEAN NOT NULL DEFAULT 0,
    custom_alias TEXT,  -- New alias (if renaming)
    additional_aliases TEXT,  -- JSON array of additional aliases
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, builtin_alias)
);

CREATE INDEX IF NOT EXISTS idx_user_bookmark_overrides_user_id ON user_bookmark_overrides(user_id);

-- Migration tracking
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Mark this migration as applied
INSERT OR IGNORE INTO schema_migrations (version) VALUES (1);
