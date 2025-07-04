//! Unit tests for database queries

mod common;

use common::db::{TestDatabase, DocumentBuilder, SampleData};
use codex_core::{
    db::queries::*,
    db::models::{Document, DocumentCreateRequest},
    CodexResult,
};
use rstest::*;
use sqlx::Row;

/// Test basic document insertion and retrieval
#[rstest]
#[tokio::test]
async fn test_document_crud_operations() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let request = DocumentBuilder::new()
        .title("CRUD Test Document")
        .content("This document tests create, read, update, delete operations.")
        .category("Technology")
        .tags(vec!["crud", "database", "testing"])
        .difficulty(2)
        .build();
    
    // Create
    let created_doc = db.manager.content().create_document(request).await?;
    assert!(!created_doc.id.is_empty());
    
    // Read
    let retrieved_doc = db.manager.content()
        .get_document_by_id(&created_doc.id)
        .await?
        .expect("Document should exist");
    assert_eq!(retrieved_doc.title, "CRUD Test Document");
    assert_eq!(retrieved_doc.category, Some("Technology".to_string()));
    
    // Update
    let update_request = DocumentBuilder::new()
        .title("Updated CRUD Test Document")
        .content("Updated content for testing.")
        .category("Science")
        .difficulty(3)
        .build();
    
    let updated_doc = db.manager.content()
        .update_document(&created_doc.id, update_request)
        .await?;
    assert_eq!(updated_doc.title, "Updated CRUD Test Document");
    assert_eq!(updated_doc.category, Some("Science".to_string()));
    assert_eq!(updated_doc.difficulty_level, Some(3));
    
    // Delete
    db.manager.content().delete_document(&created_doc.id).await?;
    let deleted_doc = db.manager.content()
        .get_document_by_id(&created_doc.id)
        .await?;
    assert!(deleted_doc.is_none());
    
    Ok(())
}

/// Test document listing and pagination
#[rstest]
#[tokio::test]
async fn test_document_listing_pagination() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create 25 test documents
    for i in 0..25 {
        let request = DocumentBuilder::new()
            .title(&format!("Pagination Test Document {}", i + 1))
            .content(&format!("Content for document number {}", i + 1))
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    // Test first page (limit 10)
    let page1 = db.manager.content().list_documents(Some(10), Some(0)).await?;
    assert_eq!(page1.len(), 10);
    
    // Test second page
    let page2 = db.manager.content().list_documents(Some(10), Some(10)).await?;
    assert_eq!(page2.len(), 10);
    
    // Test third page (partial)
    let page3 = db.manager.content().list_documents(Some(10), Some(20)).await?;
    assert_eq!(page3.len(), 5);
    
    // Test no limit
    let all_docs = db.manager.content().list_documents(None, None).await?;
    assert!(all_docs.len() >= 25);
    
    // Verify documents are properly ordered (by creation time, newest first)
    for i in 1..page1.len() {
        assert!(page1[i-1].created_at >= page1[i].created_at);
    }
    
    Ok(())
}

/// Test document filtering by category
#[rstest]
#[tokio::test]
async fn test_document_filtering_by_category() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents in different categories
    let categories = vec!["Philosophy", "Science", "Technology", "Health"];
    for category in &categories {
        for i in 0..3 {
            let request = DocumentBuilder::new()
                .title(&format!("{} Document {}", category, i + 1))
                .content(&format!("Content about {}", category))
                .category(category)
                .build();
            
            db.manager.content().create_document(request).await?;
        }
    }
    
    // Test filtering by each category
    for category in &categories {
        let docs = db.manager.content()
            .list_documents_by_category(category, None, None)
            .await?;
        
        assert_eq!(docs.len(), 3);
        for doc in &docs {
            assert_eq!(doc.category, Some(category.to_string()));
        }
    }
    
    // Test non-existent category
    let empty_docs = db.manager.content()
        .list_documents_by_category("NonExistent", None, None)
        .await?;
    assert!(empty_docs.is_empty());
    
    Ok(())
}

/// Test document filtering by tags
#[rstest]
#[tokio::test]
async fn test_document_filtering_by_tags() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents with overlapping tags
    let docs_data = vec![
        ("Doc 1", vec!["rust", "programming", "systems"]),
        ("Doc 2", vec!["rust", "web", "backend"]),
        ("Doc 3", vec!["javascript", "web", "frontend"]),
        ("Doc 4", vec!["python", "ai", "machine-learning"]),
        ("Doc 5", vec!["rust", "ai", "performance"]),
    ];
    
    for (title, tags) in &docs_data {
        let request = DocumentBuilder::new()
            .title(title)
            .content(&format!("Content for {}", title))
            .tags(tags.iter().map(|s| s.to_string()).collect())
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    // Test single tag filter
    let rust_docs = db.manager.content()
        .list_documents_by_tags(&["rust".to_string()], None, None)
        .await?;
    assert_eq!(rust_docs.len(), 3);
    
    let web_docs = db.manager.content()
        .list_documents_by_tags(&["web".to_string()], None, None)
        .await?;
    assert_eq!(web_docs.len(), 2);
    
    // Test multiple tag filter (AND operation)
    let rust_ai_docs = db.manager.content()
        .list_documents_by_tags(&["rust".to_string(), "ai".to_string()], None, None)
        .await?;
    assert_eq!(rust_ai_docs.len(), 1);
    assert_eq!(rust_ai_docs[0].title, "Doc 5");
    
    // Test non-existent tag
    let empty_docs = db.manager.content()
        .list_documents_by_tags(&["nonexistent".to_string()], None, None)
        .await?;
    assert!(empty_docs.is_empty());
    
    Ok(())
}

/// Test recent documents query
#[rstest]
#[tokio::test]
async fn test_recent_documents() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents with some delay between them
    let mut created_docs = Vec::new();
    for i in 0..5 {
        let request = DocumentBuilder::new()
            .title(&format!("Recent Document {}", i + 1))
            .content(&format!("Content for recent document {}", i + 1))
            .build();
        
        let doc = db.manager.content().create_document(request).await?;
        created_docs.push(doc);
        
        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    // Get recent documents
    let recent = db.manager.content().get_recent_documents(3).await?;
    assert_eq!(recent.len(), 3);
    
    // Should be ordered by creation time (newest first)
    assert_eq!(recent[0].title, "Recent Document 5");
    assert_eq!(recent[1].title, "Recent Document 4");
    assert_eq!(recent[2].title, "Recent Document 3");
    
    // Test with larger limit than available documents
    let all_recent = db.manager.content().get_recent_documents(10).await?;
    assert!(all_recent.len() >= 5);
    
    Ok(())
}

/// Test document view count tracking
#[rstest]
#[tokio::test]
async fn test_document_view_tracking() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let document = db.create_test_document().await?;
    assert_eq!(document.view_count, 0);
    
    // Increment view count multiple times
    for expected_count in 1..=5 {
        db.manager.content().increment_view_count(&document.id).await?;
        
        let updated_doc = db.manager.content()
            .get_document_by_id(&document.id)
            .await?
            .unwrap();
        assert_eq!(updated_doc.view_count, expected_count);
    }
    
    Ok(())
}

/// Test favorite document operations
#[rstest]
#[tokio::test]
async fn test_favorite_operations() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let document = db.create_test_document().await?;
    assert!(!document.is_favorite);
    
    // Set as favorite
    db.manager.content().set_favorite(&document.id, true).await?;
    let updated_doc = db.manager.content()
        .get_document_by_id(&document.id)
        .await?
        .unwrap();
    assert!(updated_doc.is_favorite);
    
    // Unset favorite
    db.manager.content().set_favorite(&document.id, false).await?;
    let updated_doc = db.manager.content()
        .get_document_by_id(&document.id)
        .await?
        .unwrap();
    assert!(!updated_doc.is_favorite);
    
    // Test listing favorite documents
    let doc1 = db.create_test_document().await?;
    let doc2 = db.create_test_document().await?;
    let doc3 = db.create_test_document().await?;
    
    db.manager.content().set_favorite(&doc1.id, true).await?;
    db.manager.content().set_favorite(&doc3.id, true).await?;
    
    let favorites = db.manager.content().get_favorite_documents(None, None).await?;
    assert_eq!(favorites.len(), 2);
    
    let favorite_ids: Vec<_> = favorites.iter().map(|d| &d.id).collect();
    assert!(favorite_ids.contains(&&doc1.id));
    assert!(favorite_ids.contains(&&doc3.id));
    assert!(!favorite_ids.contains(&&doc2.id));
    
    Ok(())
}

/// Test document statistics queries
#[rstest]
#[tokio::test]
async fn test_document_statistics() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents with known properties
    let docs_data = vec![
        ("Category", "Philosophy", vec!["philosophy", "ethics"], 3),
        ("Category", "Philosophy", vec!["philosophy", "logic"], 4),
        ("Category", "Science", vec!["physics", "quantum"], 5),
        ("Category", "Science", vec!["biology", "genetics"], 2),
        ("Category", "Technology", vec!["programming", "rust"], 3),
    ];
    
    for (_, category, tags, difficulty) in &docs_data {
        let request = DocumentBuilder::new()
            .title("Stats Test Document")
            .content("Content for statistics testing")
            .category(category)
            .tags(tags.iter().map(|s| s.to_string()).collect())
            .difficulty(*difficulty)
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    // Test total document count
    let total_count = db.manager.content().get_document_count().await?;
    assert!(total_count >= 5);
    
    // Test count by category
    let philosophy_count = db.manager.content()
        .get_document_count_by_category("Philosophy")
        .await?;
    assert_eq!(philosophy_count, 2);
    
    let science_count = db.manager.content()
        .get_document_count_by_category("Science")
        .await?;
    assert_eq!(science_count, 2);
    
    // Test category list
    let categories = db.manager.content().get_categories().await?;
    assert!(categories.contains(&"Philosophy".to_string()));
    assert!(categories.contains(&"Science".to_string()));
    assert!(categories.contains(&"Technology".to_string()));
    
    Ok(())
}

/// Test batch document operations
#[rstest]
#[tokio::test]
async fn test_batch_operations() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create multiple documents for batch testing
    let documents = db.create_test_documents(10).await?;
    let doc_ids: Vec<String> = documents.iter().map(|d| d.id.clone()).collect();
    
    // Test batch retrieval
    let retrieved_docs = db.manager.content()
        .get_documents_by_ids(&doc_ids)
        .await?;
    assert_eq!(retrieved_docs.len(), 10);
    
    // Test batch favorite setting
    let favorite_ids = &doc_ids[0..5];
    db.manager.content()
        .set_favorites_batch(favorite_ids, true)
        .await?;
    
    let favorites = db.manager.content().get_favorite_documents(None, None).await?;
    assert_eq!(favorites.len(), 5);
    
    // Test batch deletion
    let delete_ids = &doc_ids[7..10];
    db.manager.content()
        .delete_documents_batch(delete_ids)
        .await?;
    
    // Verify deletion
    for id in delete_ids {
        let doc = db.manager.content().get_document_by_id(id).await?;
        assert!(doc.is_none());
    }
    
    // Verify remaining documents still exist
    for id in &doc_ids[0..7] {
        let doc = db.manager.content().get_document_by_id(id).await?;
        assert!(doc.is_some());
    }
    
    Ok(())
}

/// Test query performance under load
#[rstest]
#[tokio::test]
async fn test_query_performance() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create a substantial number of documents
    let doc_count = 1000;
    println!("Creating {} documents for performance testing...", doc_count);
    
    let start = std::time::Instant::now();
    for i in 0..doc_count {
        let request = DocumentBuilder::new()
            .title(&format!("Performance Test Document {}", i + 1))
            .content(&format!("Content for performance testing document number {}", i + 1))
            .category(if i % 3 == 0 { "Philosophy" } else if i % 3 == 1 { "Science" } else { "Technology" })
            .tags(vec![
                format!("tag{}", i % 10),
                "performance".to_string(),
                "test".to_string(),
            ])
            .difficulty((i % 5) + 1)
            .build();
        
        db.manager.content().create_document(request).await?;
        
        // Log progress every 100 documents
        if (i + 1) % 100 == 0 {
            println!("Created {} documents", i + 1);
        }
    }
    
    let creation_time = start.elapsed();
    println!("Created {} documents in {:?}", doc_count, creation_time);
    
    // Test retrieval performance
    let start = std::time::Instant::now();
    let all_docs = db.manager.content().list_documents(None, None).await?;
    let retrieval_time = start.elapsed();
    
    assert!(all_docs.len() >= doc_count);
    assert!(retrieval_time.as_millis() < 500, "Document retrieval took too long: {:?}", retrieval_time);
    
    // Test category filtering performance
    let start = std::time::Instant::now();
    let philosophy_docs = db.manager.content()
        .list_documents_by_category("Philosophy", None, None)
        .await?;
    let category_filter_time = start.elapsed();
    
    assert!(philosophy_docs.len() > 0);
    assert!(category_filter_time.as_millis() < 100, "Category filtering took too long: {:?}", category_filter_time);
    
    // Test pagination performance
    let start = std::time::Instant::now();
    let page = db.manager.content().list_documents(Some(50), Some(500)).await?;
    let pagination_time = start.elapsed();
    
    assert_eq!(page.len(), 50);
    assert!(pagination_time.as_millis() < 50, "Pagination took too long: {:?}", pagination_time);
    
    println!("Performance test completed successfully");
    println!("- Document retrieval: {:?}", retrieval_time);
    println!("- Category filtering: {:?}", category_filter_time);
    println!("- Pagination: {:?}", pagination_time);
    
    Ok(())
}

/// Test transaction handling and rollback
#[rstest]
#[tokio::test]
async fn test_transaction_handling() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    let initial_count = db.document_count().await?;
    
    // Simulate a failed transaction
    let result = async {
        let mut tx = db.pool.begin().await?;
        
        // Insert a document
        let doc_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO documents (id, title, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&doc_id)
        .bind("Transaction Test")
        .bind("Test content")
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(&mut *tx)
        .await?;
        
        // Simulate an error that causes rollback
        Err::<(), codex_core::CodexError>(anyhow::anyhow!("Simulated transaction failure").into())
    }.await;
    
    // Verify the transaction was rolled back
    assert!(result.is_err());
    let final_count = db.document_count().await?;
    assert_eq!(initial_count, final_count);
    
    Ok(())
}