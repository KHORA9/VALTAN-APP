//! Content management module for Codex Core
//!
//! This module handles document parsing, indexing, and search operations
//! for the knowledge repository.

use std::sync::Arc;
use std::path::Path;
use anyhow::Result;
use tracing::{info, warn, error};

use crate::{CodexError, CodexResult};
use crate::config::ContentConfig;
use crate::db::DatabaseManager;
use crate::ai::AiEngine;

pub mod parser;
pub mod indexer;
pub mod search;

pub use parser::*;
pub use indexer::*;
pub use search::*;

/// Content manager handling all content operations
#[derive(Debug)]
pub struct ContentManager {
    db: Arc<DatabaseManager>,
    ai: Arc<AiEngine>,
    parser: Arc<ContentParser>,
    indexer: Arc<ContentIndexer>,
    search: Arc<SearchEngine>,
    config: ContentConfig,
}

impl ContentManager {
    /// Create a new content manager
    pub async fn new(
        db: Arc<DatabaseManager>,
        ai: Arc<AiEngine>,
        config: &ContentConfig,
    ) -> Result<Self> {
        info!("Initializing content manager");

        // Ensure content directory exists
        tokio::fs::create_dir_all(&config.content_dir).await?;

        // Initialize components
        let parser = Arc::new(ContentParser::new(config)?);
        let indexer = Arc::new(ContentIndexer::new(
            Arc::clone(&db),
            Arc::clone(&ai),
            config,
        ).await?);
        let search = Arc::new(SearchEngine::new(
            Arc::clone(&db),
            Arc::clone(&ai),
            config,
        ).await?);

        info!("Content manager initialized successfully");

        Ok(Self {
            db,
            ai,
            parser,
            indexer,
            search,
            config: config.clone(),
        })
    }

    /// Import a document from file
    pub async fn import_document<P: AsRef<Path>>(&self, file_path: P) -> CodexResult<uuid::Uuid> {
        let file_path = file_path.as_ref();
        info!("Importing document: {:?}", file_path);

        // Validate file
        self.validate_file(file_path).await?;

        // Parse document
        let parsed_doc = self.parser.parse_file(file_path).await?;

        // Check for duplicate content by file hash
        if let Some(existing_doc) = self.check_for_duplicate(&parsed_doc.file_hash).await? {
            warn!("Duplicate content detected for file: {:?}, existing document: {}", file_path, existing_doc.id);
            return Err(CodexError::validation(format!(
                "Document with identical content already exists: {} ({})",
                existing_doc.title, existing_doc.id
            )));
        }

        // Create document model
        let mut document = crate::db::models::Document::new(
            parsed_doc.title,
            parsed_doc.content,
            parsed_doc.content_type,
        );

        // Set additional metadata
        document.author = parsed_doc.author;
        document.language = parsed_doc.language;
        document.file_size = Some(parsed_doc.file_size as i64);
        document.file_hash = Some(parsed_doc.file_hash);

        // Generate AI-enhanced metadata
        if let Ok(summary) = self.ai.summarize(&document.content, Some(200)).await {
            document.summary = Some(summary);
        }

        if let Ok(tags) = self.ai.generate_tags(&document.content, Some(10)).await {
            document.set_tags(tags);
        }

        if let Ok(difficulty) = self.ai.assess_difficulty(&document.content).await {
            document.difficulty_level = Some(difficulty.into());
        }

        if let Ok(reading_time) = self.ai.estimate_reading_time(&document.content).await {
            document.reading_time = Some(reading_time.into());
        }

        // Save to database
        crate::db::DocumentQueries::create(self.db.pool(), &document).await?;

        // Index the document
        self.indexer.index_document(&document).await?;

        info!("Document imported successfully: {}", document.id);
        Ok(uuid::Uuid::parse_str(&document.id).unwrap_or_default())
    }

    /// Import content from text
    pub async fn import_text_content(
        &self,
        title: String,
        content: String,
        content_type: Option<String>,
    ) -> CodexResult<uuid::Uuid> {
        info!("Importing text content: {}", title);

        // Create document model
        let mut document = crate::db::models::Document::new(
            title,
            content,
            content_type.unwrap_or_else(|| "text/plain".to_string()),
        );

        // Generate AI-enhanced metadata
        if let Ok(summary) = self.ai.summarize(&document.content, Some(200)).await {
            document.summary = Some(summary);
        }

        if let Ok(tags) = self.ai.generate_tags(&document.content, Some(10)).await {
            document.set_tags(tags);
        }

        if let Ok(difficulty) = self.ai.assess_difficulty(&document.content).await {
            document.difficulty_level = Some(difficulty.into());
        }

        if let Ok(reading_time) = self.ai.estimate_reading_time(&document.content).await {
            document.reading_time = Some(reading_time.into());
        }

        // Save to database
        crate::db::DocumentQueries::create(self.db.pool(), &document).await?;

        // Index the document
        self.indexer.index_document(&document).await?;

        info!("Text content imported successfully: {}", document.id);
        Ok(uuid::Uuid::parse_str(&document.id).unwrap_or_default())
    }

    /// Update document content
    pub async fn update_document(&self, document_id: uuid::Uuid, new_content: String) -> CodexResult<()> {
        info!("Updating document: {}", document_id);

        // Get existing document
        let mut document = crate::db::DocumentQueries::get_by_id(self.db.pool(), &document_id.to_string())
            .await?
            .ok_or_else(|| CodexError::not_found("Document not found"))?;

        // Update content
        document.content = new_content;
        document.updated_at = chrono::Utc::now().to_rfc3339();

        // Regenerate AI metadata
        if let Ok(summary) = self.ai.summarize(&document.content, Some(200)).await {
            document.summary = Some(summary);
        }

        if let Ok(tags) = self.ai.generate_tags(&document.content, Some(10)).await {
            document.set_tags(tags);
        }

        if let Ok(difficulty) = self.ai.assess_difficulty(&document.content).await {
            document.difficulty_level = Some(difficulty.into());
        }

        if let Ok(reading_time) = self.ai.estimate_reading_time(&document.content).await {
            document.reading_time = Some(reading_time.into());
        }

        // Update in database
        crate::db::DocumentQueries::update(self.db.pool(), &document).await?;

        // Re-index the document
        self.indexer.reindex_document(&document).await?;

        info!("Document updated successfully: {}", document_id);
        Ok(())
    }

    /// Delete document
    pub async fn delete_document(&self, document_id: uuid::Uuid) -> CodexResult<()> {
        info!("Deleting document: {}", document_id);

        // Remove from search index
        self.indexer.remove_document(document_id).await?;

        // Soft delete from database
        crate::db::DocumentQueries::delete(self.db.pool(), &document_id.to_string()).await?;

        info!("Document deleted successfully: {}", document_id);
        Ok(())
    }

    /// Search documents
    pub async fn search_documents(&self, query: &str, options: SearchOptions) -> CodexResult<SearchResults> {
        self.search.search(query, options).await
    }

    /// Get document by ID
    pub async fn get_document(&self, document_id: uuid::Uuid) -> CodexResult<Option<crate::db::models::Document>> {
        let document = crate::db::DocumentQueries::get_by_id(self.db.pool(), &document_id.to_string()).await?;
        
        // Update access statistics
        if document.is_some() {
            let _ = crate::db::DocumentQueries::update_access(self.db.pool(), &document_id.to_string()).await;
        }

        Ok(document)
    }

    /// Get recent documents
    pub async fn get_recent_documents(&self, limit: i64) -> CodexResult<Vec<crate::db::models::Document>> {
        crate::db::DocumentQueries::get_recent(self.db.pool(), limit).await
    }

    /// Get documents by category
    pub async fn get_documents_by_category(
        &self,
        category: &str,
        limit: i64,
        offset: i64,
    ) -> CodexResult<Vec<crate::db::models::Document>> {
        crate::db::DocumentQueries::get_by_category(self.db.pool(), category, limit, offset).await
    }

    /// Get favorite documents
    pub async fn get_favorite_documents(&self, limit: i64) -> CodexResult<Vec<crate::db::models::Document>> {
        crate::db::DocumentQueries::get_favorites(self.db.pool(), limit).await
    }

    /// Toggle document favorite status
    pub async fn toggle_favorite(&self, document_id: uuid::Uuid) -> CodexResult<bool> {
        let mut document = crate::db::DocumentQueries::get_by_id(self.db.pool(), &document_id.to_string())
            .await?
            .ok_or_else(|| CodexError::not_found("Document not found"))?;

        document.is_favorite = !document.is_favorite;
        document.updated_at = chrono::Utc::now().to_rfc3339();

        crate::db::DocumentQueries::update(self.db.pool(), &document).await?;

        Ok(document.is_favorite)
    }

    /// Categorize document
    pub async fn categorize_document(&self, document_id: uuid::Uuid, category: String) -> CodexResult<()> {
        let mut document = crate::db::DocumentQueries::get_by_id(self.db.pool(), &document_id.to_string())
            .await?
            .ok_or_else(|| CodexError::not_found("Document not found"))?;

        document.category = Some(category);
        document.updated_at = chrono::Utc::now().to_rfc3339();

        crate::db::DocumentQueries::update(self.db.pool(), &document).await?;

        Ok(())
    }

    /// Bulk import documents from directory
    pub async fn bulk_import_directory<P: AsRef<Path>>(&self, directory: P) -> CodexResult<BulkImportResult> {
        let directory = directory.as_ref();
        info!("Bulk importing from directory: {:?}", directory);

        let mut result = BulkImportResult {
            total_files: 0,
            successful_imports: 0,
            failed_imports: 0,
            imported_documents: Vec::new(),
            errors: Vec::new(),
        };

        // Read directory recursively
        let mut entries = tokio::fs::read_dir(directory).await
            .map_err(|e| CodexError::io(e))?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            
            if path.is_file() {
                result.total_files += 1;

                match self.import_document(&path).await {
                    Ok(doc_id) => {
                        result.successful_imports += 1;
                        result.imported_documents.push(doc_id);
                    }
                    Err(e) => {
                        result.failed_imports += 1;
                        result.errors.push(format!("{:?}: {}", path, e));
                        warn!("Failed to import file {:?}: {}", path, e);
                    }
                }
            }
        }

        info!("Bulk import completed: {} successful, {} failed", 
               result.successful_imports, result.failed_imports);

        Ok(result)
    }

    /// Check for duplicate content by file hash
    async fn check_for_duplicate(&self, file_hash: &str) -> CodexResult<Option<crate::db::models::Document>> {
        // Query database for existing documents with the same file hash
        let existing_doc = crate::db::DocumentQueries::get_by_file_hash(self.db.pool(), file_hash).await?;
        Ok(existing_doc)
    }

    /// Validate file before import
    async fn validate_file(&self, file_path: &Path) -> CodexResult<()> {
        // Check if file exists
        if !file_path.exists() {
            return Err(CodexError::not_found("File does not exist"));
        }

        // Check file extension
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            if !self.config.supported_extensions.contains(&extension.to_lowercase()) {
                return Err(CodexError::validation(format!(
                    "Unsupported file extension: {}",
                    extension
                )));
            }
        }

        // Check file size
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size_mb = metadata.len() / (1024 * 1024);
        
        if file_size_mb > self.config.max_file_size_mb as u64 {
            return Err(CodexError::validation(format!(
                "File too large: {} MB (max: {} MB)",
                file_size_mb,
                self.config.max_file_size_mb
            )));
        }

        Ok(())
    }

    /// Get content statistics
    pub async fn get_content_stats(&self) -> CodexResult<ContentStats> {
        let db_stats = self.db.get_stats().await?;
        
        Ok(ContentStats {
            total_documents: db_stats.document_count,
            total_embeddings: db_stats.embedding_count,
            database_size_bytes: db_stats.database_size_bytes,
            indexed_documents: db_stats.document_count, // Assume all documents are indexed
        })
    }

    /// Reindex all documents
    pub async fn reindex_all_documents(&self) -> CodexResult<()> {
        info!("Starting full reindex of all documents");

        let documents = crate::db::DocumentQueries::get_recent(self.db.pool(), i64::MAX).await?;
        
        for document in documents {
            if let Err(e) = self.indexer.reindex_document(&document).await {
                error!("Failed to reindex document {}: {}", document.id, e);
            }
        }

        info!("Full reindex completed");
        Ok(())
    }

    /// Health check
    pub async fn health_check(&self) -> CodexResult<bool> {
        // Check if all components are healthy
        let db_health = self.db.health_check().await?;
        let ai_health = self.ai.health_check().await?;

        Ok(db_health && ai_health)
    }

    /// Shutdown content manager
    pub async fn shutdown(&self) -> CodexResult<()> {
        info!("Shutting down content manager");
        // No specific cleanup needed for content manager
        Ok(())
    }
}

/// Bulk import result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BulkImportResult {
    pub total_files: usize,
    pub successful_imports: usize,
    pub failed_imports: usize,
    pub imported_documents: Vec<uuid::Uuid>,
    pub errors: Vec<String>,
}

/// Content statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContentStats {
    pub total_documents: u64,
    pub total_embeddings: u64,
    pub database_size_bytes: u64,
    pub indexed_documents: u64,
}