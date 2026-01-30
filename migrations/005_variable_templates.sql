-- Migration 005: Add variable templating support
-- This migration adds support for RFC 6570-style variable templates

-- Add variable_metadata column to bookmarks table (stores JSON)
ALTER TABLE bookmarks ADD COLUMN variable_metadata TEXT;

-- Add variable_metadata column to nested_bookmarks table
ALTER TABLE nested_bookmarks ADD COLUMN variable_metadata TEXT;

-- Auto-migrate existing {} templates to {query}
UPDATE bookmarks
SET command_template = REPLACE(command_template, '{}', '{query}')
WHERE command_template IS NOT NULL
  AND command_template LIKE '%{}%'
  AND command_template NOT LIKE '%{{%}}%'; -- Skip escaped braces

UPDATE nested_bookmarks
SET command_template = REPLACE(command_template, '{}', '{query}')
WHERE command_template IS NOT NULL
  AND command_template LIKE '%{}%'
  AND command_template NOT LIKE '%{{%}}%'; -- Skip escaped braces

-- Mark this migration as applied
INSERT OR IGNORE INTO schema_migrations (version) VALUES (5);
