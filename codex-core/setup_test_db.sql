-- Setup test database with all necessary tables
-- Run all migrations in order

-- From 0001_init.sql (core tables)
CREATE TABLE documents (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    summary TEXT,
    author TEXT,
    source TEXT,
    url TEXT,
    content_type TEXT NOT NULL DEFAULT 'text/plain',
    category TEXT,
    tags TEXT,
    language TEXT NOT NULL DEFAULT 'en',
    reading_time INTEGER,
    difficulty_level INTEGER CHECK (difficulty_level >= 1 AND difficulty_level <= 5),
    file_size INTEGER,
    file_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    last_accessed TEXT,
    view_count INTEGER NOT NULL DEFAULT 0,
    is_favorite BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE
);

-- FTS5 table
CREATE VIRTUAL TABLE documents_fts USING fts5(
    title,
    content,
    summary,
    author,
    category,
    tags
);

-- Embeddings table
CREATE TABLE embeddings (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL,
    vector TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    model TEXT NOT NULL,
    chunk_index INTEGER NOT NULL DEFAULT 0,
    text_chunk TEXT NOT NULL,
    start_position INTEGER NOT NULL DEFAULT 0,
    end_position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Settings table
CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL DEFAULT 'general',
    is_user_configurable BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Other tables from migration
CREATE TABLE bookmarks (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL,
    title TEXT NOT NULL,
    notes TEXT,
    position INTEGER,
    selected_text TEXT,
    tags TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- From 0002_vector_optimization.sql
ALTER TABLE embeddings ADD COLUMN vector_blob BLOB;

-- Vector cache table
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

-- Create indexes
CREATE INDEX idx_documents_title ON documents(title);
CREATE INDEX idx_documents_category ON documents(category);
CREATE INDEX idx_embeddings_document_id ON embeddings(document_id);
CREATE INDEX idx_vector_cache_document_id ON vector_cache(document_id);

-- Insert basic settings
INSERT INTO settings (key, value, description, category, is_user_configurable)
VALUES ('schema_version', '3', 'Database schema version', 'system', FALSE);
