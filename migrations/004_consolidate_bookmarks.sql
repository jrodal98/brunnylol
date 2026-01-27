-- Migration 004: Consolidate bookmark tables into unified schema
-- This eliminates duplication between user/global bookmarks

-- Drop old tables (pre-production, clean slate approach)
DROP TABLE IF EXISTS global_nested_bookmarks;
DROP TABLE IF EXISTS global_bookmarks;
DROP TABLE IF EXISTS nested_bookmarks;
DROP TABLE IF EXISTS user_bookmarks;

-- Create new unified bookmarks table
CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scope TEXT NOT NULL CHECK(scope IN ('personal', 'global')),
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE, -- NULL for global bookmarks
    alias TEXT NOT NULL,
    bookmark_type TEXT NOT NULL CHECK(bookmark_type IN ('simple', 'templated', 'nested')),
    url TEXT NOT NULL,
    description TEXT NOT NULL,
    command_template TEXT,
    encode_query BOOLEAN NOT NULL DEFAULT 1,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL, -- Track who created global bookmarks
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(scope, user_id, alias) -- Composite unique constraint (NULL user_id values are treated as distinct)
);

-- Create new unified nested_bookmarks table
CREATE TABLE IF NOT EXISTS nested_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_bookmark_id INTEGER NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT NOT NULL,
    command_template TEXT,
    encode_query BOOLEAN NOT NULL DEFAULT 1,
    display_order INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(parent_bookmark_id, alias)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_bookmarks_scope_user ON bookmarks(scope, user_id);
CREATE INDEX IF NOT EXISTS idx_bookmarks_alias ON bookmarks(alias);
CREATE INDEX IF NOT EXISTS idx_nested_bookmarks_parent ON nested_bookmarks(parent_bookmark_id);
