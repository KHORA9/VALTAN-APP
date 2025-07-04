-- Fix vector_blob column if missing
-- Since vector_blob was added in migration 0002, this is now a no-op
-- Just update the schema version to indicate we've run this migration
UPDATE settings SET value = '3' WHERE key = 'schema_version';
