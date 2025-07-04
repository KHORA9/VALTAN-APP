//! Database testing utilities and fixtures

use codex_core::{
    db::{DatabaseManager, models::{Document, DocumentCreateRequest, Embedding}},
    config::DatabaseConfig,
    CodexResult,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use fake::{Fake, Faker};
use fake::faker::lorem::en::*;

/// Test database manager with automatic cleanup
pub struct TestDatabase {
    pub manager: DatabaseManager,
    pub pool: SqlitePool,
    pub config: DatabaseConfig,
}

impl TestDatabase {
    /// Create a new test database with migrations applied
    pub async fn new() -> CodexResult<Self> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        
        let config = DatabaseConfig {
            url: format!("sqlite:{}", db_path.display()),
            max_connections: 5,
            acquire_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 1800,
        };
        
        let manager = DatabaseManager::new(&config).await?;
        let pool = manager.pool().clone();
        
        Ok(Self {
            manager,
            pool,
            config,
        })
    }
    
    /// Create a test document with random data
    pub async fn create_test_document(&self) -> CodexResult<Document> {
        let request = DocumentCreateRequest {
            title: Words(2..5).fake::<String>(),
            content: Paragraphs(3..8).fake::<Vec<String>>().join("\n\n"),
            summary: Some(Sentence(10..20).fake()),
            author: Some(Name().fake()),
            category: Some(["Philosophy", "Science", "Technology", "Health"].choose(&mut rand::thread_rng()).unwrap().to_string()),
            tags: Some(vec![Word().fake(), Word().fake()]),
            language: Some("en".to_string()),
            reading_time: Some((100..600).fake()),
            difficulty_level: Some((1..=5).fake()),
            source_url: None,
            file_path: None,
            file_hash: None,
        };
        
        self.manager.content().create_document(request).await
    }
    
    /// Create multiple test documents
    pub async fn create_test_documents(&self, count: usize) -> CodexResult<Vec<Document>> {
        let mut documents = Vec::with_capacity(count);
        for _ in 0..count {
            documents.push(self.create_test_document().await?);
        }
        Ok(documents)
    }
    
    /// Create a test embedding
    pub async fn create_test_embedding(&self, doc_id: &str) -> CodexResult<Embedding> {
        let vector: Vec<f32> = (0..384).map(|_| rand::random::<f32>()).collect();
        
        Embedding::create(
            &self.pool,
            doc_id,
            &vector,
            "test-model",
            384,
        ).await
    }
    
    /// Clean up test data
    pub async fn cleanup(&self) -> CodexResult<()> {
        sqlx::query("DELETE FROM documents").execute(&self.pool).await?;
        sqlx::query("DELETE FROM embeddings").execute(&self.pool).await?;
        sqlx::query("DELETE FROM settings").execute(&self.pool).await?;
        Ok(())
    }
    
    /// Get document count
    pub async fn document_count(&self) -> CodexResult<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }
    
    /// Assert document exists
    pub async fn assert_document_exists(&self, id: &str) -> CodexResult<Document> {
        self.manager.content()
            .get_document_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Document {} not found", id))
    }
    
    /// Assert document does not exist
    pub async fn assert_document_not_exists(&self, id: &str) -> CodexResult<()> {
        let doc = self.manager.content().get_document_by_id(id).await?;
        if doc.is_some() {
            return Err(anyhow::anyhow!("Document {} should not exist", id).into());
        }
        Ok(())
    }
}

/// Builder for creating test documents with specific properties
#[derive(Default)]
pub struct DocumentBuilder {
    title: Option<String>,
    content: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    reading_time: Option<i32>,
    difficulty_level: Option<i32>,
}

impl DocumentBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }
    
    pub fn content(mut self, content: &str) -> Self {
        self.content = Some(content.to_string());
        self
    }
    
    pub fn category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }
    
    pub fn tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = Some(tags.into_iter().map(|s| s.to_string()).collect());
        self
    }
    
    pub fn reading_time(mut self, time: i32) -> Self {
        self.reading_time = Some(time);
        self
    }
    
    pub fn difficulty(mut self, level: i32) -> Self {
        self.difficulty_level = Some(level);
        self
    }
    
    pub fn build(self) -> DocumentCreateRequest {
        DocumentCreateRequest {
            title: self.title.unwrap_or_else(|| Words(2..5).fake()),
            content: self.content.unwrap_or_else(|| Paragraphs(3..8).fake::<Vec<String>>().join("\n\n")),
            summary: Some(Sentence(10..20).fake()),
            author: Some(Name().fake()),
            category: self.category,
            tags: self.tags,
            language: Some("en".to_string()),
            reading_time: self.reading_time,
            difficulty_level: self.difficulty_level,
            source_url: None,
            file_path: None,
            file_hash: None,
        }
    }
}

/// Sample data sets for testing
pub struct SampleData;

impl SampleData {
    /// Philosophy documents
    pub fn philosophy_docs() -> Vec<DocumentCreateRequest> {
        vec![
            DocumentBuilder::new()
                .title("Introduction to Stoicism")
                .content("Stoicism is a philosophy that teaches virtue, wisdom, and emotional resilience...")
                .category("Philosophy")
                .tags(vec!["stoicism", "ancient philosophy", "virtue ethics"])
                .difficulty(2)
                .build(),
            DocumentBuilder::new()
                .title("Principles of Existentialism")
                .content("Existentialism emphasizes individual existence, freedom and choice...")
                .category("Philosophy")
                .tags(vec!["existentialism", "freedom", "authenticity"])
                .difficulty(4)
                .build(),
        ]
    }
    
    /// Science documents
    pub fn science_docs() -> Vec<DocumentCreateRequest> {
        vec![
            DocumentBuilder::new()
                .title("Quantum Computing Basics")
                .content("Quantum computing leverages quantum mechanics principles...")
                .category("Science")
                .tags(vec!["quantum", "computing", "physics"])
                .difficulty(5)
                .build(),
            DocumentBuilder::new()
                .title("DNA Structure and Function")
                .content("DNA is the hereditary material in all known living organisms...")
                .category("Science")
                .tags(vec!["dna", "genetics", "biology"])
                .difficulty(3)
                .build(),
        ]
    }
}

/// Macro for database test setup
#[macro_export]
macro_rules! db_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> CodexResult<()> {
            let db = TestDatabase::new().await?;
            let result = $body(&db).await;
            let _ = db.cleanup().await; // Best effort cleanup
            result
        }
    };
}