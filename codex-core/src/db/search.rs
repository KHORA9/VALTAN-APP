use sqlx::SqlitePool;
use crate::CodexResult;
use super::models::Document;

pub struct Search;

impl Search {
    /// Simple FTS5 search that ACTUALLY WORKS
    pub async fn fts5(pool: &SqlitePool, query: &str, limit: i64) -> CodexResult<Vec<Document>> {
        let sanitized = query.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>();
        
        if sanitized.is_empty() {
            return Ok(Vec::new());
        }
        
        let sql = r#"
            SELECT d.* FROM documents d
            JOIN documents_fts ON d.rowid = documents_fts.rowid
            WHERE documents_fts MATCH ? AND d.is_deleted = 0
            ORDER BY rank
            LIMIT ?
        "#;
        
        let docs = sqlx::query_as::<_, Document>(sql)
            .bind(format!("\"{}\"", sanitized))
            .bind(limit)
            .fetch_all(pool)
            .await?;
        
        Ok(docs)
    }
}