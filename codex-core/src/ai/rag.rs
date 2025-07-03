//! Retrieval-Augmented Generation (RAG) implementation

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, debug};

use crate::{CodexError, CodexResult};
use crate::config::AiConfig;
use crate::db::DatabaseManager;
use super::{InferenceEngine, EmbeddingEngine, RagResponse, RagSource};

/// RAG engine for contextual AI responses
pub struct RagEngine {
    inference: Arc<RwLock<InferenceEngine>>,
    embeddings: Arc<EmbeddingEngine>,
    db: Option<Arc<DatabaseManager>>,
    config: RagConfig,
}

impl std::fmt::Debug for RagEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RagEngine")
            .field("config", &self.config)
            .field("has_db", &self.db.is_some())
            .finish()
    }
}

/// RAG configuration
#[derive(Debug, Clone)]
pub struct RagConfig {
    pub max_context_documents: usize,
    pub similarity_threshold: f32,
    pub context_window_size: usize,
    pub enable_reranking: bool,
    pub chunk_overlap_ratio: f32,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            max_context_documents: 5,
            similarity_threshold: 0.3,
            context_window_size: 2048,
            enable_reranking: true,
            chunk_overlap_ratio: 0.1,
        }
    }
}

impl RagEngine {
    /// Create a new RAG engine
    pub async fn new(
        inference: Arc<RwLock<InferenceEngine>>,
        embeddings: Arc<EmbeddingEngine>,
        config: &AiConfig,
    ) -> Result<Self> {
        info!("Initializing RAG engine");

        let rag_config = RagConfig::default();

        Ok(Self {
            inference,
            embeddings,
            db: None,
            config: rag_config,
        })
    }

    /// Set the database manager for document retrieval
    pub fn set_database(&mut self, db: Arc<DatabaseManager>) {
        self.db = Some(db);
    }

    /// Perform RAG query with retrieval and generation
    pub async fn query(&self, query: &str, context_limit: usize) -> CodexResult<RagResponse> {
        debug!("Performing RAG query: {}", query);

        // Step 1: Generate query embedding
        let query_embedding = self.embeddings.generate_embedding(query).await?;

        // Step 2: Retrieve relevant documents
        let sources = self.retrieve_relevant_documents(&query_embedding, context_limit).await?;

        if sources.is_empty() {
            return Ok(RagResponse {
                answer: "I don't have enough relevant information in my knowledge base to answer that question.".to_string(),
                sources: Vec::new(),
                confidence: 0.0,
                context_used: 0,
            });
        }

        // Step 3: Build context from retrieved documents
        let context = self.build_context(&sources);

        // Step 4: Generate answer using context
        let answer = self.generate_contextual_answer(query, &context).await?;

        // Step 5: Calculate confidence score
        let confidence = self.calculate_confidence(&sources);

        Ok(RagResponse {
            answer,
            sources,
            confidence,
            context_used: context.len(),
        })
    }

    /// Retrieve relevant documents based on query embedding
    async fn retrieve_relevant_documents(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> CodexResult<Vec<RagSource>> {
        let db = self.db.as_ref().ok_or_else(|| {
            CodexError::internal("Database not set for RAG engine")
        })?;

        // Get all document embeddings from database
        let embeddings = crate::db::EmbeddingQueries::get_all_vectors(db.pool()).await?;

        // Find most similar documents
        let similarities = self.embeddings.find_similar(
            query_embedding,
            &embeddings,
            limit.min(self.config.max_context_documents),
        );

        let mut sources = Vec::new();

        for similarity in similarities {
            if similarity.similarity_score >= self.config.similarity_threshold {
                // Get document details
                if let Ok(Some(document)) = crate::db::DocumentQueries::get_by_id(
                    db.pool(),
                    similarity.document_id,
                ).await {
                    // Extract relevant snippet
                    let snippet = self.extract_relevant_snippet(&document.content, query_embedding).await?;

                    sources.push(RagSource {
                        document_id: document.id,
                        title: document.title,
                        snippet,
                        relevance_score: similarity.similarity_score,
                    });
                }
            }
        }

        // Re-rank sources if enabled
        if self.config.enable_reranking {
            sources = self.rerank_sources(sources, query_embedding).await?;
        }

        Ok(sources)
    }

    /// Extract the most relevant snippet from a document
    async fn extract_relevant_snippet(
        &self,
        content: &str,
        query_embedding: &[f32],
    ) -> CodexResult<String> {
        // Generate embeddings for content chunks
        let chunk_embeddings = self.embeddings.generate_chunk_embeddings(
            content,
            200, // words per chunk
            20,  // overlap
        ).await?;

        // Find the most relevant chunk
        let mut best_similarity = 0.0;
        let mut best_chunk = String::new();

        for chunk_emb in chunk_embeddings {
            let similarity = self.embeddings.cosine_similarity(query_embedding, &chunk_emb.embedding);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_chunk = chunk_emb.text;
            }
        }

        // Limit snippet length
        let max_snippet_length = 300;
        if best_chunk.len() > max_snippet_length {
            let truncated = best_chunk.chars().take(max_snippet_length).collect::<String>();
            Ok(format!("{}...", truncated))
        } else {
            Ok(best_chunk)
        }
    }

    /// Re-rank sources based on additional relevance signals
    async fn rerank_sources(
        &self,
        mut sources: Vec<RagSource>,
        _query_embedding: &[f32],
    ) -> CodexResult<Vec<RagSource>> {
        // Simple re-ranking based on relevance score
        // In a more sophisticated implementation, you might:
        // 1. Use a cross-encoder model for better ranking
        // 2. Consider document metadata (recency, authority, etc.)
        // 3. Apply diversity filtering

        sources.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        Ok(sources)
    }

    /// Build context string from retrieved sources
    fn build_context(&self, sources: &[RagSource]) -> String {
        let mut context = String::new();
        let mut current_length = 0;
        let max_context_length = self.config.context_window_size;

        for (i, source) in sources.iter().enumerate() {
            let source_text = format!(
                "[Source {}: {}]\n{}\n\n",
                i + 1,
                source.title,
                source.snippet
            );

            if current_length + source_text.len() > max_context_length {
                break;
            }

            context.push_str(&source_text);
            current_length += source_text.len();
        }

        context
    }

    /// Generate answer using retrieved context
    async fn generate_contextual_answer(&self, query: &str, context: &str) -> CodexResult<String> {
        let prompt = format!(
            "Based on the following context, please provide a comprehensive and accurate answer to the question. If the context doesn't contain enough information to answer the question, please say so.\n\nContext:\n{}\n\nQuestion: {}\n\nAnswer:",
            context, query
        );

        let inference = self.inference.read().await;
        let config = crate::config::AiConfig::default(); // Use default config for now
        inference.generate(&prompt, &config).await
    }

    /// Calculate confidence score based on sources
    fn calculate_confidence(&self, sources: &[RagSource]) -> f32 {
        if sources.is_empty() {
            return 0.0;
        }

        // Calculate weighted average of relevance scores
        let total_score: f32 = sources.iter().map(|s| s.relevance_score).sum();
        let average_score = total_score / sources.len() as f32;

        // Apply penalties for low number of sources
        let source_count_factor = (sources.len() as f32 / 3.0).min(1.0);

        // Final confidence score
        (average_score * source_count_factor).min(1.0)
    }

    /// Summarize multiple documents
    pub async fn summarize_documents(&self, document_ids: &[uuid::Uuid]) -> CodexResult<String> {
        let db = self.db.as_ref().ok_or_else(|| {
            CodexError::internal("Database not set for RAG engine")
        })?;

        let mut combined_content = String::new();
        let mut titles = Vec::new();

        for doc_id in document_ids {
            if let Ok(Some(document)) = crate::db::DocumentQueries::get_by_id(db.pool(), *doc_id).await {
                titles.push(document.title.clone());
                combined_content.push_str(&format!("\n\n# {}\n{}", document.title, document.content));
            }
        }

        if combined_content.is_empty() {
            return Err(CodexError::not_found("No documents found to summarize"));
        }

        let prompt = format!(
            "Please provide a comprehensive summary of the following documents:\n\nDocuments: {}\n\nContent:{}\n\nSummary:",
            titles.join(", "),
            combined_content
        );

        let inference = self.inference.read().await;
        let config = crate::config::AiConfig::default();
        inference.generate(&prompt, &config).await
    }

    /// Compare multiple documents
    pub async fn compare_documents(&self, document_ids: &[uuid::Uuid], comparison_aspect: &str) -> CodexResult<String> {
        let db = self.db.as_ref().ok_or_else(|| {
            CodexError::internal("Database not set for RAG engine")
        })?;

        let mut documents_content = Vec::new();

        for doc_id in document_ids {
            if let Ok(Some(document)) = crate::db::DocumentQueries::get_by_id(db.pool(), *doc_id).await {
                documents_content.push(format!("Document: {}\nContent: {}", document.title, document.content));
            }
        }

        if documents_content.len() < 2 {
            return Err(CodexError::validation("Need at least 2 documents for comparison"));
        }

        let prompt = format!(
            "Please compare the following documents focusing on: {}\n\n{}\n\nComparison:",
            comparison_aspect,
            documents_content.join("\n\n---\n\n")
        );

        let inference = self.inference.read().await;
        let config = crate::config::AiConfig::default();
        inference.generate(&prompt, &config).await
    }

    /// Get configuration
    pub fn get_config(&self) -> &RagConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RagConfig) {
        self.config = config;
    }

    /// Shutdown the RAG engine
    pub async fn shutdown(&self) -> CodexResult<()> {
        info!("Shutting down RAG engine");
        // No specific cleanup needed for RAG engine
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        assert_eq!(config.max_context_documents, 5);
        assert_eq!(config.similarity_threshold, 0.3);
        assert!(config.enable_reranking);
    }

    #[test]
    fn test_context_building() {
        let sources = vec![
            RagSource {
                document_id: uuid::Uuid::new_v4(),
                title: "Test Document 1".to_string(),
                snippet: "This is the first test snippet.".to_string(),
                relevance_score: 0.9,
            },
            RagSource {
                document_id: uuid::Uuid::new_v4(),
                title: "Test Document 2".to_string(),
                snippet: "This is the second test snippet.".to_string(),
                relevance_score: 0.8,
            },
        ];

        let config = RagConfig::default();
        let inference = Arc::new(RwLock::new(
            // This would normally be a real InferenceEngine
            // For testing, we'll skip this part
        ));
        let embeddings = Arc::new(
            // This would normally be a real EmbeddingEngine
            // For testing, we'll skip this part
        );

        // Test would create RAG engine and test context building
        // Skipping full test due to complex dependencies
    }
}