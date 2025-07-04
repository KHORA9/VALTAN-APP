//! Unit tests for database vector operations

mod common;

use common::db::{TestDatabase, DocumentBuilder};
use codex_core::{
    db::{vector_ops::*, models::Embedding},
    CodexResult,
};
use rstest::*;
use approx::assert_relative_eq;

/// Test basic vector similarity calculations
#[rstest]
#[case(vec![1.0, 0.0, 0.0], vec![1.0, 0.0, 0.0], 1.0)] // Identical vectors
#[case(vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0], 0.0)] // Orthogonal vectors
#[case(vec![1.0, 0.0, 0.0], vec![-1.0, 0.0, 0.0], -1.0)] // Opposite vectors
#[case(vec![1.0, 1.0, 0.0], vec![1.0, 1.0, 0.0], 1.0)] // Same direction
#[case(vec![3.0, 4.0, 0.0], vec![6.0, 8.0, 0.0], 1.0)] // Scaled same direction
#[tokio::test]
async fn test_cosine_similarity(
    #[case] vector1: Vec<f32>,
    #[case] vector2: Vec<f32>,
    #[case] expected: f32,
) -> CodexResult<()> {
    let similarity = VectorOps::cosine_similarity(&vector1, &vector2);
    assert_relative_eq!(similarity, expected, epsilon = 1e-6);
    Ok(())
}

/// Test edge cases for cosine similarity
#[rstest]
#[tokio::test]
async fn test_cosine_similarity_edge_cases() -> CodexResult<()> {
    // Zero vectors should return 0 similarity
    let zero_vec = vec![0.0, 0.0, 0.0];
    let normal_vec = vec![1.0, 2.0, 3.0];
    
    let similarity = VectorOps::cosine_similarity(&zero_vec, &normal_vec);
    assert_eq!(similarity, 0.0);
    
    // Two zero vectors should return 0
    let similarity = VectorOps::cosine_similarity(&zero_vec, &zero_vec);
    assert_eq!(similarity, 0.0);
    
    // Very small vectors should still work
    let small_vec1 = vec![1e-10, 2e-10, 3e-10];
    let small_vec2 = vec![2e-10, 4e-10, 6e-10];
    let similarity = VectorOps::cosine_similarity(&small_vec1, &small_vec2);
    assert_relative_eq!(similarity, 1.0, epsilon = 1e-6);
    
    Ok(())
}

/// Test vector normalization
#[rstest]
#[case(vec![3.0, 4.0], vec![0.6, 0.8])] // 3-4-5 triangle
#[case(vec![1.0, 1.0, 1.0], vec![0.5773502691896258, 0.5773502691896258, 0.5773502691896258])] // Unit cube diagonal
#[case(vec![5.0, 0.0, 0.0], vec![1.0, 0.0, 0.0])] // Along axis
#[tokio::test]
async fn test_vector_normalization(
    #[case] input: Vec<f32>,
    #[case] expected: Vec<f32>,
) -> CodexResult<()> {
    let normalized = VectorOps::normalize(&input);
    
    assert_eq!(normalized.len(), expected.len());
    for (actual, expected) in normalized.iter().zip(expected.iter()) {
        assert_relative_eq!(actual, expected, epsilon = 1e-6);
    }
    
    // Verify the magnitude is 1 (except for zero vectors)
    let magnitude: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
    if input.iter().any(|&x| x != 0.0) {
        assert_relative_eq!(magnitude, 1.0, epsilon = 1e-6);
    }
    
    Ok(())
}

/// Test vector storage and retrieval in database
#[rstest]
#[tokio::test]
async fn test_vector_storage_retrieval() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create a test document
    let document = db.create_test_document().await?;
    
    // Create test vector
    let original_vector: Vec<f32> = (0..512).map(|i| (i as f32) * 0.001).collect();
    
    // Store vector
    VectorOps::store_vector(
        &db.pool,
        &document.id,
        &original_vector,
        "test-model",
    ).await?;
    
    // Retrieve embedding
    let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
    assert_eq!(embeddings.len(), 1);
    
    let retrieved_vector = embeddings[0].get_vector()?;
    assert_eq!(retrieved_vector.len(), original_vector.len());
    
    // Verify vector data integrity
    for (original, retrieved) in original_vector.iter().zip(retrieved_vector.iter()) {
        assert_relative_eq!(original, retrieved, epsilon = 1e-6);
    }
    
    Ok(())
}

/// Test vector similarity search
#[rstest]
#[tokio::test]
async fn test_vector_similarity_search() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create test documents with known vector relationships
    let test_vectors = vec![
        (vec![1.0, 0.0, 0.0], "Document A"), // Base vector
        (vec![0.9, 0.1, 0.0], "Document B"), // Very similar
        (vec![0.7, 0.3, 0.0], "Document C"), // Moderately similar
        (vec![0.0, 1.0, 0.0], "Document D"), // Orthogonal
        (vec![0.0, 0.0, 1.0], "Document E"), // Also orthogonal
    ];
    
    let mut documents = Vec::new();
    for (vector, title) in &test_vectors {
        let request = DocumentBuilder::new()
            .title(title)
            .content(&format!("Content for {}", title))
            .build();
        
        let document = db.manager.content().create_document(request).await?;
        
        VectorOps::store_vector(&db.pool, &document.id, vector, "test-model").await?;
        documents.push(document);
    }
    
    // Search for documents similar to the base vector [1.0, 0.0, 0.0]
    let query_vector = vec![1.0, 0.0, 0.0];
    let similar_docs = VectorOps::find_similar_documents(
        &db.pool,
        &query_vector,
        "test-model",
        5,
        0.5, // Minimum similarity threshold
    ).await?;
    
    // Should find documents A, B, and C (but not D and E which are orthogonal)
    assert!(similar_docs.len() >= 3);
    
    // Results should be ordered by similarity (highest first)
    let similarities: Vec<f32> = similar_docs.iter().map(|(_, sim)| *sim).collect();
    for i in 1..similarities.len() {
        assert!(similarities[i-1] >= similarities[i], 
            "Results should be ordered by similarity: {} >= {}", 
            similarities[i-1], similarities[i]);
    }
    
    // The most similar should be Document A (identical vector)
    assert_relative_eq!(similarities[0], 1.0, epsilon = 1e-6);
    
    Ok(())
}

/// Test vector operations with different dimensions
#[rstest]
#[case(128)]
#[case(256)]
#[case(384)]
#[case(512)]
#[case(768)]
#[case(1024)]
#[tokio::test]
async fn test_vector_operations_different_dimensions(#[case] dimensions: usize) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create test vector of specified dimensions
    let vector: Vec<f32> = (0..dimensions)
        .map(|i| ((i as f32) / (dimensions as f32)).sin())
        .collect();
    
    let document = db.create_test_document().await?;
    
    // Store vector
    VectorOps::store_vector(&db.pool, &document.id, &vector, "test-model").await?;
    
    // Retrieve and verify
    let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
    assert_eq!(embeddings.len(), 1);
    assert_eq!(embeddings[0].dimensions, dimensions as i32);
    
    let retrieved_vector = embeddings[0].get_vector()?;
    assert_eq!(retrieved_vector.len(), dimensions);
    
    // Verify similarity with itself
    let similarity = VectorOps::cosine_similarity(&vector, &retrieved_vector);
    assert_relative_eq!(similarity, 1.0, epsilon = 1e-6);
    
    Ok(())
}

/// Test batch vector operations
#[rstest]
#[tokio::test]
async fn test_batch_vector_operations() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create multiple documents with vectors
    let batch_size = 100;
    let vector_dim = 256;
    
    let mut documents = Vec::new();
    let mut vectors = Vec::new();
    
    for i in 0..batch_size {
        let document = db.create_test_document().await?;
        
        // Create a unique vector for each document
        let vector: Vec<f32> = (0..vector_dim)
            .map(|j| ((i * vector_dim + j) as f32 * 0.001).sin())
            .collect();
        
        VectorOps::store_vector(&db.pool, &document.id, &vector, "test-model").await?;
        
        documents.push(document);
        vectors.push(vector);
    }
    
    // Test batch retrieval
    let start = std::time::Instant::now();
    let all_embeddings = VectorOps::get_all_embeddings(&db.pool, "test-model").await?;
    let retrieval_time = start.elapsed();
    
    assert_eq!(all_embeddings.len(), batch_size);
    assert!(retrieval_time.as_millis() < 500, "Batch retrieval took too long: {:?}", retrieval_time);
    
    // Test batch similarity search
    let query_vector = &vectors[0]; // Use first vector as query
    let start = std::time::Instant::now();
    let similar_docs = VectorOps::find_similar_documents(
        &db.pool,
        query_vector,
        "test-model",
        10,
        0.1,
    ).await?;
    let search_time = start.elapsed();
    
    assert!(similar_docs.len() > 0);
    assert!(search_time.as_millis() < 200, "Similarity search took too long: {:?}", search_time);
    
    // The first result should be the query document itself (highest similarity)
    assert_relative_eq!(similar_docs[0].1, 1.0, epsilon = 1e-6);
    
    Ok(())
}

/// Test vector operations with multiple models
#[rstest]
#[tokio::test]
async fn test_multiple_models() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let document = db.create_test_document().await?;
    
    // Store vectors from different models
    let models = vec!["model-a", "model-b", "model-c"];
    let vectors = vec![
        vec![1.0, 0.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0, 0.0],
        vec![0.0, 0.0, 1.0, 0.0],
    ];
    
    for (model, vector) in models.iter().zip(vectors.iter()) {
        VectorOps::store_vector(&db.pool, &document.id, vector, model).await?;
    }
    
    // Retrieve embeddings for each model
    for (model, expected_vector) in models.iter().zip(vectors.iter()) {
        let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
        let model_embedding = embeddings.iter()
            .find(|e| e.model_name == *model)
            .expect("Should find embedding for model");
        
        let retrieved_vector = model_embedding.get_vector()?;
        assert_eq!(retrieved_vector.len(), expected_vector.len());
        
        for (expected, retrieved) in expected_vector.iter().zip(retrieved_vector.iter()) {
            assert_relative_eq!(expected, retrieved, epsilon = 1e-6);
        }
    }
    
    // Test model-specific similarity search
    let query_vector = vec![0.9, 0.1, 0.0, 0.0]; // Similar to model-a vector
    let similar_docs = VectorOps::find_similar_documents(
        &db.pool,
        &query_vector,
        "model-a",
        5,
        0.5,
    ).await?;
    
    assert_eq!(similar_docs.len(), 1); // Should only find the document with model-a embedding
    
    Ok(())
}

/// Test vector update operations
#[rstest]
#[tokio::test]
async fn test_vector_updates() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let document = db.create_test_document().await?;
    
    // Store initial vector
    let initial_vector = vec![1.0, 0.0, 0.0];
    VectorOps::store_vector(&db.pool, &document.id, &initial_vector, "test-model").await?;
    
    // Verify initial storage
    let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
    assert_eq!(embeddings.len(), 1);
    let initial_retrieved = embeddings[0].get_vector()?;
    assert_eq!(initial_retrieved, initial_vector);
    
    // Update vector (should replace, not add)
    let updated_vector = vec![0.0, 1.0, 0.0];
    VectorOps::store_vector(&db.pool, &document.id, &updated_vector, "test-model").await?;
    
    // Verify update
    let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
    assert_eq!(embeddings.len(), 1); // Should still be only one embedding
    let updated_retrieved = embeddings[0].get_vector()?;
    assert_eq!(updated_retrieved, updated_vector);
    assert_ne!(updated_retrieved, initial_vector);
    
    Ok(())
}

/// Test vector deletion operations
#[rstest]
#[tokio::test]
async fn test_vector_deletion() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let document = db.create_test_document().await?;
    
    // Store vector
    let vector = vec![1.0, 2.0, 3.0];
    VectorOps::store_vector(&db.pool, &document.id, &vector, "test-model").await?;
    
    // Verify storage
    let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
    assert_eq!(embeddings.len(), 1);
    
    // Delete vector
    VectorOps::delete_vectors_for_document(&db.pool, &document.id).await?;
    
    // Verify deletion
    let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
    assert!(embeddings.is_empty());
    
    Ok(())
}

/// Test vector operations performance benchmarks
#[rstest]
#[tokio::test]
async fn test_vector_operations_performance() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let vector_count = 1000;
    let vector_dim = 384;
    
    println!("Creating {} vectors of dimension {} for performance testing...", vector_count, vector_dim);
    
    // Create test data
    let mut documents = Vec::new();
    let start = std::time::Instant::now();
    
    for i in 0..vector_count {
        let document = db.create_test_document().await?;
        
        let vector: Vec<f32> = (0..vector_dim)
            .map(|j| ((i * vector_dim + j) as f32 * 0.001).sin())
            .collect();
        
        VectorOps::store_vector(&db.pool, &document.id, &vector, "perf-model").await?;
        documents.push(document);
        
        if (i + 1) % 100 == 0 {
            println!("Created {} vectors", i + 1);
        }
    }
    
    let creation_time = start.elapsed();
    println!("Created {} vectors in {:?}", vector_count, creation_time);
    
    // Test similarity search performance
    let query_vector: Vec<f32> = (0..vector_dim).map(|i| (i as f32 * 0.001).cos()).collect();
    
    let start = std::time::Instant::now();
    let similar_docs = VectorOps::find_similar_documents(
        &db.pool,
        &query_vector,
        "perf-model",
        50,
        0.1,
    ).await?;
    let search_time = start.elapsed();
    
    assert!(similar_docs.len() > 0);
    assert!(search_time.as_millis() < 1000, "Vector similarity search took too long: {:?}", search_time);
    
    // Test batch retrieval performance
    let start = std::time::Instant::now();
    let all_embeddings = VectorOps::get_all_embeddings(&db.pool, "perf-model").await?;
    let retrieval_time = start.elapsed();
    
    assert_eq!(all_embeddings.len(), vector_count);
    assert!(retrieval_time.as_millis() < 2000, "Batch vector retrieval took too long: {:?}", retrieval_time);
    
    // Test individual cosine similarity calculation performance
    let vec1: Vec<f32> = (0..vector_dim).map(|i| (i as f32).sin()).collect();
    let vec2: Vec<f32> = (0..vector_dim).map(|i| (i as f32).cos()).collect();
    
    let start = std::time::Instant::now();
    for _ in 0..10000 {
        VectorOps::cosine_similarity(&vec1, &vec2);
    }
    let similarity_time = start.elapsed();
    
    let avg_similarity_time = similarity_time / 10000;
    assert!(avg_similarity_time.as_nanos() < 10000, "Individual similarity calculation too slow: {:?}", avg_similarity_time);
    
    println!("Vector operations performance test completed successfully");
    println!("- Vector creation: {:?}", creation_time);
    println!("- Similarity search (50 results): {:?}", search_time);
    println!("- Batch retrieval: {:?}", retrieval_time);
    println!("- Average similarity calculation: {:?}", avg_similarity_time);
    
    Ok(())
}

/// Test vector data integrity and precision
#[rstest]
#[tokio::test]
async fn test_vector_data_integrity() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Test with various precision values
    let test_vectors = vec![
        vec![1.0, 0.5, 0.25, 0.125], // Powers of 2
        vec![1.0/3.0, 2.0/3.0, 1.0], // Fractions
        vec![std::f32::consts::PI, std::f32::consts::E, std::f32::consts::SQRT_2], // Mathematical constants
        vec![1e-6, 1e-3, 1e3, 1e6], // Different scales
        vec![f32::MIN_POSITIVE, 1.0, f32::MAX / 1e10], // Extreme values (avoiding overflow)
    ];
    
    for (i, original_vector) in test_vectors.iter().enumerate() {
        let document = db.create_test_document().await?;
        
        // Store vector
        VectorOps::store_vector(&db.pool, &document.id, original_vector, "precision-test").await?;
        
        // Retrieve vector
        let embeddings = VectorOps::get_embeddings_for_document(&db.pool, &document.id).await?;
        assert_eq!(embeddings.len(), 1);
        
        let retrieved_vector = embeddings[0].get_vector()?;
        assert_eq!(retrieved_vector.len(), original_vector.len());
        
        // Verify precision (allowing for some floating-point error)
        for (j, (original, retrieved)) in original_vector.iter().zip(retrieved_vector.iter()).enumerate() {
            assert_relative_eq!(
                original, retrieved, 
                epsilon = 1e-6,
                "Vector {}, component {}: {} != {}", i, j, original, retrieved
            );
        }
        
        // Verify cosine similarity with itself is 1.0
        let self_similarity = VectorOps::cosine_similarity(original_vector, &retrieved_vector);
        assert_relative_eq!(self_similarity, 1.0, epsilon = 1e-6);
    }
    
    Ok(())
}