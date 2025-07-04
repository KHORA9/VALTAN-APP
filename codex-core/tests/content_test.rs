//! Content module comprehensive tests
//!
//! Tests covering ContentManager, ContentParser, ContentIndexer, and SearchEngine

use std::sync::Arc;
use std::path::PathBuf;
use uuid::Uuid;
use tempfile::TempDir;
use tokio::fs;
use rstest::{rstest, fixture};
use serial_test::serial;

use codex_core::{
    CodexResult, CodexError,
    config::{ContentConfig, DatabaseConfig, AiConfig},
    content::{ContentManager, ContentParser, ContentIndexer, SearchEngine, SearchOptions, SearchType, SortBy, SortOrder},
    db::{DatabaseManager, models::Document},
    ai::AiEngine,
};

mod common;
use common::test_utils::*;

// =====================================================
// TEST FIXTURES
// =====================================================

#[fixture]
async fn test_content_config() -> ContentConfig {
    ContentConfig {
        content_dir: create_temp_dir("test_content").path().to_path_buf(),
        supported_extensions: vec!["txt".to_string(), "md".to_string(), "html".to_string()],
        max_file_size_mb: 10,
        enable_ai_metadata: true,
        auto_categorize: true,
    }
}

#[fixture]
async fn test_content_manager() -> (ContentManager, TempDir) {
    let temp_dir = create_temp_dir("content_test");
    let db_path = temp_dir.path().join("test.db");
    
    // Create database config
    let db_config = DatabaseConfig {
        database_url: format!("sqlite:{}", db_path.display()),
        max_connections: 5,
        enable_wal: true,
        enable_foreign_keys: true,
        cache_size_mb: 50,
    };
    
    // Create AI config
    let ai_config = AiConfig {
        model_path: temp_dir.path().join("test-model.gguf"),
        max_context_length: 2048,
        temperature: 0.7,
        enable_caching: true,
        cache_size_mb: 100,
    };
    
    // Create content config
    let content_config = ContentConfig {
        content_dir: temp_dir.path().join("content"),
        supported_extensions: vec!["txt".to_string(), "md".to_string(), "html".to_string()],
        max_file_size_mb: 10,
        enable_ai_metadata: true,
        auto_categorize: true,
    };
    
    // Initialize components
    let db = Arc::new(DatabaseManager::new(&db_config).await.unwrap());
    let ai = Arc::new(AiEngine::new(&ai_config).await.unwrap());
    let content_manager = ContentManager::new(db, ai, &content_config).await.unwrap();
    
    (content_manager, temp_dir)
}

// =====================================================
// CONTENT PARSER TESTS
// =====================================================

#[rstest]
#[tokio::test]
async fn test_content_parser_text_file() -> CodexResult<()> {
    let temp_dir = create_temp_dir("parser_test");
    let config = ContentConfig {
        content_dir: temp_dir.path().to_path_buf(),
        supported_extensions: vec!["txt".to_string()],
        max_file_size_mb: 10,
        enable_ai_metadata: true,
        auto_categorize: true,
    };
    
    let parser = ContentParser::new(&config)?;
    
    // Create test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "This is a test document.\nIt has multiple lines.").await?;
    
    // Parse file
    let parsed = parser.parse_file(&test_file).await?;
    
    assert_eq!(parsed.title, "test");
    assert!(parsed.content.contains("This is a test document"));
    assert_eq!(parsed.content_type, "text/plain");
    assert!(parsed.file_size > 0);
    assert!(!parsed.file_hash.is_empty());
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_content_parser_markdown_file() -> CodexResult<()> {
    let temp_dir = create_temp_dir("parser_test");
    let config = ContentConfig {
        content_dir: temp_dir.path().to_path_buf(),
        supported_extensions: vec!["md".to_string()],
        max_file_size_mb: 10,
        enable_ai_metadata: true,
        auto_categorize: true,
    };
    
    let parser = ContentParser::new(&config)?;
    
    // Create markdown test file with YAML frontmatter
    let markdown_content = r#"---
title: "Test Document"
author: "Test Author"
category: "Philosophy"
tags: ["test", "markdown"]
---

# Test Document

This is a **markdown** document with *formatting*.

## Section 1

Some content here.
"#;
    
    let test_file = temp_dir.path().join("test.md");
    fs::write(&test_file, markdown_content).await?;
    
    // Parse file
    let parsed = parser.parse_file(&test_file).await?;
    
    assert_eq!(parsed.title, "Test Document");
    assert_eq!(parsed.author, Some("Test Author".to_string()));
    assert_eq!(parsed.content_type, "text/markdown");
    assert!(parsed.content.contains("This is a **markdown** document"));
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_content_parser_file_not_found() -> CodexResult<()> {
    let temp_dir = create_temp_dir("parser_test");
    let config = ContentConfig {
        content_dir: temp_dir.path().to_path_buf(),
        supported_extensions: vec!["txt".to_string()],
        max_file_size_mb: 10,
        enable_ai_metadata: true,
        auto_categorize: true,
    };
    
    let parser = ContentParser::new(&config)?;
    let non_existent_file = temp_dir.path().join("non_existent.txt");
    
    // Should return error for non-existent file
    let result = parser.parse_file(&non_existent_file).await;
    assert!(result.is_err());
    
    Ok(())
}

// =====================================================
// CONTENT MANAGER TESTS
// =====================================================

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_import_text_content() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import text content
    let title = "Test Document".to_string();
    let content = "This is a test document with some content.".to_string();
    let content_type = Some("text/plain".to_string());
    
    let document_id = content_manager.import_text_content(title.clone(), content.clone(), content_type).await?;
    
    // Verify document was created
    assert_ne!(document_id, Uuid::nil());
    
    // Retrieve document
    let retrieved = content_manager.get_document(document_id).await?;
    assert!(retrieved.is_some());
    
    let doc = retrieved.unwrap();
    assert_eq!(doc.title, title);
    assert_eq!(doc.content, content);
    assert_eq!(doc.content_type, "text/plain");
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_import_document_file() -> CodexResult<()> {
    let (content_manager, temp_dir) = test_content_manager().await;
    
    // Create test file
    let test_file = temp_dir.path().join("import_test.txt");
    let test_content = "This is a test file for import testing.";
    fs::write(&test_file, test_content).await?;
    
    // Import document
    let document_id = content_manager.import_document(&test_file).await?;
    
    // Verify document was created
    assert_ne!(document_id, Uuid::nil());
    
    // Retrieve document
    let retrieved = content_manager.get_document(document_id).await?;
    assert!(retrieved.is_some());
    
    let doc = retrieved.unwrap();
    assert_eq!(doc.title, "import_test");
    assert_eq!(doc.content, test_content);
    assert!(doc.file_hash.is_some());
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_duplicate_detection() -> CodexResult<()> {
    let (content_manager, temp_dir) = test_content_manager().await;
    
    // Create test file
    let test_file = temp_dir.path().join("duplicate_test.txt");
    let test_content = "This is a test file for duplicate detection.";
    fs::write(&test_file, test_content).await?;
    
    // Import document first time
    let first_import = content_manager.import_document(&test_file).await?;
    assert_ne!(first_import, Uuid::nil());
    
    // Try to import again - should detect duplicate
    let second_import = content_manager.import_document(&test_file).await;
    assert!(second_import.is_err());
    
    if let Err(CodexError::Validation(msg)) = second_import {
        assert!(msg.contains("identical content already exists"));
    } else {
        panic!("Expected validation error for duplicate content");
    }
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_update_document() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import initial document
    let title = "Update Test".to_string();
    let initial_content = "Initial content".to_string();
    let document_id = content_manager.import_text_content(title, initial_content, None).await?;
    
    // Update document
    let new_content = "Updated content with new information".to_string();
    content_manager.update_document(document_id, new_content.clone()).await?;
    
    // Verify update
    let updated_doc = content_manager.get_document(document_id).await?;
    assert!(updated_doc.is_some());
    
    let doc = updated_doc.unwrap();
    assert_eq!(doc.content, new_content);
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_toggle_favorite() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import document
    let document_id = content_manager.import_text_content(
        "Favorite Test".to_string(),
        "Content for favorite testing".to_string(),
        None,
    ).await?;
    
    // Toggle favorite (should become true)
    let is_favorite = content_manager.toggle_favorite(document_id).await?;
    assert!(is_favorite);
    
    // Verify in database
    let doc = content_manager.get_document(document_id).await?.unwrap();
    assert!(doc.is_favorite);
    
    // Toggle again (should become false)
    let is_favorite = content_manager.toggle_favorite(document_id).await?;
    assert!(!is_favorite);
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_categorize_document() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import document
    let document_id = content_manager.import_text_content(
        "Category Test".to_string(),
        "Content for categorization testing".to_string(),
        None,
    ).await?;
    
    // Categorize document
    let category = "Philosophy".to_string();
    content_manager.categorize_document(document_id, category.clone()).await?;
    
    // Verify categorization
    let doc = content_manager.get_document(document_id).await?.unwrap();
    assert_eq!(doc.category, Some(category));
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_get_recent_documents() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import multiple documents
    for i in 1..=5 {
        content_manager.import_text_content(
            format!("Recent Test {}", i),
            format!("Content for recent test {}", i),
            None,
        ).await?;
        
        // Add small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    // Get recent documents
    let recent = content_manager.get_recent_documents(3).await?;
    assert_eq!(recent.len(), 3);
    
    // Should be in reverse chronological order
    assert!(recent[0].title.contains("Recent Test 5"));
    assert!(recent[1].title.contains("Recent Test 4"));
    assert!(recent[2].title.contains("Recent Test 3"));
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_get_documents_by_category() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import documents with categories
    let philosophy_id = content_manager.import_text_content(
        "Philosophy Doc".to_string(),
        "Content about philosophy".to_string(),
        None,
    ).await?;
    content_manager.categorize_document(philosophy_id, "Philosophy".to_string()).await?;
    
    let science_id = content_manager.import_text_content(
        "Science Doc".to_string(),
        "Content about science".to_string(),
        None,
    ).await?;
    content_manager.categorize_document(science_id, "Science".to_string()).await?;
    
    // Get documents by category
    let philosophy_docs = content_manager.get_documents_by_category("Philosophy", 10, 0).await?;
    assert_eq!(philosophy_docs.len(), 1);
    assert_eq!(philosophy_docs[0].title, "Philosophy Doc");
    
    let science_docs = content_manager.get_documents_by_category("Science", 10, 0).await?;
    assert_eq!(science_docs.len(), 1);
    assert_eq!(science_docs[0].title, "Science Doc");
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_get_favorite_documents() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import documents and mark some as favorites
    let doc1_id = content_manager.import_text_content(
        "Favorite Doc 1".to_string(),
        "Content for favorite 1".to_string(),
        None,
    ).await?;
    content_manager.toggle_favorite(doc1_id).await?;
    
    let _doc2_id = content_manager.import_text_content(
        "Regular Doc".to_string(),
        "Content for regular doc".to_string(),
        None,
    ).await?;
    // Don't mark as favorite
    
    let doc3_id = content_manager.import_text_content(
        "Favorite Doc 2".to_string(),
        "Content for favorite 2".to_string(),
        None,
    ).await?;
    content_manager.toggle_favorite(doc3_id).await?;
    
    // Get favorite documents
    let favorites = content_manager.get_favorite_documents(10).await?;
    assert_eq!(favorites.len(), 2);
    
    let favorite_titles: Vec<String> = favorites.iter().map(|d| d.title.clone()).collect();
    assert!(favorite_titles.contains(&"Favorite Doc 1".to_string()));
    assert!(favorite_titles.contains(&"Favorite Doc 2".to_string()));
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_delete_document() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import document
    let document_id = content_manager.import_text_content(
        "Delete Test".to_string(),
        "Content for deletion testing".to_string(),
        None,
    ).await?;
    
    // Verify document exists
    let doc = content_manager.get_document(document_id).await?;
    assert!(doc.is_some());
    
    // Delete document
    content_manager.delete_document(document_id).await?;
    
    // Verify document is deleted (soft delete should still return None for user queries)
    let deleted_doc = content_manager.get_document(document_id).await?;
    assert!(deleted_doc.is_none());
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_search_documents() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import searchable documents
    content_manager.import_text_content(
        "Philosophy of Mind".to_string(),
        "The philosophy of mind is a branch of philosophy that studies consciousness.".to_string(),
        None,
    ).await?;
    
    content_manager.import_text_content(
        "Quantum Physics".to_string(),
        "Quantum physics deals with the behavior of matter and energy at the atomic level.".to_string(),
        None,
    ).await?;
    
    // Search for documents
    let search_options = SearchOptions {
        search_type: SearchType::FullText,
        limit: 10,
        offset: 0,
        category: None,
        tags: None,
        author: None,
        language: None,
        difficulty_level: None,
        date_range: None,
        similarity_threshold: Some(0.5),
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Descending,
    };
    
    let results = content_manager.search_documents("philosophy", search_options).await?;
    assert!(results.documents.len() > 0);
    assert!(results.documents[0].document.title.contains("Philosophy"));
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_get_content_stats() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import some documents
    for i in 1..=3 {
        content_manager.import_text_content(
            format!("Stats Test {}", i),
            format!("Content for stats test document {}", i),
            None,
        ).await?;
    }
    
    // Get content statistics
    let stats = content_manager.get_content_stats().await?;
    assert!(stats.total_documents >= 3);
    assert!(stats.database_size_bytes > 0);
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_health_check() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Health check should pass for properly initialized manager
    let health = content_manager.health_check().await?;
    assert!(health);
    
    Ok(())
}

// =====================================================
// EDGE CASES AND ERROR HANDLING
// =====================================================

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_get_nonexistent_document() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    let nonexistent_id = Uuid::new_v4();
    let result = content_manager.get_document(nonexistent_id).await?;
    assert!(result.is_none());
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_update_nonexistent_document() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    let nonexistent_id = Uuid::new_v4();
    let result = content_manager.update_document(nonexistent_id, "New content".to_string()).await;
    
    assert!(result.is_err());
    if let Err(CodexError::NotFound(msg)) = result {
        assert!(msg.contains("Document not found"));
    } else {
        panic!("Expected NotFound error");
    }
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_content_manager_large_content() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Create large content (but within limits)
    let large_content = "a".repeat(1000); // 1KB content
    let document_id = content_manager.import_text_content(
        "Large Content Test".to_string(),
        large_content.clone(),
        None,
    ).await?;
    
    // Verify large content was stored correctly
    let doc = content_manager.get_document(document_id).await?.unwrap();
    assert_eq!(doc.content.len(), large_content.len());
    
    Ok(())
}

// =====================================================
// PERFORMANCE TESTS
// =====================================================

#[rstest]
#[tokio::test]
#[serial]
async fn test_content_manager_bulk_import_performance() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    let start_time = std::time::Instant::now();
    
    // Import multiple documents
    for i in 1..=10 {
        content_manager.import_text_content(
            format!("Bulk Test {}", i),
            format!("Content for bulk import test document number {}", i),
            None,
        ).await?;
    }
    
    let elapsed = start_time.elapsed();
    
    // Should complete within reasonable time (adjust threshold as needed)
    assert!(elapsed.as_secs() < 30, "Bulk import took too long: {:?}", elapsed);
    
    // Verify all documents were imported
    let recent = content_manager.get_recent_documents(15).await?;
    assert!(recent.len() >= 10);
    
    Ok(())
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_search_performance() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    
    // Import documents for search testing
    for i in 1..=20 {
        content_manager.import_text_content(
            format!("Search Performance Test {}", i),
            format!("This is content for search performance testing document {}. It contains various keywords and phrases.", i),
            None,
        ).await?;
    }
    
    let search_options = SearchOptions {
        search_type: SearchType::FullText,
        limit: 10,
        offset: 0,
        category: None,
        tags: None,
        author: None,
        language: None,
        difficulty_level: None,
        date_range: None,
        similarity_threshold: Some(0.5),
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Descending,
    };
    
    let start_time = std::time::Instant::now();
    let results = content_manager.search_documents("performance testing", search_options).await?;
    let search_time = start_time.elapsed();
    
    // Search should be fast (sub-second for small dataset)
    assert!(search_time.as_millis() < 1000, "Search took too long: {:?}", search_time);
    assert!(results.documents.len() > 0);
    
    Ok(())
}

// =====================================================
// CONCURRENT ACCESS TESTS
// =====================================================

#[rstest]
#[tokio::test]
#[serial]
async fn test_concurrent_document_operations() -> CodexResult<()> {
    let (content_manager, _temp_dir) = test_content_manager().await;
    let content_manager = Arc::new(content_manager);
    
    // Spawn multiple concurrent import tasks
    let mut tasks = Vec::new();
    
    for i in 1..=5 {
        let cm = Arc::clone(&content_manager);
        let task = tokio::spawn(async move {
            cm.import_text_content(
                format!("Concurrent Test {}", i),
                format!("Content for concurrent test {}", i),
                None,
            ).await
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let mut successful_imports = 0;
    for task in tasks {
        if let Ok(Ok(_)) = task.await {
            successful_imports += 1;
        }
    }
    
    // All imports should succeed
    assert_eq!(successful_imports, 5);
    
    // Verify all documents were created
    let recent = content_manager.get_recent_documents(10).await?;
    assert!(recent.len() >= 5);
    
    Ok(())
}