-- Vector search optimization migration
-- Version: 0002
-- Description: Add binary vector storage and vector search optimization

-- Add binary vector column for better performance
ALTER TABLE embeddings ADD COLUMN vector_blob BLOB;

-- Create index for faster vector similarity searches
CREATE INDEX idx_embeddings_vector_blob ON embeddings(vector_blob) WHERE vector_blob IS NOT NULL;

-- Add vector search optimization table for pre-computed similarities
CREATE TABLE vector_similarities (
    id TEXT PRIMARY KEY NOT NULL,
    document_id_1 TEXT NOT NULL,
    document_id_2 TEXT NOT NULL,
    similarity REAL NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id_1) REFERENCES documents(id) ON DELETE CASCADE,
    FOREIGN KEY (document_id_2) REFERENCES documents(id) ON DELETE CASCADE
);

-- Indexes for similarity table
CREATE INDEX idx_vector_similarities_doc1 ON vector_similarities(document_id_1);
CREATE INDEX idx_vector_similarities_doc2 ON vector_similarities(document_id_2);
CREATE INDEX idx_vector_similarities_similarity ON vector_similarities(similarity);
CREATE INDEX idx_vector_similarities_model ON vector_similarities(model);

-- Add vector search cache table for frequently accessed vectors
CREATE TABLE vector_cache (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL,
    vector_blob BLOB NOT NULL,
    dimensions INTEGER NOT NULL,
    model TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Indexes for vector cache
CREATE INDEX idx_vector_cache_document_id ON vector_cache(document_id);
CREATE INDEX idx_vector_cache_model ON vector_cache(model);
CREATE INDEX idx_vector_cache_access_count ON vector_cache(access_count);
CREATE INDEX idx_vector_cache_last_accessed ON vector_cache(last_accessed);

-- Update schema version
UPDATE settings SET value = '2' WHERE key = 'schema_version';

-- Add vector search settings
INSERT INTO settings (key, value, description, category, is_user_configurable)
VALUES 
    ('vector_similarity_threshold', '0.7', 'Minimum similarity threshold for vector search', 'search', TRUE),
    ('vector_cache_size', '1000', 'Maximum number of vectors to cache in memory', 'performance', TRUE),
    ('vector_batch_size', '50', 'Batch size for vector operations', 'performance', TRUE),
    ('enable_vector_cache', 'true', 'Enable vector caching for better performance', 'performance', TRUE);