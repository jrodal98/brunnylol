-- Initial schema for multi-user bookmark management with variable template support
-- Single consolidated migration

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT 0,
    default_alias TEXT, -- User's preferred default alias
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

-- Unified bookmarks table (personal and global)
CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scope TEXT NOT NULL CHECK(scope IN ('personal', 'global')),
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE, -- NULL for global bookmarks
    alias TEXT NOT NULL,
    bookmark_type TEXT NOT NULL CHECK(bookmark_type IN ('simple', 'templated', 'nested')),
    url TEXT NOT NULL,
    description TEXT NOT NULL,
    command_template TEXT,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    variable_metadata TEXT, -- JSON metadata for RFC 6570-style template variables
    UNIQUE(scope, user_id, alias)
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_scope_user ON bookmarks(scope, user_id);
CREATE INDEX IF NOT EXISTS idx_bookmarks_alias ON bookmarks(alias);

-- Nested bookmarks table
CREATE TABLE IF NOT EXISTS nested_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_bookmark_id INTEGER NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT NOT NULL,
    command_template TEXT,
    display_order INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    variable_metadata TEXT, -- JSON metadata for RFC 6570-style template variables
    UNIQUE(parent_bookmark_id, alias)
);

CREATE INDEX IF NOT EXISTS idx_nested_bookmarks_parent ON nested_bookmarks(parent_bookmark_id);

-- User overrides for global bookmarks
CREATE TABLE IF NOT EXISTS user_bookmark_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    builtin_alias TEXT NOT NULL,
    is_disabled BOOLEAN NOT NULL DEFAULT 0,
    custom_alias TEXT,
    additional_aliases TEXT, -- JSON array
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
