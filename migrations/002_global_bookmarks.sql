-- Global bookmarks migration
-- Migration version: 002

-- Global bookmarks table (admin-managed, shared across all users)
CREATE TABLE IF NOT EXISTS global_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    alias TEXT UNIQUE NOT NULL,
    bookmark_type TEXT NOT NULL CHECK(bookmark_type IN ('simple', 'templated', 'nested')),
    url TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    command_template TEXT,
    encode_query BOOLEAN DEFAULT 1,
    created_by INTEGER,  -- NULL for seeded bookmarks, admin user_id for user-created
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_global_bookmarks_alias ON global_bookmarks(alias);

-- Nested bookmark sub-commands for global bookmarks
CREATE TABLE IF NOT EXISTS global_nested_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_bookmark_id INTEGER NOT NULL,
    alias TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    command_template TEXT,
    encode_query BOOLEAN DEFAULT 1,
    display_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_bookmark_id) REFERENCES global_bookmarks(id) ON DELETE CASCADE,
    UNIQUE(parent_bookmark_id, alias)
);

CREATE INDEX IF NOT EXISTS idx_global_nested_bookmarks_parent ON global_nested_bookmarks(parent_bookmark_id);

-- Track migration
INSERT OR IGNORE INTO schema_migrations (version) VALUES (2);
