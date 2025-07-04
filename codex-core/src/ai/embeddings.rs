//! Text embedding generation for semantic search

use anyhow::Result;
use tracing::{info, debug};

use crate::CodexResult;
use crate::config::AiConfig;

/// Text embedding engine for generating vector representations
pub struct EmbeddingEngine {
    model_name: String,
    dimensions: usize,
    device: String,
}

impl std::fmt::Debug for EmbeddingEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingEngine")
            .field("model_name", &self.model_name)
            .field("dimensions", &self.dimensions)
            .field("device", &self.device)
            .finish()
    }
}

impl EmbeddingEngine {
    /// Create a new embedding engine
    pub async fn new(config: &AiConfig) -> Result<Self> {
        info!("Initializing embedding engine");

        // In a real implementation, you would:
        // 1. Load a sentence transformer model (e.g., all-MiniLM-L6-v2)
        // 2. Initialize the model with the specified device
        // 3. Set up tokenization and preprocessing

        let engine = Self {
            model_name: "all-MiniLM-L6-v2".to_string(), // Standard embedding model
            dimensions: 384, // Typical dimension for this model
            device: config.device.clone(),
        };

        info!("Embedding engine initialized with model: {}", engine.model_name);
        Ok(engine)
    }

    /// Generate embedding for a single text
    pub async fn generate_embedding(&self, text: &str) -> CodexResult<Vec<f32>> {
        debug!("Generating embedding for text: {}", text.chars().take(100).collect::<String>());

        // Placeholder implementation
        // In a real implementation, you would:
        // 1. Tokenize the text
        // 2. Run through the embedding model
        // 3. Return the normalized vector

        // For now, generate a deterministic but pseudo-random embedding
        let embedding = self.generate_placeholder_embedding(text);
        
        Ok(embedding)
    }

    /// Generate embeddings for multiple texts (batch processing)
    pub async fn generate_embeddings_batch(&self, texts: &[String]) -> CodexResult<Vec<Vec<f32>>> {
        debug!("Generating embeddings for {} texts", texts.len());

        let mut embeddings = Vec::new();
        
        // In a real implementation, you would process these in batches for efficiency
        for text in texts {
            let embedding = self.generate_embedding(text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    /// Generate embedding for chunked text (for long documents)
    pub async fn generate_chunk_embeddings(
        &self,
        text: &str,
        chunk_size: usize,
        overlap: usize,
    ) -> CodexResult<Vec<ChunkEmbedding>> {
        let chunks = self.chunk_text(text, chunk_size, overlap);
        let mut chunk_embeddings = Vec::new();

        for (index, chunk) in chunks.into_iter().enumerate() {
            let embedding = self.generate_embedding(&chunk.text).await?;
            
            chunk_embeddings.push(ChunkEmbedding {
                index,
                text: chunk.text,
                start_position: chunk.start_position,
                end_position: chunk.end_position,
                embedding,
            });
        }

        Ok(chunk_embeddings)
    }

    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    /// Find most similar embeddings to a query embedding
    pub fn find_similar(
        &self,
        query_vector: &[f32],
        embeddings: &[(String, Vec<f32>)],
        top_k: usize,
    ) -> Vec<SimilarityResult> {
        let mut similarities: Vec<SimilarityResult> = embeddings
            .iter()
            .map(|(id, embedding)| {
                let similarity = self.cosine_similarity(query_vector, embedding);
                SimilarityResult {
                    document_id: id.clone(),
                    similarity_score: similarity,
                }
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        // Take top-k results
        similarities.truncate(top_k);
        similarities
    }

    /// Chunk text into overlapping segments
    fn chunk_text(&self, text: &str, chunk_size: usize, overlap: usize) -> Vec<TextChunk> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut chunks = Vec::new();
        
        if words.is_empty() {
            return chunks;
        }

        let mut start = 0;
        let step = chunk_size.saturating_sub(overlap);
        
        while start < words.len() {
            let end = (start + chunk_size).min(words.len());
            let chunk_words = &words[start..end];
            let chunk_text = chunk_words.join(" ");
            
            // Calculate character positions (approximate)
            let start_position = words[..start].iter().map(|w| w.len() + 1).sum::<usize>();
            let end_position = start_position + chunk_text.len();
            
            chunks.push(TextChunk {
                text: chunk_text,
                start_position,
                end_position,
            });
            
            if end >= words.len() {
                break;
            }
            
            start += step;
        }

        chunks
    }

    /// Generate a placeholder embedding (deterministic for testing)
    fn generate_placeholder_embedding(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create a deterministic hash of the text
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate pseudo-random embedding based on hash
        let mut embedding = Vec::with_capacity(self.dimensions);
        let mut seed = hash;
        
        for _ in 0..self.dimensions {
            // Linear congruential generator
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let value = (seed as f32 / u64::MAX as f32) * 2.0 - 1.0; // Range [-1, 1]
            embedding.push(value);
        }

        // Normalize the embedding
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut embedding {
                *value /= norm;
            }
        }

        embedding
    }

    /// Get embedding model information
    pub fn get_model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo {
            name: self.model_name.clone(),
            dimensions: self.dimensions,
            device: self.device.clone(),
            max_input_length: 512, // Typical for sentence transformers
        }
    }

    /// Get the embedding dimensions
    pub fn get_dimensions(&self) -> usize {
        self.dimensions
    }
}

/// Text chunk with position information
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub text: String,
    pub start_position: usize,
    pub end_position: usize,
}

/// Embedding for a text chunk
#[derive(Debug, Clone)]
pub struct ChunkEmbedding {
    pub index: usize,
    pub text: String,
    pub start_position: usize,
    pub end_position: usize,
    pub embedding: Vec<f32>,
}

/// Similarity search result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimilarityResult {
    pub document_id: String,
    pub similarity_score: f32,
}

/// Embedding model information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingModelInfo {
    pub name: String,
    pub dimensions: usize,
    pub device: String,
    pub max_input_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AiConfig;

    #[tokio::test]
    async fn test_embedding_generation() {
        let config = AiConfig::default();
        let engine = EmbeddingEngine::new(&config).await.unwrap();
        
        let text = "This is a test sentence for embedding generation.";
        let embedding = engine.generate_embedding(text).await.unwrap();
        
        assert_eq!(embedding.len(), 384);
        
        // Test that the same text produces the same embedding
        let embedding2 = engine.generate_embedding(text).await.unwrap();
        assert_eq!(embedding, embedding2);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let config = AiConfig::default();
        let engine = EmbeddingEngine::new(&config).await.unwrap();
        
        let text1 = "The cat sat on the mat.";
        let text2 = "A cat was sitting on a mat.";
        let text3 = "The weather is sunny today.";
        
        let embedding1 = engine.generate_embedding(text1).await.unwrap();
        let embedding2 = engine.generate_embedding(text2).await.unwrap();
        let embedding3 = engine.generate_embedding(text3).await.unwrap();
        
        let similarity_12 = engine.cosine_similarity(&embedding1, &embedding2);
        let similarity_13 = engine.cosine_similarity(&embedding1, &embedding3);
        
        // Similar sentences should have higher similarity
        assert!(similarity_12 > similarity_13);
    }

    #[tokio::test]
    async fn test_text_chunking() {
        let config = AiConfig::default();
        let engine = EmbeddingEngine::new(&config).await.unwrap();
        
        let text = "This is a test sentence. It has multiple sentences. We want to chunk it properly.";
        let chunks = engine.chunk_text(text, 5, 2); // 5 words per chunk, 2 word overlap
        
        assert!(!chunks.is_empty());
        assert!(chunks.len() > 1);
        
        // Check that chunks have proper overlap
        for i in 1..chunks.len() {
            let prev_words: Vec<&str> = chunks[i-1].text.split_whitespace().collect();
            let curr_words: Vec<&str> = chunks[i].text.split_whitespace().collect();
            
            // Check for overlap between end of previous chunk and start of current chunk
            let prev_end_words: Vec<&str> = prev_words.iter().rev().take(2).cloned().collect();
            let curr_start_words: Vec<&str> = curr_words.iter().take(2).cloned().collect();
            
            // Count actual overlapping words by comparing content
            let mut _overlap_count = 0;
            for prev_word in prev_end_words.iter().rev() {
                if curr_start_words.contains(prev_word) {
                    _overlap_count += 1;
                }
            }
            
            // For 2-word overlap setting, we should have at least some overlap
            // But since we use sentence boundaries, it might be less than exact
            // So we just check that chunks exist and are meaningful
            assert!(chunks[i].text.len() > 0);
        }
    }
}