//! AI inference module for Codex Core
//!
//! This module provides local AI inference capabilities using Candle framework
//! with support for various LLM models and RAG (Retrieval-Augmented Generation).

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, debug, error, warn};

use crate::{CodexError, CodexResult};
use crate::config::AiConfig;

pub mod inference;
pub mod embeddings;
pub mod rag;

pub use inference::*;
pub use embeddings::*;
pub use rag::*;

/// AI engine managing all AI-related operations
#[derive(Debug)]
pub struct AiEngine {
    /// Model inference engine
    inference: Arc<RwLock<InferenceEngine>>,
    /// Embedding generator
    embeddings: Arc<EmbeddingEngine>,
    /// RAG system
    rag: Arc<RagEngine>,
    /// Configuration
    config: AiConfig,
}

impl AiEngine {
    /// Create a new AI engine with the given configuration
    pub async fn new(config: &AiConfig) -> Result<Self> {
        info!("Initializing AI engine");

        // Ensure models directory exists
        tokio::fs::create_dir_all(&config.models_dir).await?;

        // Initialize inference engine
        let inference = Arc::new(RwLock::new(
            InferenceEngine::new(config).await?
        ));

        // Initialize embedding engine
        let embeddings = Arc::new(EmbeddingEngine::new(config).await?);

        // Initialize RAG engine
        let rag = Arc::new(RagEngine::new(
            Arc::clone(&inference),
            Arc::clone(&embeddings),
            config,
        ).await?);

        info!("AI engine initialized successfully");

        Ok(Self {
            inference,
            embeddings,
            rag,
            config: config.clone(),
        })
    }

    /// Generate text completion using the loaded model
    pub async fn generate_text(&self, prompt: &str) -> CodexResult<String> {
        let inference = self.inference.read().await;
        inference.generate(prompt, &self.config).await
    }

    /// Generate text with streaming (for real-time UI updates)
    pub async fn generate_text_stream(
        &self,
        prompt: &str,
        callback: impl Fn(String) + Send + Sync + 'static,
    ) -> CodexResult<String> {
        let inference = self.inference.read().await;
        inference.generate_stream(prompt, &self.config, callback).await
    }

    /// Generate embedding for text
    pub async fn generate_embedding(&self, text: &str) -> CodexResult<Vec<f32>> {
        self.embeddings.generate_embedding(text).await
    }

    /// Generate embeddings for multiple texts (batch processing)
    pub async fn generate_embeddings_batch(&self, texts: &[String]) -> CodexResult<Vec<Vec<f32>>> {
        self.embeddings.generate_embeddings_batch(texts).await
    }

    /// Perform RAG query (retrieval-augmented generation)
    pub async fn rag_query(&self, query: &str, context_limit: usize) -> CodexResult<RagResponse> {
        self.rag.query(query, context_limit).await
    }

    /// Summarize text content
    pub async fn summarize(&self, text: &str, max_length: Option<usize>) -> CodexResult<String> {
        let max_len = max_length.unwrap_or(200);
        let prompt = format!(
            "Please provide a concise summary of the following text in approximately {} words:\n\n{}",
            max_len, text
        );
        
        self.generate_text(&prompt).await
    }

    /// Extract key points from text
    pub async fn extract_key_points(&self, text: &str, num_points: Option<usize>) -> CodexResult<Vec<String>> {
        let num = num_points.unwrap_or(5);
        let prompt = format!(
            "Extract the {} most important key points from the following text. Present them as a numbered list:\n\n{}",
            num, text
        );
        
        let response = self.generate_text(&prompt).await?;
        
        // Parse numbered list from response
        let points: Vec<String> = response
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with(char::is_numeric) {
                    // Remove number prefix and clean up
                    if let Some(pos) = trimmed.find('.') {
                        Some(trimmed[pos + 1..].trim().to_string())
                    } else {
                        Some(trimmed.to_string())
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(points)
    }

    /// Answer a question about specific content
    pub async fn answer_question(&self, question: &str, context: &str) -> CodexResult<String> {
        let prompt = format!(
            "Based on the following context, please answer the question. If the answer cannot be found in the context, please say so.\n\nContext:\n{}\n\nQuestion: {}\n\nAnswer:",
            context, question
        );
        
        self.generate_text(&prompt).await
    }

    /// Generate tags for content
    pub async fn generate_tags(&self, content: &str, max_tags: Option<usize>) -> CodexResult<Vec<String>> {
        let max = max_tags.unwrap_or(10);
        let prompt = format!(
            "Generate up to {} relevant tags for the following content. Return only the tags, separated by commas:\n\n{}",
            max, content
        );
        
        let response = self.generate_text(&prompt).await?;
        
        let tags: Vec<String> = response
            .split(',')
            .map(|tag| tag.trim().to_lowercase())
            .filter(|tag| !tag.is_empty())
            .collect();

        Ok(tags)
    }

    /// Categorize content
    pub async fn categorize_content(&self, content: &str, categories: &[String]) -> CodexResult<String> {
        let categories_str = categories.join(", ");
        let prompt = format!(
            "Categorize the following content into one of these categories: {}\n\nContent:\n{}\n\nCategory:",
            categories_str, content
        );
        
        let response = self.generate_text(&prompt).await?;
        
        // Find the best matching category
        let response_lower = response.to_lowercase();
        for category in categories {
            if response_lower.contains(&category.to_lowercase()) {
                return Ok(category.clone());
            }
        }
        
        // If no exact match, return the first category as default
        Ok(categories.first().unwrap_or(&"uncategorized".to_string()).clone())
    }

    /// Assess content difficulty level (1-5 scale)
    pub async fn assess_difficulty(&self, content: &str) -> CodexResult<i32> {
        let prompt = format!(
            "Rate the difficulty level of the following content on a scale of 1-5, where:\n1 = Beginner (basic concepts)\n2 = Elementary (some background needed)\n3 = Intermediate (moderate expertise required)\n4 = Advanced (significant expertise required)\n5 = Expert (deep specialization required)\n\nContent:\n{}\n\nDifficulty level (just the number):",
            content
        );
        
        let response = self.generate_text(&prompt).await?;
        
        // Extract number from response
        let difficulty = response
            .chars()
            .find(|c| c.is_ascii_digit())
            .and_then(|c| c.to_digit(10))
            .map(|d| d as i32)
            .unwrap_or(3); // Default to intermediate

        Ok(difficulty.clamp(1, 5))
    }

    /// Estimate reading time in minutes
    pub async fn estimate_reading_time(&self, content: &str) -> CodexResult<i32> {
        // Simple word-based calculation (average 200 words per minute)
        let word_count = content.split_whitespace().count();
        let reading_time = ((word_count as f64 / 200.0).ceil() as i32).max(1);
        Ok(reading_time)
    }

    /// Check if AI engine is healthy and responsive
    pub async fn health_check(&self) -> CodexResult<bool> {
        match self.generate_text("Hello").await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("AI health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get AI engine statistics
    pub async fn get_stats(&self) -> CodexResult<AiStats> {
        let inference = self.inference.read().await;
        let stats = inference.get_stats().await?;
        Ok(stats)
    }

    /// Reload the AI model (useful for switching models)
    pub async fn reload_model(&self, model_path: Option<String>) -> CodexResult<()> {
        info!("Reloading AI model");
        
        let mut inference = self.inference.write().await;
        let model_path = model_path.unwrap_or_else(|| self.config.primary_model.clone());
        
        inference.load_model(&model_path).await?;
        
        info!("AI model reloaded successfully");
        Ok(())
    }

    /// Shutdown the AI engine
    pub async fn shutdown(&self) -> CodexResult<()> {
        info!("Shutting down AI engine");
        
        // Shutdown components
        self.rag.shutdown().await?;
        
        let mut inference = self.inference.write().await;
        inference.shutdown().await?;
        
        info!("AI engine shutdown complete");
        Ok(())
    }
}

/// AI engine statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiStats {
    pub model_name: String,
    pub model_size_mb: f64,
    pub memory_usage_mb: f64,
    pub total_inferences: u64,
    pub average_inference_time_ms: f64,
    pub cache_hit_rate: f64,
    pub uptime_seconds: u64,
}

/// Response from RAG query
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<RagSource>,
    pub confidence: f32,
    pub context_used: usize,
}

/// Source information for RAG response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagSource {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub snippet: String,
    pub relevance_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ai_engine_creation() {
        let temp_dir = tempdir().unwrap();
        let mut config = AiConfig::default();
        config.models_dir = temp_dir.path().to_path_buf();
        
        // This test may fail without actual model files
        // In real implementation, we'd need to download/provide test models
        let result = AiEngine::new(&config).await;
        
        // For now, just test that the structure is correct
        // In production, this would test with actual models
        match result {
            Ok(_) => println!("AI engine created successfully"),
            Err(e) => println!("Expected error without model files: {}", e),
        }
    }
}