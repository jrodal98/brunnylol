-- AI settings: global (admin-managed, shared) config, per-user config,
-- and the allow-list used when the global access mode is 'selected_users'.
-- All statements are idempotent so this can run on every startup, matching
-- the way migration 001 is applied in db::init_db.

-- Global AI settings (singleton row, id is always 1)
CREATE TABLE IF NOT EXISTS ai_global_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    base_url TEXT,   -- OpenAI-compatible base URL; NULL/empty = fall back to env
    api_key TEXT,    -- API token; NULL/empty = fall back to env. Never rendered back to clients in full.
    model TEXT,      -- Model name; NULL/empty = fall back to env
    access_mode TEXT NOT NULL DEFAULT 'admins_only'
        CHECK (access_mode IN ('disabled', 'admins_only', 'all_users', 'selected_users')),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Per-user AI settings (same shape as global, applies to a single user)
CREATE TABLE IF NOT EXISTS ai_user_settings (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    base_url TEXT,   -- NULL/empty = fall back to the shared (admin) config
    api_key TEXT,    -- NULL/empty = fall back to the shared (admin) config
    model TEXT,      -- NULL/empty = fall back to the shared (admin) config
    enabled BOOLEAN NOT NULL DEFAULT 1, -- user-level opt-out: 0 disables AI entirely for this user
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Users explicitly allowed to use the shared AI config when
-- ai_global_settings.access_mode = 'selected_users'
CREATE TABLE IF NOT EXISTS ai_allowed_users (
    user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations (version) VALUES (2);
