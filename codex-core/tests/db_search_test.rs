//! Unit tests for database search functionality

mod common;

use common::db::{TestDatabase, DocumentBuilder, SampleData};
use common::fixtures::{TestFixtures, SearchTestCase};
use codex_core::{
    db::search::*,
    content::{SearchOptions, SearchType, SortBy, SortOrder},
    CodexResult,
};
use rstest::*;

/// Test FTS5 full-text search functionality
#[rstest]
#[tokio::test]
async fn test_fts5_full_text_search() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents with searchable content
    let docs_data = vec![
        ("Stoic Philosophy", "Stoicism teaches virtue, wisdom, and emotional resilience through rational thinking."),
        ("Quantum Physics", "Quantum mechanics describes the behavior of matter and energy at atomic scales."),
        ("Machine Learning", "Artificial intelligence uses algorithms to learn patterns from data."),
        ("Renaissance Art", "Renaissance artists developed perspective and realistic human representation."),
    ];
    
    for (title, content) in &docs_data {
        let request = DocumentBuilder::new()
            .title(title)
            .content(content)
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    // Test simple word search
    let results = Search::fts5(&db.pool, "philosophy", 10).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("Stoic"));
    
    // Test phrase search
    let results = Search::fts5(&db.pool, "\"quantum mechanics\"", 10).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("Quantum"));
    
    // Test multiple terms (OR search)
    let results = Search::fts5(&db.pool, "quantum OR art", 10).await?;
    assert_eq!(results.len(), 2);
    
    // Test wildcard search
    let results = Search::fts5(&db.pool, "learn*", 10).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("Machine"));
    
    // Test no results
    let results = Search::fts5(&db.pool, "nonexistent", 10).await?;
    assert!(results.is_empty());
    
    Ok(())
}

/// Test search ranking and relevance
#[rstest]
#[tokio::test]
async fn test_search_ranking() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents with varying relevance for the term "programming"
    let docs_data = vec![
        ("Programming Fundamentals", "Programming is the art of writing code. Programming involves logic, algorithms, and problem-solving. Programming languages include Python, Rust, and JavaScript."), // High relevance
        ("Software Development", "Software development includes programming, testing, and deployment. Good programming practices are essential."), // Medium relevance
        ("Computer Science Overview", "Computer science covers many topics including programming, databases, and networks."), // Low relevance
        ("History of Art", "Renaissance art focused on realistic representation and perspective techniques."), // No relevance
    ];
    
    for (title, content) in &docs_data {
        let request = DocumentBuilder::new()
            .title(title)
            .content(content)
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    let results = Search::fts5(&db.pool, "programming", 10).await?;
    
    // Should return 3 documents (excluding art document)
    assert_eq!(results.len(), 3);
    
    // Should be ranked by relevance (highest first)
    assert!(results[0].title.contains("Programming Fundamentals"));
    assert!(results[1].title.contains("Software Development"));
    assert!(results[2].title.contains("Computer Science"));
    
    Ok(())
}

/// Test search with filters and options
#[rstest]
#[tokio::test]
async fn test_search_with_filters() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents across different categories
    let philosophy_docs = SampleData::philosophy_docs();
    let science_docs = SampleData::science_docs();
    
    for doc_request in philosophy_docs.into_iter().chain(science_docs.into_iter()) {
        db.manager.content().create_document(doc_request).await?;
    }
    
    // Test category filter
    let options = SearchOptions {
        query: "philosophy".to_string(),
        category: Some("Philosophy".to_string()),
        tags: vec![],
        search_type: SearchType::FullText,
        limit: 10,
        offset: 0,
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Desc,
        include_content: true,
    };
    
    let results = db.manager.content().search_documents(&options.query, options).await?;
    assert!(results.documents.len() > 0);
    
    // All results should be in Philosophy category
    for doc in &results.documents {
        assert_eq!(doc.category, Some("Philosophy".to_string()));
    }
    
    // Test tag filter
    let options = SearchOptions {
        query: "quantum".to_string(),
        category: None,
        tags: vec!["quantum".to_string()],
        search_type: SearchType::FullText,
        limit: 10,
        offset: 0,
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Desc,
        include_content: true,
    };
    
    let results = db.manager.content().search_documents(&options.query, options).await?;
    assert!(results.documents.len() > 0);
    
    // All results should have the quantum tag
    for doc in &results.documents {
        let tags = doc.get_tags();
        assert!(tags.contains(&"quantum".to_string()));
    }
    
    Ok(())
}

/// Test search pagination
#[rstest]
#[tokio::test]
async fn test_search_pagination() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create many documents with similar content
    for i in 0..25 {
        let request = DocumentBuilder::new()
            .title(&format!("Search Test Document {}", i + 1))
            .content("This document contains the searchable term 'testing' for pagination tests.")
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    // Test first page
    let options = SearchOptions {
        query: "testing".to_string(),
        category: None,
        tags: vec![],
        search_type: SearchType::FullText,
        limit: 10,
        offset: 0,
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Desc,
        include_content: false,
    };
    
    let page1 = db.manager.content().search_documents(&options.query, options).await?;
    assert_eq!(page1.documents.len(), 10);
    assert!(page1.total_count >= 25);
    
    // Test second page
    let options = SearchOptions {
        query: "testing".to_string(),
        category: None,
        tags: vec![],
        search_type: SearchType::FullText,
        limit: 10,
        offset: 10,
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Desc,
        include_content: false,
    };
    
    let page2 = db.manager.content().search_documents(&options.query, options).await?;
    assert_eq!(page2.documents.len(), 10);
    
    // Test third page (partial)
    let options = SearchOptions {
        query: "testing".to_string(),
        category: None,
        tags: vec![],
        search_type: SearchType::FullText,
        limit: 10,
        offset: 20,
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Desc,
        include_content: false,
    };
    
    let page3 = db.manager.content().search_documents(&options.query, options).await?;
    assert!(page3.documents.len() >= 5);
    
    // Verify no document ID overlap between pages
    let page1_ids: Vec<_> = page1.documents.iter().map(|d| &d.id).collect();
    let page2_ids: Vec<_> = page2.documents.iter().map(|d| &d.id).collect();
    let page3_ids: Vec<_> = page3.documents.iter().map(|d| &d.id).collect();
    
    for id in &page1_ids {
        assert!(!page2_ids.contains(id));
        assert!(!page3_ids.contains(id));
    }
    
    for id in &page2_ids {
        assert!(!page3_ids.contains(id));
    }
    
    Ok(())
}

/// Test search sorting options
#[rstest]
#[case(SortBy::Relevance, SortOrder::Desc)]
#[case(SortBy::CreatedAt, SortOrder::Desc)]
#[case(SortBy::CreatedAt, SortOrder::Asc)]
#[case(SortBy::Title, SortOrder::Asc)]
#[case(SortBy::Title, SortOrder::Desc)]
#[tokio::test]
async fn test_search_sorting(#[case] sort_by: SortBy, #[case] sort_order: SortOrder) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create documents with known properties for sorting
    let docs_data = vec![
        ("Alpha Document", "Content with searchable terms"),
        ("Beta Document", "More content with searchable terms"),
        ("Gamma Document", "Additional content with searchable terms"),
    ];
    
    let mut doc_ids = Vec::new();
    for (title, content) in &docs_data {
        let request = DocumentBuilder::new()
            .title(title)
            .content(content)
            .build();
        
        let doc = db.manager.content().create_document(request).await?;
        doc_ids.push(doc.id);
        
        // Small delay to ensure different creation times
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    
    let options = SearchOptions {
        query: "searchable".to_string(),
        category: None,
        tags: vec![],
        search_type: SearchType::FullText,
        limit: 10,
        offset: 0,
        sort_by,
        sort_order,
        include_content: true,
    };
    
    let results = db.manager.content().search_documents(&options.query, options).await?;
    assert_eq!(results.documents.len(), 3);
    
    // Verify sorting
    match sort_by {
        SortBy::Title => {
            match sort_order {
                SortOrder::Asc => {
                    assert!(results.documents[0].title < results.documents[1].title);
                    assert!(results.documents[1].title < results.documents[2].title);
                },
                SortOrder::Desc => {
                    assert!(results.documents[0].title > results.documents[1].title);
                    assert!(results.documents[1].title > results.documents[2].title);
                },
            }
        },
        SortBy::CreatedAt => {
            match sort_order {
                SortOrder::Asc => {
                    assert!(results.documents[0].created_at <= results.documents[1].created_at);
                    assert!(results.documents[1].created_at <= results.documents[2].created_at);
                },
                SortOrder::Desc => {
                    assert!(results.documents[0].created_at >= results.documents[1].created_at);
                    assert!(results.documents[1].created_at >= results.documents[2].created_at);
                },
            }
        },
        SortBy::Relevance => {
            // For relevance, descending should be default (highest relevance first)
            // Ascending relevance is unusual but should still work
        },
    }
    
    Ok(())
}

/// Test search performance under load
#[rstest]
#[tokio::test]
async fn test_search_performance() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create a large number of documents for performance testing
    let doc_count = 5000;
    println!("Creating {} documents for search performance testing...", doc_count);
    
    let search_terms = vec!["technology", "science", "philosophy", "health", "education"];
    let categories = vec!["Technology", "Science", "Philosophy", "Health", "Education"];
    
    for i in 0..doc_count {
        let term_index = i % search_terms.len();
        let category_index = i % categories.len();
        
        let request = DocumentBuilder::new()
            .title(&format!("Performance Test Document {}", i + 1))
            .content(&format!(
                "This document is about {} and contains various terms for performance testing. \
                The content includes information about {} and related topics.",
                search_terms[term_index],
                search_terms[term_index]
            ))
            .category(&categories[category_index])
            .tags(vec![search_terms[term_index].to_string(), "performance".to_string()])
            .build();
        
        db.manager.content().create_document(request).await?;
        
        if (i + 1) % 1000 == 0 {
            println!("Created {} documents", i + 1);
        }
    }
    
    println!("Testing search performance...");
    
    // Test simple search performance
    let start = std::time::Instant::now();
    let results = Search::fts5(&db.pool, "technology", 50).await?;
    let simple_search_time = start.elapsed();
    
    assert!(results.len() > 0);
    assert!(simple_search_time.as_millis() < 200, "Simple search took too long: {:?}", simple_search_time);
    
    // Test complex search with filters
    let start = std::time::Instant::now();
    let options = SearchOptions {
        query: "science OR technology".to_string(),
        category: Some("Science".to_string()),
        tags: vec!["science".to_string()],
        search_type: SearchType::FullText,
        limit: 100,
        offset: 0,
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Desc,
        include_content: false,
    };
    
    let filtered_results = db.manager.content().search_documents(&options.query, options).await?;
    let filtered_search_time = start.elapsed();
    
    assert!(filtered_results.documents.len() > 0);
    assert!(filtered_search_time.as_millis() < 300, "Filtered search took too long: {:?}", filtered_search_time);
    
    // Test pagination performance
    let start = std::time::Instant::now();
    let options = SearchOptions {
        query: "performance".to_string(),
        category: None,
        tags: vec![],
        search_type: SearchType::FullText,
        limit: 50,
        offset: 1000,
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Desc,
        include_content: false,
    };
    
    let paginated_results = db.manager.content().search_documents(&options.query, options).await?;
    let pagination_time = start.elapsed();
    
    assert!(paginated_results.documents.len() > 0);
    assert!(pagination_time.as_millis() < 100, "Paginated search took too long: {:?}", pagination_time);
    
    println!("Search performance test completed successfully");
    println!("- Simple search (50 results): {:?}", simple_search_time);
    println!("- Filtered search (100 results): {:?}", filtered_search_time);
    println!("- Paginated search (offset 1000): {:?}", pagination_time);
    
    Ok(())
}

/// Test search with special characters and edge cases
#[rstest]
#[case("test")]
#[case("test123")]
#[case("test-case")]
#[case("test_case")]
#[case("test.case")]
#[case("test@example")]
#[case("test's")]
#[case("test\"quoted\"")]
#[tokio::test]
async fn test_search_special_characters(#[case] search_term: &str) -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create a document containing the search term
    let request = DocumentBuilder::new()
        .title("Special Characters Test")
        .content(&format!("This document contains the term: {}", search_term))
        .build();
    
    db.manager.content().create_document(request).await?;
    
    // Test searching for the term (should handle special characters gracefully)
    let results = Search::fts5(&db.pool, &search_term.replace('"', ""), 10).await;
    
    // Search should not fail, even with special characters
    assert!(results.is_ok());
    
    let results = results.unwrap();
    if !search_term.contains('"') && !search_term.contains('@') {
        // Should find the document for most terms (excluding quotes and @ which have special meaning in FTS)
        assert!(results.len() > 0 || search_term.contains('"') || search_term.contains('@'));
    }
    
    Ok(())
}

/// Test search index consistency
#[rstest]
#[tokio::test]
async fn test_search_index_consistency() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create a document
    let request = DocumentBuilder::new()
        .title("Index Consistency Test")
        .content("Original content for index testing")
        .build();
    
    let document = db.manager.content().create_document(request).await?;
    
    // Verify it's searchable
    let results = Search::fts5(&db.pool, "original", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, document.id);
    
    // Update the document
    let update_request = DocumentBuilder::new()
        .title("Updated Index Test")
        .content("Modified content for index testing")
        .build();
    
    let updated_doc = db.manager.content()
        .update_document(&document.id, update_request)
        .await?;
    
    // Old content should not be searchable
    let old_results = Search::fts5(&db.pool, "original", 10).await?;
    assert!(old_results.is_empty());
    
    // New content should be searchable
    let new_results = Search::fts5(&db.pool, "modified", 10).await?;
    assert_eq!(new_results.len(), 1);
    assert_eq!(new_results[0].id, updated_doc.id);
    assert_eq!(new_results[0].title, "Updated Index Test");
    
    // Delete the document
    db.manager.content().delete_document(&document.id).await?;
    
    // Should no longer be searchable
    let deleted_results = Search::fts5(&db.pool, "modified", 10).await?;
    assert!(deleted_results.is_empty());
    
    Ok(())
}

/// Test comprehensive search scenarios using test fixtures
#[rstest]
#[tokio::test]
async fn test_comprehensive_search_scenarios() -> CodexResult<()> {
    let db = TestDatabase::new().await?;
    
    // Create all test documents from fixtures
    let test_documents = TestFixtures::all_test_documents();
    for test_doc in test_documents {
        let request = DocumentBuilder::new()
            .title(&test_doc.title)
            .content(&test_doc.content)
            .category(&test_doc.category)
            .tags(test_doc.tags)
            .difficulty(test_doc.difficulty)
            .build();
        
        db.manager.content().create_document(request).await?;
    }
    
    // Run all search test cases
    let search_test_cases = TestFixtures::search_test_cases();
    for test_case in search_test_cases {
        let results = Search::fts5(&db.pool, &test_case.query, 20).await?;
        
        // Verify minimum expected results
        assert!(
            results.len() >= test_case.expected_min_results,
            "Query '{}' returned {} results, expected at least {}",
            test_case.query,
            results.len(),
            test_case.expected_min_results
        );
        
        // Verify expected categories are present
        let result_categories: Vec<_> = results
            .iter()
            .filter_map(|doc| doc.category.as_ref())
            .collect();
        
        for expected_category in &test_case.expected_categories {
            assert!(
                result_categories.contains(&expected_category),
                "Query '{}' should return documents from category '{}'",
                test_case.query,
                expected_category
            );
        }
        
        // Verify expected tags are present in results
        let result_tags: Vec<_> = results
            .iter()
            .flat_map(|doc| doc.get_tags())
            .collect();
        
        for expected_tag in &test_case.expected_tags {
            assert!(
                result_tags.contains(expected_tag),
                "Query '{}' should return documents with tag '{}'",
                test_case.query,
                expected_tag
            );
        }
    }
    
    Ok(())
}