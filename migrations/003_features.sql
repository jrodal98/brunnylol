-- Feature tables: bookmark usage metrics and API tokens.
-- All statements are idempotent (safe to run on every startup).

-- Per-bookmark usage counters, keyed by bookmark id so counts follow the
-- bookmark and disappear with it (ON DELETE CASCADE).
CREATE TABLE IF NOT EXISTS bookmark_usage (
    bookmark_id INTEGER PRIMARY KEY REFERENCES bookmarks(id) ON DELETE CASCADE,
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TIMESTAMP
);

-- Personal API tokens for programmatic access (scripts, AI agents).
-- Only a SHA-256 hash of each token is stored; the plaintext is shown
-- to the user exactly once, at creation time.
CREATE TABLE IF NOT EXISTS api_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_api_tokens_user ON api_tokens(user_id);
