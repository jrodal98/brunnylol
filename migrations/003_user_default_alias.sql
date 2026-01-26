-- Add default_alias setting to users
-- Migration version: 003

-- Add default_alias column to users table
-- Note: Error is caught and ignored in code if column already exists
ALTER TABLE users ADD COLUMN default_alias TEXT DEFAULT NULL;

-- Track migration
INSERT OR IGNORE INTO schema_migrations (version) VALUES (3);
