//! Database query operations for Codex Core

use sqlx::{SqlitePool, Row, Sqlite, Statement};
use chrono::Utc;
use std::collections::HashMap;

use crate::{CodexError, CodexResult};
use super::models::*;

/// Prepared statement manager for high-performance database operations
pub struct PreparedStatements {
    statements: HashMap<String, Statement<'static, Sqlite>>,
}

impl PreparedStatements {
    /// Create a new prepared statements manager
    pub async fn new(pool: &SqlitePool) -> CodexResult<Self> {
        let mut statements = HashMap::new();
        
        // Document queries
        statements.insert(
            "document_search_fts".to_string(),
            pool.prepare(
                r#"
                SELECT d.* FROM documents d
                JOIN documents_fts fts ON d.rowid = fts.rowid
                WHERE fts MATCH ? AND d.is_deleted = false
                ORDER BY rank
                LIMIT ?
                "#
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "document_search_ranked".to_string(),
            pool.prepare(
                r#"
                SELECT d.*, 
                       bm25(documents_fts, 10.0, 5.0, 1.0, 1.0, 3.0, 2.0) as rank_score
                FROM documents d
                JOIN documents_fts fts ON d.rowid = fts.rowid
                WHERE fts MATCH ? AND d.is_deleted = false
                ORDER BY rank_score DESC
                LIMIT ? OFFSET ?
                "#
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "document_by_id".to_string(),
            pool.prepare(
                "SELECT * FROM documents WHERE id = ? AND is_deleted = false"
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "document_by_category".to_string(),
            pool.prepare(
                r#"
                SELECT * FROM documents
                WHERE category = ? AND is_deleted = false
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "document_recent".to_string(),
            pool.prepare(
                r#"
                SELECT * FROM documents
                WHERE is_deleted = false
                ORDER BY created_at DESC
                LIMIT ?
                "#
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "document_favorites".to_string(),
            pool.prepare(
                r#"
                SELECT * FROM documents
                WHERE is_favorite = true AND is_deleted = false
                ORDER BY updated_at DESC
                LIMIT ?
                "#
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "document_update_access".to_string(),
            pool.prepare(
                r#"
                UPDATE documents
                SET view_count = view_count + 1, last_accessed = ?
                WHERE id = ?
                "#
            ).await.map_err(CodexError::from)?
        );
        
        // Embedding queries
        statements.insert(
            "embedding_by_document".to_string(),
            pool.prepare(
                "SELECT * FROM embeddings WHERE document_id = ? ORDER BY chunk_index"
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "embedding_all_vectors".to_string(),
            pool.prepare(
                "SELECT document_id, vector FROM embeddings ORDER BY document_id, chunk_index"
            ).await.map_err(CodexError::from)?
        );
        
        // Search history queries
        statements.insert(
            "search_history_insert".to_string(),
            pool.prepare(
                r#"
                INSERT INTO search_history (id, query, search_type, result_count, searched_at)
                VALUES (?, ?, ?, ?, ?)
                "#
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "search_history_recent".to_string(),
            pool.prepare(
                r#"
                SELECT DISTINCT query FROM search_history
                WHERE search_type = ?
                ORDER BY searched_at DESC
                LIMIT ?
                "#
            ).await.map_err(CodexError::from)?
        );
        
        // Settings queries
        statements.insert(
            "setting_get".to_string(),
            pool.prepare("SELECT * FROM settings WHERE key = ?").await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "setting_by_category".to_string(),
            pool.prepare(
                "SELECT * FROM settings WHERE category = ? ORDER BY key"
            ).await.map_err(CodexError::from)?
        );
        
        // Bookmark queries
        statements.insert(
            "bookmark_by_document".to_string(),
            pool.prepare(
                "SELECT * FROM bookmarks WHERE document_id = ? ORDER BY position"
            ).await.map_err(CodexError::from)?
        );
        
        statements.insert(
            "bookmark_all".to_string(),
            pool.prepare(
                "SELECT * FROM bookmarks ORDER BY created_at DESC LIMIT ?"
            ).await.map_err(CodexError::from)?
        );
        
        Ok(Self { statements })
    }
    
    /// Get a prepared statement by name
    pub fn get(&self, name: &str) -> Option<&Statement<'static, Sqlite>> {
        self.statements.get(name)
    }
    
    /// Get all statement names
    pub fn statement_names(&self) -> Vec<&String> {
        self.statements.keys().collect()
    }
}

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
            r#"
            SELECT document_id, vector, vector_blob 
            FROM embeddings 
            ORDER BY document_id, chunk_index
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            // Prefer binary vector format for better performance
            let vector = if let Some(blob) = row.vector_blob {
                // Deserialize from binary format
                bincode::deserialize::<Vec<f32>>(&blob)
                    .map_err(|e| CodexError::Database(sqlx::Error::Decode(Box::new(e))))?
            } else {
                // Fallback to JSON format
                serde_json::from_str::<Vec<f32>>(&row.vector)
                    .map_err(|e| CodexError::Database(sqlx::Error::Decode(Box::new(e))))?
            };
            
            result.push((row.document_id, vector));
        }

        Ok(result)
    }
    
    /// Store embedding with both JSON and binary formats
    pub async fn create_with_binary(pool: &SqlitePool, embedding: &Embedding) -> CodexResult<()> {
        let vector = embedding.get_vector();
        let vector_blob = bincode::serialize(&vector)
            .map_err(|e| CodexError::Database(sqlx::Error::Encode(Box::new(e))))?;
        
        sqlx::query!(
            r#"
            INSERT INTO embeddings (
                id, document_id, vector, vector_blob, dimensions, model, chunk_index,
                text_chunk, start_position, end_position, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            embedding.id,
            embedding.document_id,
            embedding.vector,
            vector_blob,
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
    
    /// Get cached vectors for fast similarity search
    pub async fn get_cached_vectors(pool: &SqlitePool, model: &str, limit: Option<i64>) -> CodexResult<Vec<(String, Vec<f32>)>> {
        let limit = limit.unwrap_or(1000);
        
        let rows = sqlx::query!(
            r#"
            SELECT document_id, vector_blob, access_count
            FROM vector_cache
            WHERE model = ?
            ORDER BY access_count DESC, last_accessed DESC
            LIMIT ?
            "#,
            model,
            limit
        )
        .fetch_all(pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let vector = bincode::deserialize::<Vec<f32>>(&row.vector_blob)
                .map_err(|e| CodexError::Database(sqlx::Error::Decode(Box::new(e))))?;
            result.push((row.document_id, vector));
        }

        Ok(result)
    }
    
    /// Update vector cache access statistics
    pub async fn update_cache_access(pool: &SqlitePool, document_id: &str) -> CodexResult<()> {
        let now = Utc::now();
        
        sqlx::query!(
            r#"
            UPDATE vector_cache
            SET access_count = access_count + 1, last_accessed = ?
            WHERE document_id = ?
            "#,
            now,
            document_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
    
    /// Add vector to cache
    pub async fn cache_vector(
        pool: &SqlitePool,
        document_id: &str,
        vector: &[f32],
        model: &str,
    ) -> CodexResult<()> {
        let vector_blob = bincode::serialize(vector)
            .map_err(|e| CodexError::Database(sqlx::Error::Encode(Box::new(e))))?;
        
        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO vector_cache (
                id, document_id, vector_blob, dimensions, model, access_count, last_accessed, created_at
            ) VALUES (?, ?, ?, ?, ?, 1, ?, ?)
            "#,
            uuid::Uuid::new_v4().to_string(),
            document_id,
            vector_blob,
            vector.len() as i64,
            model,
            Utc::now().to_rfc3339(),
            Utc::now().to_rfc3339()
        )
        .execute(pool)
        .await?;

        Ok(())
    }
    
    /// Clean up old cache entries
    pub async fn cleanup_cache(pool: &SqlitePool, max_entries: i64) -> CodexResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM vector_cache
            WHERE id NOT IN (
                SELECT id FROM vector_cache
                ORDER BY access_count DESC, last_accessed DESC
                LIMIT ?
            )
            "#,
            max_entries
        )
        .execute(pool)
        .await?;

        Ok(())
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

/// Search query operations - unified search interface
pub struct SearchQueries;

impl SearchQueries {
    /// Simple search interface for FTS5 full-text search
    pub async fn search(
        pool: &SqlitePool,
        query: &str,
        limit: Option<i64>,
    ) -> CodexResult<Vec<Document>> {
        let limit = limit.unwrap_or(50);
        
        // Sanitize query for FTS5
        let sanitized_query = Self::sanitize_fts_query(query);
        
        let start = std::time::Instant::now();
        
        let documents = sqlx::query_as!(
            Document,
            r#"
            SELECT d.* FROM documents d
            JOIN documents_fts fts ON d.rowid = fts.rowid
            WHERE fts MATCH ? AND d.is_deleted = false
            ORDER BY rank
            LIMIT ?
            "#,
            sanitized_query,
            limit
        )
        .fetch_all(pool)
        .await?;
        
        let duration = start.elapsed();
        tracing::debug!(
            "FTS5 search completed in {:?}ms for query: '{}' (found {} results)",
            duration.as_millis(),
            query,
            documents.len()
        );
        
        Ok(documents)
    }
    
    /// Enhanced search with ranking and highlighting
    pub async fn search_with_ranking(
        pool: &SqlitePool,
        query: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> CodexResult<Vec<(Document, f64)>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);
        
        let sanitized_query = Self::sanitize_fts_query(query);
        
        let start = std::time::Instant::now();
        
        let rows = sqlx::query!(
            r#"
            SELECT d.*, 
                   bm25(documents_fts, 10.0, 5.0, 1.0, 1.0, 3.0, 2.0) as rank_score,
                   snippet(documents_fts, 1, '<mark>', '</mark>', '...', 32) as snippet
            FROM documents d
            JOIN documents_fts fts ON d.rowid = fts.rowid
            WHERE fts MATCH ? AND d.is_deleted = false
            ORDER BY rank_score DESC
            LIMIT ? OFFSET ?
            "#,
            sanitized_query,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;
        
        let duration = start.elapsed();
        tracing::debug!(
            "Ranked FTS5 search completed in {:?}ms for query: '{}' (found {} results)",
            duration.as_millis(),
            query,
            rows.len()
        );
        
        let mut results = Vec::new();
        for row in rows {
            let document = Document {
                id: row.id,
                title: row.title,
                content: row.content,
                summary: row.summary,
                author: row.author,
                source: row.source,
                url: row.url,
                content_type: row.content_type,
                category: row.category,
                tags: row.tags,
                language: row.language,
                reading_time: row.reading_time,
                difficulty_level: row.difficulty_level,
                file_size: row.file_size,
                file_hash: row.file_hash,
                created_at: row.created_at,
                updated_at: row.updated_at,
                last_accessed: row.last_accessed,
                view_count: row.view_count,
                is_favorite: row.is_favorite,
                is_archived: row.is_archived,
                is_deleted: row.is_deleted,
            };
            
            let score = row.rank_score.unwrap_or(0.0);
            results.push((document, score));
        }
        
        Ok(results)
    }
    
    /// Semantic search using vector embeddings
    pub async fn search_semantic(
        pool: &SqlitePool,
        query_vector: &[f32],
        limit: Option<i64>,
        similarity_threshold: Option<f32>,
    ) -> CodexResult<Vec<(Document, f32)>> {
        let limit = limit.unwrap_or(10);
        let threshold = similarity_threshold.unwrap_or(0.5);
        
        let start = std::time::Instant::now();
        
        // Get all embeddings and compute similarity in-memory
        // Note: For production, consider using a proper vector database
        let embeddings = EmbeddingQueries::get_all_vectors(pool).await?;
        
        let mut similarities = Vec::new();
        for (doc_id, embedding) in embeddings {
            let similarity = Self::cosine_similarity(query_vector, &embedding);
            if similarity >= threshold {
                similarities.push((doc_id, similarity));
            }
        }
        
        // Sort by similarity and limit results
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(limit as usize);
        
        // Fetch documents for top results
        let mut results = Vec::new();
        for (doc_id, similarity) in similarities {
            if let Some(document) = DocumentQueries::get_by_id(pool, &doc_id).await? {
                results.push((document, similarity));
            }
        }
        
        let duration = start.elapsed();
        tracing::debug!(
            "Semantic search completed in {:?}ms (found {} results)",
            duration.as_millis(),
            results.len()
        );
        
        Ok(results)
    }
    
    /// Hybrid search combining full-text and semantic search
    pub async fn search_hybrid(
        pool: &SqlitePool,
        query: &str,
        query_vector: Option<&[f32]>,
        limit: Option<i64>,
        text_weight: Option<f32>,
        semantic_weight: Option<f32>,
    ) -> CodexResult<Vec<(Document, f32)>> {
        let limit = limit.unwrap_or(20);
        let text_weight = text_weight.unwrap_or(0.7);
        let semantic_weight = semantic_weight.unwrap_or(0.3);
        
        let start = std::time::Instant::now();
        
        // Get full-text search results
        let text_results = Self::search_with_ranking(pool, query, Some(limit * 2), None).await?;
        
        // Get semantic search results if query vector is provided
        let semantic_results = if let Some(vector) = query_vector {
            Self::search_semantic(pool, vector, Some(limit), Some(0.3)).await?
        } else {
            Vec::new()
        };
        
        // Combine and re-rank results
        let mut combined_scores = std::collections::HashMap::new();
        let mut all_documents = std::collections::HashMap::new();
        
        // Add text search scores
        for (doc, score) in text_results {
            let normalized_score = Self::normalize_score(score, 0.0, 10.0);
            combined_scores.insert(doc.id.clone(), normalized_score * text_weight);
            all_documents.insert(doc.id.clone(), doc);
        }
        
        // Add semantic search scores
        for (doc, score) in semantic_results {
            let normalized_score = Self::normalize_score(score as f64, 0.0, 1.0);
            let existing_score = combined_scores.get(&doc.id).unwrap_or(&0.0);
            combined_scores.insert(doc.id.clone(), existing_score + (normalized_score * semantic_weight as f64));
            all_documents.insert(doc.id.clone(), doc);
        }
        
        // Sort by combined score
        let mut final_results: Vec<_> = combined_scores.into_iter()
            .filter_map(|(doc_id, score)| {
                all_documents.get(&doc_id).map(|doc| (doc.clone(), score as f32))
            })
            .collect();
        
        final_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        final_results.truncate(limit as usize);
        
        let duration = start.elapsed();
        tracing::debug!(
            "Hybrid search completed in {:?}ms (found {} results)",
            duration.as_millis(),
            final_results.len()
        );
        
        Ok(final_results)
    }
    
    /// Sanitize FTS5 query to prevent syntax errors
    fn sanitize_fts_query(query: &str) -> String {
        // Remove special FTS5 characters that might cause syntax errors
        let cleaned = query
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "-_".contains(*c))
            .collect::<String>();
        
        // Split into words and join with OR for better matching
        let words: Vec<&str> = cleaned.split_whitespace().collect();
        if words.is_empty() {
            return "".to_string();
        }
        
        // Create a phrase query for exact matches, fallback to OR for partial matches
        if words.len() == 1 {
            format!("\"{}\" OR {}*", words[0], words[0])
        } else {
            let phrase = format!("\"{}\"", words.join(" "));
            let or_terms = words.iter().map(|w| format!("{}*", w)).collect::<Vec<_>>().join(" OR ");
            format!("{} OR {}", phrase, or_terms)
        }
    }
    
    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (magnitude_a * magnitude_b)
    }
    
    /// Normalize score to 0-1 range
    fn normalize_score(score: f64, min_val: f64, max_val: f64) -> f64 {
        if max_val == min_val {
            return 0.0;
        }
        (score - min_val) / (max_val - min_val)
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