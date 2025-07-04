//! Unit tests for database models

mod common;

use common::db::{TestDatabase, DocumentBuilder};
use codex_core::{
    db::models::{Document, DocumentCreateRequest, Embedding, Setting},
    CodexResult,
};
use rstest::*;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Test document model creation and validation
#[rstest]
#[tokio::test]
async fn test_document_creation() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let request = DocumentBuilder::new()
        .title("Test Document")
        .content("This is test content for validation.")
        .category("Philosophy")
        .tags(vec!["test", "philosophy"])
        .difficulty(3)
        .build();
    
    let document = db.manager.content().create_document(request).await?;
    
    // Validate document properties
    assert_eq!(document.title, "Test Document");
    assert!(document.content.contains("test content"));
    assert_eq!(document.category, Some("Philosophy".to_string()));
    assert_eq!(document.difficulty_level, Some(3));
    assert!(!document.id.is_empty());
    assert!(document.created_at <= Utc::now());
    
    Ok(())
}

/// Test document ID generation and uniqueness
#[rstest]
#[tokio::test]
async fn test_document_id_uniqueness() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let mut document_ids = Vec::new();
    
    // Create multiple documents
    for i in 0..10 {
        let request = DocumentBuilder::new()
            .title(&format!("Document {}", i))
            .content("Test content")
            .build();
        
        let document = db.manager.content().create_document(request).await?;
        document_ids.push(document.id);
    }
    
    // Verify all IDs are unique
    document_ids.sort();
    document_ids.dedup();
    assert_eq!(document_ids.len(), 10);
    
    Ok(())
}

/// Test document tag handling
#[rstest]
#[case(None)]
#[case(Some(vec![]))]
#[case(Some(vec!["tag1".to_string()]))]
#[case(Some(vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()]))]
#[tokio::test]
async fn test_document_tags(#[case] tags: Option<Vec<String>>) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let request = DocumentCreateRequest {
        title: "Tag Test Document".to_string(),
        content: "Test content".to_string(),
        summary: None,
        author: None,
        category: None,
        tags: tags.clone(),
        language: Some("en".to_string()),
        reading_time: None,
        difficulty_level: None,
        source_url: None,
        file_path: None,
        file_hash: None,
    };
    
    let document = db.manager.content().create_document(request).await?;
    let retrieved_tags = document.get_tags();
    
    match tags {
        None => assert!(retrieved_tags.is_empty()),
        Some(expected_tags) => assert_eq!(retrieved_tags, expected_tags),
    }
    
    Ok(())
}

/// Test document validation edge cases
#[rstest]
#[case("", "Valid content", true)] // Empty title should fail
#[case("Valid title", "", true)] // Empty content should fail
#[case("A".repeat(1000), "Valid content", false)] // Very long title
#[case("Valid title", "A".repeat(100000), false)] // Very long content
#[tokio::test]
async fn test_document_validation(
    #[case] title: String,
    #[case] content: String,
    #[case] should_fail: bool,
) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let request = DocumentCreateRequest {
        title,
        content,
        summary: None,
        author: None,
        category: None,
        tags: None,
        language: Some("en".to_string()),
        reading_time: None,
        difficulty_level: None,
        source_url: None,
        file_path: None,
        file_hash: None,
    };
    
    let result = db.manager.content().create_document(request).await;
    
    if should_fail {
        assert!(result.is_err());
    } else {
        assert!(result.is_ok());
    }
    
    Ok(())
}

/// Test document update operations
#[rstest]
#[tokio::test]
async fn test_document_update() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create initial document
    let original_doc = db.create_test_document().await?;
    let original_id = original_doc.id.clone();
    
    // Update document
    let update_request = DocumentCreateRequest {
        title: "Updated Title".to_string(),
        content: "Updated content with new information.".to_string(),
        summary: Some("Updated summary".to_string()),
        author: Some("Updated Author".to_string()),
        category: Some("Science".to_string()),
        tags: Some(vec!["updated".to_string(), "science".to_string()]),
        language: Some("en".to_string()),
        reading_time: Some(300),
        difficulty_level: Some(4),
        source_url: None,
        file_path: None,
        file_hash: None,
    };
    
    let updated_doc = db.manager.content().update_document(&original_id, update_request).await?;
    
    // Verify updates
    assert_eq!(updated_doc.id, original_id); // ID should not change
    assert_eq!(updated_doc.title, "Updated Title");
    assert_eq!(updated_doc.category, Some("Science".to_string()));
    assert_eq!(updated_doc.difficulty_level, Some(4));
    assert!(updated_doc.updated_at > original_doc.updated_at);
    
    Ok(())
}

/// Test embedding model creation and operations
#[rstest]
#[tokio::test]
async fn test_embedding_creation() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create a document first
    let document = db.create_test_document().await?;
    
    // Create embedding
    let vector: Vec<f32> = (0..384).map(|i| i as f32 * 0.1).collect();
    let embedding = Embedding::create(
        &db.pool,
        &document.id,
        &vector,
        "test-model",
        384,
    ).await?;
    
    // Validate embedding
    assert_eq!(embedding.document_id, document.id);
    assert_eq!(embedding.model_name, "test-model");
    assert_eq!(embedding.dimensions, 384);
    assert!(!embedding.id.is_empty());
    
    // Verify vector storage
    let retrieved_vector = embedding.get_vector()?;
    assert_eq!(retrieved_vector.len(), 384);
    assert!((retrieved_vector[0] - 0.0).abs() < f32::EPSILON);
    assert!((retrieved_vector[10] - 1.0).abs() < f32::EPSILON);
    
    Ok(())
}

/// Test embedding vector operations
#[rstest]
#[case(vec![1.0, 0.0, 0.0], vec![1.0, 0.0, 0.0], 1.0)] // Identical vectors
#[case(vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0], 0.0)] // Orthogonal vectors
#[case(vec![1.0, 1.0, 0.0], vec![1.0, 1.0, 0.0], 1.0)] // Same direction
#[tokio::test]
async fn test_embedding_similarity(
    #[case] vector1: Vec<f32>,
    #[case] vector2: Vec<f32>,
    #[case] expected_similarity: f32,
) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create two documents
    let doc1 = db.create_test_document().await?;
    let doc2 = db.create_test_document().await?;
    
    // Create embeddings
    let embedding1 = Embedding::create(&db.pool, &doc1.id, &vector1, "test-model", vector1.len()).await?;
    let embedding2 = Embedding::create(&db.pool, &doc2.id, &vector2, "test-model", vector2.len()).await?;
    
    // Calculate similarity
    let similarity = embedding1.cosine_similarity(&embedding2)?;
    assert!((similarity - expected_similarity).abs() < 0.001);
    
    Ok(())
}

/// Test setting model operations
#[rstest]
#[tokio::test]
async fn test_setting_operations() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Test string setting
    Setting::set(&db.pool, "test_string", "test_value").await?;
    let retrieved = Setting::get(&db.pool, "test_string").await?;
    assert_eq!(retrieved, Some("test_value".to_string()));
    
    // Test integer setting
    Setting::set(&db.pool, "test_int", "42").await?;
    let int_value = Setting::get_int(&db.pool, "test_int").await?;
    assert_eq!(int_value, Some(42));
    
    // Test boolean setting
    Setting::set(&db.pool, "test_bool", "true").await?;
    let bool_value = Setting::get_bool(&db.pool, "test_bool").await?;
    assert_eq!(bool_value, Some(true));
    
    // Test non-existent setting
    let missing = Setting::get(&db.pool, "nonexistent").await?;
    assert_eq!(missing, None);
    
    // Test setting update
    Setting::set(&db.pool, "test_string", "updated_value").await?;
    let updated = Setting::get(&db.pool, "test_string").await?;
    assert_eq!(updated, Some("updated_value".to_string()));
    
    Ok(())
}

/// Test setting data type conversions
#[rstest]
#[case("42", Some(42))]
#[case("0", Some(0))]
#[case("-123", Some(-123))]
#[case("not_a_number", None)]
#[case("", None)]
#[tokio::test]
async fn test_setting_int_conversion(
    #[case] value: &str,
    #[case] expected: Option<i32>,
) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    Setting::set(&db.pool, "int_test", value).await?;
    let result = Setting::get_int(&db.pool, "int_test").await?;
    assert_eq!(result, expected);
    
    Ok(())
}

/// Test setting boolean conversions
#[rstest]
#[case("true", Some(true))]
#[case("false", Some(false))]
#[case("1", Some(true))]
#[case("0", Some(false))]
#[case("yes", Some(true))]
#[case("no", Some(false))]
#[case("invalid", None)]
#[tokio::test]
async fn test_setting_bool_conversion(
    #[case] value: &str,
    #[case] expected: Option<bool>,
) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    Setting::set(&db.pool, "bool_test", value).await?;
    let result = Setting::get_bool(&db.pool, "bool_test").await?;
    assert_eq!(result, expected);
    
    Ok(())
}

/// Test concurrent document creation
#[rstest]
#[tokio::test]
async fn test_concurrent_document_creation() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create multiple documents concurrently
    let handles: Vec<_> = (0..20)
        .map(|i| {
            let db_clone = &db;
            tokio::spawn(async move {
                let request = DocumentBuilder::new()
                    .title(&format!("Concurrent Document {}", i))
                    .content(&format!("Content for document {}", i))
                    .build();
                
                db_clone.manager.content().create_document(request).await
            })
        })
        .collect();
    
    // Wait for all tasks to complete
    let results: Vec<_> = futures::future::join_all(handles).await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    
    // Verify all documents were created successfully
    let documents: Vec<_> = results.into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    
    assert_eq!(documents.len(), 20);
    
    // Verify all documents have unique IDs
    let mut ids: Vec<_> = documents.iter().map(|d| d.id.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 20);
    
    Ok(())
}

/// Benchmark document creation performance
#[rstest]
#[tokio::test]
async fn test_document_creation_performance() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let start = std::time::Instant::now();
    
    // Create 100 documents
    for i in 0..100 {
        let request = DocumentBuilder::new()
            .title(&format!("Performance Test Document {}", i))
            .content("Standard test content for performance testing.")
            .category("Technology")
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    let duration = start.elapsed();
    let avg_per_doc = duration / 100;
    
    // Each document should be created in less than 10ms on average
    assert!(avg_per_doc.as_millis() < 10, 
        "Document creation took too long: {:?} per document", avg_per_doc);
    
    println!("Created 100 documents in {:?} (avg: {:?} per document)", duration, avg_per_doc);
    
    Ok(())
}