//! Database connection utilities and helpers

use sqlx::{SqlitePool, Row};
use anyhow::Result;
use uuid::Uuid;

use crate::CodexResult;

/// Database connection utilities
pub struct ConnectionUtils;

impl ConnectionUtils {
    /// Check if database tables exist
    pub async fn tables_exist(pool: &SqlitePool) -> CodexResult<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('documents', 'embeddings', 'settings')"
        )
        .fetch_one(pool)
        .await?;

        Ok(count >= 3)
    }

    /// Get database version/schema version
    pub async fn get_schema_version(pool: &SqlitePool) -> CodexResult<Option<i32>> {
        let version = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT value FROM settings WHERE key = 'schema_version'"
        )
        .fetch_optional(pool)
        .await?;

        Ok(version.flatten())
    }

    /// Set database schema version
    pub async fn set_schema_version(pool: &SqlitePool, version: i32) -> CodexResult<()> {
        sqlx::query!(
            "INSERT OR REPLACE INTO settings (key, value, category, is_user_configurable, created_at, updated_at) VALUES ('schema_version', ?, 'system', false, datetime('now'), datetime('now'))",
            version
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Execute raw SQL query (for maintenance operations)
    pub async fn execute_raw(pool: &SqlitePool, sql: &str) -> CodexResult<u64> {
        let result = sqlx::query(sql).execute(pool).await?;
        Ok(result.rows_affected())
    }

    /// Check foreign key constraints
    pub async fn check_foreign_keys(pool: &SqlitePool) -> CodexResult<Vec<String>> {
        let mut errors = Vec::new();

        // Check documents-embeddings relationship
        let orphaned_embeddings: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM embeddings e LEFT JOIN documents d ON e.document_id = d.id WHERE d.id IS NULL"
        )
        .fetch_one(pool)
        .await?;

        if orphaned_embeddings > 0 {
            errors.push(format!("Found {} orphaned embeddings", orphaned_embeddings));
        }

        // Check document-collection relationships
        let orphaned_doc_collections: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM document_collections dc LEFT JOIN documents d ON dc.document_id = d.id WHERE d.id IS NULL"
        )
        .fetch_one(pool)
        .await?;

        if orphaned_doc_collections > 0 {
            errors.push(format!("Found {} orphaned document-collection links", orphaned_doc_collections));
        }

        Ok(errors)
    }

    /// Clean up orphaned records
    pub async fn cleanup_orphaned_records(pool: &SqlitePool) -> CodexResult<u64> {
        let mut total_cleaned = 0u64;

        // Clean up orphaned embeddings
        let embeddings_result = sqlx::query(
            "DELETE FROM embeddings WHERE document_id NOT IN (SELECT id FROM documents)"
        )
        .execute(pool)
        .await?;
        total_cleaned += embeddings_result.rows_affected();

        // Clean up orphaned bookmarks
        let bookmarks_result = sqlx::query(
            "DELETE FROM bookmarks WHERE document_id NOT IN (SELECT id FROM documents)"
        )
        .execute(pool)
        .await?;
        total_cleaned += bookmarks_result.rows_affected();

        // Clean up orphaned document-collection relationships
        let doc_collections_result = sqlx::query(
            "DELETE FROM document_collections WHERE document_id NOT IN (SELECT id FROM documents) OR collection_id NOT IN (SELECT id FROM collections)"
        )
        .execute(pool)
        .await?;
        total_cleaned += doc_collections_result.rows_affected();

        // Clean up orphaned reading progress
        let progress_result = sqlx::query(
            "DELETE FROM reading_progress WHERE document_id NOT IN (SELECT id FROM documents)"
        )
        .execute(pool)
        .await?;
        total_cleaned += progress_result.rows_affected();

        Ok(total_cleaned)
    }

    /// Rebuild FTS5 index
    pub async fn rebuild_fts_index(pool: &SqlitePool) -> CodexResult<()> {
        sqlx::query("INSERT INTO documents_fts(documents_fts) VALUES('rebuild')")
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Get database integrity check results
    pub async fn integrity_check(pool: &SqlitePool) -> CodexResult<Vec<String>> {
        let rows = sqlx::query("PRAGMA integrity_check")
            .fetch_all(pool)
            .await?;

        let results: Vec<String> = rows
            .into_iter()
            .map(|row| row.get::<String, _>(0))
            .collect();

        Ok(results)
    }

    /// Get database file size and page count
    pub async fn get_database_info(pool: &SqlitePool) -> CodexResult<DatabaseInfo> {
        let page_count: i64 = sqlx::query_scalar("PRAGMA page_count").fetch_one(pool).await?;
        let page_size: i64 = sqlx::query_scalar("PRAGMA page_size").fetch_one(pool).await?;
        let unused_pages: i64 = sqlx::query_scalar("PRAGMA freelist_count").fetch_one(pool).await?;

        Ok(DatabaseInfo {
            page_count: page_count as u64,
            page_size: page_size as u64,
            total_size_bytes: (page_count * page_size) as u64,
            unused_pages: unused_pages as u64,
            unused_bytes: (unused_pages * page_size) as u64,
        })
    }
}

/// Database information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseInfo {
    pub page_count: u64,
    pub page_size: u64,
    pub total_size_bytes: u64,
    pub unused_pages: u64,
    pub unused_bytes: u64,
}

impl DatabaseInfo {
    /// Get the percentage of unused space
    pub fn unused_percentage(&self) -> f64 {
        if self.total_size_bytes == 0 {
            0.0
        } else {
            (self.unused_bytes as f64 / self.total_size_bytes as f64) * 100.0
        }
    }

    /// Check if database needs optimization (>10% unused space)
    pub fn needs_optimization(&self) -> bool {
        self.unused_percentage() > 10.0
    }
}