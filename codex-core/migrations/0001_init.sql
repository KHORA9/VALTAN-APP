-- Initial database schema for Codex Vault Next-Gen
-- Version: 0001
-- Description: Create core tables for documents, embeddings, settings, and user data

-- Enable foreign key constraints
PRAGMA foreign_keys = ON;

-- =====================================================
-- CORE CONTENT TABLES
-- =====================================================

-- Documents table - stores all content documents
CREATE TABLE documents (
    id TEXT PRIMARY KEY NOT NULL,  -- UUID as TEXT
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    summary TEXT,
    author TEXT,
    source TEXT,
    url TEXT,
    content_type TEXT NOT NULL DEFAULT 'text/plain',
    category TEXT,
    tags TEXT,  -- JSON array of tags
    language TEXT NOT NULL DEFAULT 'en',
    reading_time INTEGER,  -- in minutes
    difficulty_level INTEGER CHECK (difficulty_level >= 1 AND difficulty_level <= 5),
    file_size INTEGER,  -- in bytes
    file_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    last_accessed TEXT,
    view_count INTEGER NOT NULL DEFAULT 0,
    is_favorite BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE
);

-- Indexes for documents table
CREATE INDEX idx_documents_title ON documents(title);
CREATE INDEX idx_documents_category ON documents(category);
CREATE INDEX idx_documents_author ON documents(author);
CREATE INDEX idx_documents_language ON documents(language);
CREATE INDEX idx_documents_content_type ON documents(content_type);
CREATE INDEX idx_documents_created_at ON documents(created_at);
CREATE INDEX idx_documents_updated_at ON documents(updated_at);
CREATE INDEX idx_documents_view_count ON documents(view_count);
CREATE INDEX idx_documents_is_favorite ON documents(is_favorite);
CREATE INDEX idx_documents_is_archived ON documents(is_archived);
CREATE INDEX idx_documents_is_deleted ON documents(is_deleted);
CREATE INDEX idx_documents_difficulty_level ON documents(difficulty_level);

-- Full-text search index using FTS5
CREATE VIRTUAL TABLE documents_fts USING fts5(
    title,
    content,
    summary,
    author,
    category,
    tags,
    content_tokens='porter',
    tokenize='porter unicode61 remove_diacritics 1'
);

-- Trigger to keep FTS5 index in sync with documents table
CREATE TRIGGER documents_fts_insert AFTER INSERT ON documents BEGIN
    INSERT INTO documents_fts(rowid, title, content, summary, author, category, tags)
    VALUES (NEW.rowid, NEW.title, NEW.content, NEW.summary, NEW.author, NEW.category, NEW.tags);
END;

CREATE TRIGGER documents_fts_update AFTER UPDATE ON documents BEGIN
    UPDATE documents_fts SET
        title = NEW.title,
        content = NEW.content,
        summary = NEW.summary,
        author = NEW.author,
        category = NEW.category,
        tags = NEW.tags
    WHERE rowid = NEW.rowid;
END;

CREATE TRIGGER documents_fts_delete AFTER DELETE ON documents BEGIN
    DELETE FROM documents_fts WHERE rowid = OLD.rowid;
END;

-- =====================================================
-- AI EMBEDDINGS TABLES
-- =====================================================

-- Vector embeddings for semantic search
CREATE TABLE embeddings (
    id TEXT PRIMARY KEY NOT NULL,  -- UUID as TEXT
    document_id TEXT NOT NULL,
    vector TEXT NOT NULL,  -- JSON array of float values
    dimensions INTEGER NOT NULL,
    model TEXT NOT NULL,
    chunk_index INTEGER NOT NULL DEFAULT 0,
    text_chunk TEXT NOT NULL,
    start_position INTEGER NOT NULL DEFAULT 0,
    end_position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Indexes for embeddings table
CREATE INDEX idx_embeddings_document_id ON embeddings(document_id);
CREATE INDEX idx_embeddings_model ON embeddings(model);
CREATE INDEX idx_embeddings_chunk_index ON embeddings(chunk_index);
CREATE INDEX idx_embeddings_created_at ON embeddings(created_at);

-- =====================================================
-- APPLICATION SETTINGS
-- =====================================================

-- Application settings and configuration
CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,  -- JSON value
    description TEXT,
    category TEXT NOT NULL DEFAULT 'general',
    is_user_configurable BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Index for settings
CREATE INDEX idx_settings_category ON settings(category);
CREATE INDEX idx_settings_is_user_configurable ON settings(is_user_configurable);

-- =====================================================
-- USER DATA TABLES
-- =====================================================

-- User bookmarks
CREATE TABLE bookmarks (
    id TEXT PRIMARY KEY NOT NULL,  -- UUID as TEXT
    document_id TEXT NOT NULL,
    title TEXT NOT NULL,
    notes TEXT,
    position INTEGER,  -- character offset in document
    selected_text TEXT,
    tags TEXT,  -- JSON array
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Indexes for bookmarks
CREATE INDEX idx_bookmarks_document_id ON bookmarks(document_id);
CREATE INDEX idx_bookmarks_created_at ON bookmarks(created_at);
CREATE INDEX idx_bookmarks_position ON bookmarks(position);

-- User notes
CREATE TABLE notes (
    id TEXT PRIMARY KEY NOT NULL,  -- UUID as TEXT
    document_id TEXT,  -- NULL for standalone notes
    title TEXT NOT NULL,
    content TEXT NOT NULL,  -- Markdown content
    tags TEXT,  -- JSON array
    color TEXT,
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE SET NULL
);

-- Indexes for notes
CREATE INDEX idx_notes_document_id ON notes(document_id);
CREATE INDEX idx_notes_title ON notes(title);
CREATE INDEX idx_notes_is_pinned ON notes(is_pinned);
CREATE INDEX idx_notes_created_at ON notes(created_at);

-- Collections for organizing documents
CREATE TABLE collections (
    id TEXT PRIMARY KEY NOT NULL,  -- UUID as TEXT
    name TEXT NOT NULL,
    description TEXT,
    color TEXT,
    icon TEXT,
    is_pinned BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Indexes for collections
CREATE INDEX idx_collections_name ON collections(name);
CREATE INDEX idx_collections_is_pinned ON collections(is_pinned);
CREATE INDEX idx_collections_created_at ON collections(created_at);

-- Many-to-many relationship between documents and collections
CREATE TABLE document_collections (
    document_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    added_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    PRIMARY KEY (document_id, collection_id),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- Indexes for document_collections
CREATE INDEX idx_document_collections_collection_id ON document_collections(collection_id);
CREATE INDEX idx_document_collections_order_index ON document_collections(order_index);
CREATE INDEX idx_document_collections_added_at ON document_collections(added_at);

-- =====================================================
-- ANALYTICS AND TRACKING
-- =====================================================

-- Search history for analytics and suggestions
CREATE TABLE search_history (
    id TEXT PRIMARY KEY NOT NULL,  -- UUID as TEXT
    query TEXT NOT NULL,
    search_type TEXT NOT NULL DEFAULT 'full_text',  -- full_text, semantic, hybrid
    result_count INTEGER NOT NULL DEFAULT 0,
    searched_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
);

-- Indexes for search_history
CREATE INDEX idx_search_history_query ON search_history(query);
CREATE INDEX idx_search_history_search_type ON search_history(search_type);
CREATE INDEX idx_search_history_searched_at ON search_history(searched_at);

-- Reading progress tracking
CREATE TABLE reading_progress (
    document_id TEXT PRIMARY KEY NOT NULL,
    progress_percentage REAL NOT NULL DEFAULT 0.0 CHECK (progress_percentage >= 0.0 AND progress_percentage <= 100.0),
    scroll_position INTEGER,
    session_start TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    total_reading_time INTEGER NOT NULL DEFAULT 0,  -- in seconds
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'utc')),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

-- Index for reading_progress
CREATE INDEX idx_reading_progress_updated_at ON reading_progress(updated_at);
CREATE INDEX idx_reading_progress_progress_percentage ON reading_progress(progress_percentage);

-- =====================================================
-- SYSTEM METADATA
-- =====================================================

-- Database schema version tracking
INSERT INTO settings (key, value, description, category, is_user_configurable)
VALUES (
    'schema_version',
    '1',
    'Database schema version for migrations',
    'system',
    FALSE
);

-- Default application settings
INSERT INTO settings (key, value, description, category, is_user_configurable)
VALUES 
    ('app_version', '"0.1.0"', 'Application version', 'system', FALSE),
    ('first_run', 'true', 'Whether this is the first application run', 'system', FALSE),
    ('last_backup', 'null', 'Timestamp of last backup', 'system', FALSE),
    ('ai_model', '"llama-2-7b-chat.gguf"', 'Primary AI model file', 'ai', TRUE),
    ('search_suggestions_enabled', 'true', 'Enable search suggestions', 'search', TRUE),
    ('auto_index_enabled', 'true', 'Enable automatic content indexing', 'content', TRUE),
    ('theme', '"auto"', 'UI theme (light, dark, auto)', 'ui', TRUE),
    ('language', '"en"', 'Application language', 'ui', TRUE),
    ('analytics_enabled', 'false', 'Enable privacy-first local analytics', 'privacy', TRUE);

-- Create indexes for performance optimization after all inserts
ANALYZE;

-- =====================================================
-- PERFORMANCE OPTIMIZATIONS
-- =====================================================

-- Optimize SQLite settings for our use case
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;  -- 64MB cache
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 268435456;  -- 256MB mmap