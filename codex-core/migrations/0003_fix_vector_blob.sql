-- Fix vector_blob column if missing
ALTER TABLE embeddings ADD COLUMN IF NOT EXISTS vector_blob BLOB;