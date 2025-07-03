//! Database query operations for Codex Core

use sqlx::{SqlitePool, Row};
use chrono::Utc;

use crate::{CodexError, CodexResult};
use super::models::*;

/// Document query operations
pub struct DocumentQueries;

impl DocumentQueries {
    /// Create a new document
    pub async fn create(pool: &SqlitePool, document: &Document) -> CodexResult<()> {
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, title, content, summary, author, source, url, content_type,
                category, tags, language, reading_time, difficulty_level,
                file_size, file_hash, created_at, updated_at, last_accessed,
                view_count, is_favorite, is_archived, is_deleted
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&document.id)
        .bind(&document.title)
        .bind(&document.content)
        .bind(&document.summary)
        .bind(&document.author)
        .bind(&document.source)
        .bind(&document.url)
        .bind(&document.content_type)
        .bind(&document.category)
        .bind(&document.tags)
        .bind(&document.language)
        .bind(document.reading_time)
        .bind(document.difficulty_level)
        .bind(document.file_size)
        .bind(&document.file_hash)
        .bind(&document.created_at)
        .bind(&document.updated_at)
        .bind(&document.last_accessed)
        .bind(document.view_count)
        .bind(document.is_favorite)
        .bind(document.is_archived)
        .bind(document.is_deleted)
        .execute(pool)
        .await
        .map_err(|e| CodexError::Database(e))?;

        Ok(())
    }

    /// Get document by ID
    pub async fn get_by_id(pool: &SqlitePool, id: &str) -> CodexResult<Option<Document>> {
        let row = sqlx::query(
            "SELECT * FROM documents WHERE id = ? AND is_deleted = false"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| CodexError::Database(e))?;

        if let Some(row) = row {
            let document = Document {
                id: row.get("id"),
                title: row.get("title"),
                content: row.get("content"),
                summary: row.get("summary"),
                author: row.get("author"),
                source: row.get("source"),
                url: row.get("url"),
                content_type: row.get("content_type"),
                category: row.get("category"),
                tags: row.get("tags"),
                language: row.get("language"),
                reading_time: row.get("reading_time"),
                difficulty_level: row.get("difficulty_level"),
                file_size: row.get("file_size"),
                file_hash: row.get("file_hash"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                last_accessed: row.get("last_accessed"),
                view_count: row.get("view_count"),
                is_favorite: row.get("is_favorite"),
                is_archived: row.get("is_archived"),
                is_deleted: row.get("is_deleted"),
            };
            Ok(Some(document))
        } else {
            Ok(None)
        }
    }

    /// Update document
    pub async fn update(pool: &SqlitePool, document: &Document) -> CodexResult<()> {
        let updated_at = Utc::now();
        
        sqlx::query!(
            r#"
            UPDATE documents SET
                title = ?, content = ?, summary = ?, author = ?, source = ?,
                url = ?, content_type = ?, category = ?, tags = ?, language = ?,
                reading_time = ?, difficulty_level = ?, file_size = ?, file_hash = ?,
                updated_at = ?, last_accessed = ?, view_count = ?, is_favorite = ?,
                is_archived = ?, is_deleted = ?
            WHERE id = ?
            "#,
            document.title,
            document.content,
            document.summary,
            document.author,
            document.source,
            document.url,
            document.content_type,
            document.category,
            document.tags,
            document.language,
            document.reading_time,
            document.difficulty_level,
            document.file_size,
            document.file_hash,
            updated_at,
            document.last_accessed,
            document.view_count,
            document.is_favorite,
            document.is_archived,
            document.is_deleted,
            document.id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete document (soft delete)
    pub async fn delete(pool: &SqlitePool, id: &str) -> CodexResult<()> {
        let updated_at = Utc::now();
        
        sqlx::query!(
            "UPDATE documents SET is_deleted = true, updated_at = ? WHERE id = ?",
            updated_at,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Search documents using FTS5
    pub async fn search_full_text(
        pool: &SqlitePool,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> CodexResult<Vec<Document>> {
        let documents = sqlx::query_as!(
            Document,
            r#"
            SELECT d.* FROM documents d
            WHERE d.rowid IN (
                SELECT rowid FROM documents_fts WHERE documents_fts MATCH ?
            ) AND d.is_deleted = false
            ORDER BY d.created_at DESC
            LIMIT ? OFFSET ?
            "#,
            query,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok(documents)
    }

    /// Get documents by category
    pub async fn get_by_category(
        pool: &SqlitePool,
        category: &str,
        limit: i64,
        offset: i64,
    ) -> CodexResult<Vec<Document>> {
        let documents = sqlx::query_as!(
            Document,
            r#"
            SELECT * FROM documents
            WHERE category = ? AND is_deleted = false
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
            category,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok(documents)
    }

    /// Get recent documents
    pub async fn get_recent(pool: &SqlitePool, limit: i64) -> CodexResult<Vec<Document>> {
        let documents = sqlx::query_as!(
            Document,
            r#"
            SELECT * FROM documents
            WHERE is_deleted = false
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(documents)
    }

    /// Get favorite documents
    pub async fn get_favorites(pool: &SqlitePool, limit: i64) -> CodexResult<Vec<Document>> {
        let documents = sqlx::query_as!(
            Document,
            r#"
            SELECT * FROM documents
            WHERE is_favorite = true AND is_deleted = false
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(documents)
    }

    /// Update view count and last accessed
    pub async fn update_access(pool: &SqlitePool, id: &str) -> CodexResult<()> {
        let now = Utc::now();
        
        sqlx::query!(
            r#"
            UPDATE documents
            SET view_count = view_count + 1, last_accessed = ?
            WHERE id = ?
            "#,
            now,
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Embedding query operations
pub struct EmbeddingQueries;

impl EmbeddingQueries {
    /// Create a new embedding
    pub async fn create(pool: &SqlitePool, embedding: &Embedding) -> CodexResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO embeddings (
                id, document_id, vector, dimensions, model, chunk_index,
                text_chunk, start_position, end_position, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            embedding.id,
            embedding.document_id,
            embedding.vector,
            embedding.dimensions,
            embedding.model,
            embedding.chunk_index,
            embedding.text_chunk,
            embedding.start_position,
            embedding.end_position,
            embedding.created_at
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get embeddings for a document
    pub async fn get_by_document(
        pool: &SqlitePool,
        document_id: &str,
    ) -> CodexResult<Vec<Embedding>> {
        let embeddings = sqlx::query_as!(
            Embedding,
            "SELECT * FROM embeddings WHERE document_id = ? ORDER BY chunk_index",
            document_id
        )
        .fetch_all(pool)
        .await?;

        Ok(embeddings)
    }

    /// Delete embeddings for a document
    pub async fn delete_by_document(pool: &SqlitePool, document_id: &str) -> CodexResult<()> {
        sqlx::query!(
            "DELETE FROM embeddings WHERE document_id = ?",
            document_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all embeddings for similarity search
    pub async fn get_all_vectors(pool: &SqlitePool) -> CodexResult<Vec<(String, Vec<f32>)>> {
        let rows = sqlx::query!(
            "SELECT document_id, vector FROM embeddings ORDER BY document_id, chunk_index"
        )
        .fetch_all(pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            if let Ok(vector) = serde_json::from_str::<Vec<f32>>(&row.vector) {
                result.push((row.document_id, vector));
            }
        }

        Ok(result)
    }
}

/// Settings query operations
pub struct SettingQueries;

impl SettingQueries {
    /// Get setting by key
    pub async fn get(pool: &SqlitePool, key: &str) -> CodexResult<Option<Setting>> {
        let setting = sqlx::query_as!(
            Setting,
            "SELECT * FROM settings WHERE key = ?",
            key
        )
        .fetch_optional(pool)
        .await?;

        Ok(setting)
    }

    /// Set setting value
    pub async fn set(pool: &SqlitePool, setting: &Setting) -> CodexResult<()> {
        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO settings (
                key, value, description, category, is_user_configurable, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            setting.key,
            setting.value,
            setting.description,
            setting.category,
            setting.is_user_configurable,
            setting.created_at,
            setting.updated_at
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all settings by category
    pub async fn get_by_category(pool: &SqlitePool, category: &str) -> CodexResult<Vec<Setting>> {
        let settings = sqlx::query_as!(
            Setting,
            "SELECT * FROM settings WHERE category = ? ORDER BY key",
            category
        )
        .fetch_all(pool)
        .await?;

        Ok(settings)
    }

    /// Delete setting
    pub async fn delete(pool: &SqlitePool, key: &str) -> CodexResult<()> {
        sqlx::query!("DELETE FROM settings WHERE key = ?", key)
            .execute(pool)
            .await?;

        Ok(())
    }
}

/// Bookmark query operations
pub struct BookmarkQueries;

impl BookmarkQueries {
    /// Create a new bookmark
    pub async fn create(pool: &SqlitePool, bookmark: &Bookmark) -> CodexResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO bookmarks (
                id, document_id, title, notes, position, selected_text, tags, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            bookmark.id,
            bookmark.document_id,
            bookmark.title,
            bookmark.notes,
            bookmark.position,
            bookmark.selected_text,
            bookmark.tags,
            bookmark.created_at,
            bookmark.updated_at
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get bookmarks for a document
    pub async fn get_by_document(
        pool: &SqlitePool,
        document_id: &str,
    ) -> CodexResult<Vec<Bookmark>> {
        let bookmarks = sqlx::query_as!(
            Bookmark,
            "SELECT * FROM bookmarks WHERE document_id = ? ORDER BY position",
            document_id
        )
        .fetch_all(pool)
        .await?;

        Ok(bookmarks)
    }

    /// Get all bookmarks
    pub async fn get_all(pool: &SqlitePool, limit: i64) -> CodexResult<Vec<Bookmark>> {
        let bookmarks = sqlx::query_as!(
            Bookmark,
            "SELECT * FROM bookmarks ORDER BY created_at DESC LIMIT ?",
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(bookmarks)
    }

    /// Delete bookmark
    pub async fn delete(pool: &SqlitePool, id: &str) -> CodexResult<()> {
        sqlx::query!("DELETE FROM bookmarks WHERE id = ?", id)
            .execute(pool)
            .await?;

        Ok(())
    }
}