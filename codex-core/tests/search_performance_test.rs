//! Search performance tests for database operations
//!
//! This module tests that search operations meet the <200ms performance target

use codex_core::db::*;
use codex_core::config::DatabaseConfig;
use sqlx::SqlitePool;
use std::time::Instant;
use tempfile::tempdir;
use tokio;

/// Test utilities for database setup
struct TestDatabase {
    pool: SqlitePool,
    _temp_dir: tempfile::TempDir,
}

impl TestDatabase {
    async fn new() -> anyhow::Result<Self> {
        let temp_dir = tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        
        let config = DatabaseConfig {
            path: db_path.clone(),
            max_connections: 5,
            connection_timeout: 30,
            enable_wal: true,
            enable_foreign_keys: true,
        };
        
        let db_manager = DatabaseManager::new(&config).await?;
        let pool = db_manager.pool().clone();
        
        // Seed with sample data
        ContentSeeder::seed_sample_content(&pool).await?;
        
        Ok(Self {
            pool,
            _temp_dir: temp_dir,
        })
    }
    
    fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[tokio::test]
async fn test_fts5_search_performance() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    // Test queries with expected performance < 200ms
    let test_queries = vec![
        "philosophy",
        "quantum computing",
        "machine learning",
        "hero journey",
        "scientific revolution",
        "stoicism virtue",
        "algorithms data",
        "quantum superposition",
        "narrative pattern",
        "empirical observation",
    ];
    
    for query in test_queries {
        let start = Instant::now();
        let results = SearchQueries::search(pool, query, Some(20)).await?;
        let duration = start.elapsed();
        
        println!("Query '{}': {}ms - Found {} results", 
                query, duration.as_millis(), results.len());
        
        // Assert performance target
        assert!(duration.as_millis() < 200, 
               "Search for '{}' took {}ms, exceeding 200ms target", 
               query, duration.as_millis());
        
        // Verify we get reasonable results
        assert!(!results.is_empty() || query.len() < 3, 
               "No results found for query: '{}'", query);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_ranked_search_performance() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    let test_queries = vec![
        "quantum",
        "learning",
        "philosophy ancient",
        "scientific method",
        "hero mythical",
    ];
    
    for query in test_queries {
        let start = Instant::now();
        let results = SearchQueries::search_with_ranking(pool, query, Some(10), Some(0)).await?;
        let duration = start.elapsed();
        
        println!("Ranked query '{}': {}ms - Found {} results", 
                query, duration.as_millis(), results.len());
        
        // Assert performance target
        assert!(duration.as_millis() < 200, 
               "Ranked search for '{}' took {}ms, exceeding 200ms target", 
               query, duration.as_millis());
        
        // Verify ranking is in descending order
        for i in 1..results.len() {
            assert!(results[i-1].1 >= results[i].1, 
                   "Results not properly ranked by score");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_document_retrieval_performance() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    // Test various document query operations
    let operations = vec![
        ("recent_documents", "get recent documents"),
        ("category_search", "search by category"),
        ("favorites", "get favorite documents"),
    ];
    
    for (op_name, description) in operations {
        let start = Instant::now();
        
        match op_name {
            "recent_documents" => {
                let _results = DocumentQueries::get_recent(pool, 20).await?;
            },
            "category_search" => {
                let _results = DocumentQueries::get_by_category(pool, "Philosophy", 10, 0).await?;
            },
            "favorites" => {
                let _results = DocumentQueries::get_favorites(pool, 10).await?;
            },
            _ => {}
        }
        
        let duration = start.elapsed();
        
        println!("{}: {}ms", description, duration.as_millis());
        
        // Assert performance target
        assert!(duration.as_millis() < 200, 
               "{} took {}ms, exceeding 200ms target", 
               description, duration.as_millis());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_vector_search_performance() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    // Create sample embeddings for testing
    let sample_docs = vec![
        ("doc1", vec![0.1, 0.2, 0.3, 0.4, 0.5]),
        ("doc2", vec![0.2, 0.3, 0.4, 0.5, 0.6]),
        ("doc3", vec![0.3, 0.4, 0.5, 0.6, 0.7]),
        ("doc4", vec![0.4, 0.5, 0.6, 0.7, 0.8]),
        ("doc5", vec![0.5, 0.6, 0.7, 0.8, 0.9]),
    ];
    
    // Insert sample embeddings
    for (doc_id, vector) in &sample_docs {
        let embedding = Embedding::new(
            doc_id.to_string(),
            vector.clone(),
            "test-model".to_string(),
            0,
            "test chunk".to_string(),
            0,
            10,
        );
        
        EmbeddingQueries::create_with_binary(pool, &embedding).await?;
    }
    
    // Test vector similarity search
    let query_vector = vec![0.3, 0.4, 0.5, 0.6, 0.7];
    
    let start = Instant::now();
    let results = SearchQueries::search_semantic(pool, &query_vector, Some(3), Some(0.5)).await?;
    let duration = start.elapsed();
    
    println!("Vector search: {}ms - Found {} results", 
            duration.as_millis(), results.len());
    
    // Assert performance target
    assert!(duration.as_millis() < 200, 
           "Vector search took {}ms, exceeding 200ms target", 
           duration.as_millis());
    
    // Verify we get results and they're properly ranked
    assert!(!results.is_empty(), "Vector search returned no results");
    
    for i in 1..results.len() {
        assert!(results[i-1].1 >= results[i].1, 
               "Vector search results not properly ranked by similarity");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_hybrid_search_performance() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    // Add some embeddings for testing
    let sample_embedding = Embedding::new(
        "quantum-computing-001".to_string(),
        vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        "test-model".to_string(),
        0,
        "quantum computing principles".to_string(),
        0,
        25,
    );
    
    EmbeddingQueries::create_with_binary(pool, &sample_embedding).await?;
    
    let query = "quantum computing";
    let query_vector = vec![0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
    
    let start = Instant::now();
    let results = SearchQueries::search_hybrid(
        pool, 
        query, 
        Some(&query_vector), 
        Some(10), 
        Some(0.7), 
        Some(0.3)
    ).await?;
    let duration = start.elapsed();
    
    println!("Hybrid search: {}ms - Found {} results", 
            duration.as_millis(), results.len());
    
    // Assert performance target
    assert!(duration.as_millis() < 200, 
           "Hybrid search took {}ms, exceeding 200ms target", 
           duration.as_millis());
    
    // Verify we get results
    assert!(!results.is_empty(), "Hybrid search returned no results");
    
    Ok(())
}

#[tokio::test]
async fn test_bulk_search_operations() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    // Test multiple rapid searches to ensure consistent performance
    let queries = vec!["quantum", "philosophy", "machine", "hero", "science"];
    
    let start = Instant::now();
    
    for query in queries {
        let _results = SearchQueries::search(pool, query, Some(10)).await?;
    }
    
    let total_duration = start.elapsed();
    let avg_duration = total_duration.as_millis() / 5;
    
    println!("Bulk search (5 queries): {}ms total, {}ms average", 
            total_duration.as_millis(), avg_duration);
    
    // Assert average performance target
    assert!(avg_duration < 200, 
           "Average search time {}ms exceeds 200ms target", avg_duration);
    
    Ok(())
}

#[tokio::test]
async fn test_prepared_statements_performance() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    // Test prepared statements performance
    let start = Instant::now();
    let _prepared = PreparedStatements::new(pool).await?;
    let preparation_time = start.elapsed();
    
    println!("Prepared statements initialization: {}ms", preparation_time.as_millis());
    
    // Preparation should be fast
    assert!(preparation_time.as_millis() < 100, 
           "Prepared statements took {}ms to initialize", preparation_time.as_millis());
    
    Ok(())
}

#[tokio::test]
async fn test_database_stats_performance() -> anyhow::Result<()> {
    let test_db = TestDatabase::new().await?;
    let pool = test_db.pool();
    
    let config = DatabaseConfig {
        path: std::path::PathBuf::from(":memory:"),
        max_connections: 5,
        connection_timeout: 30,
        enable_wal: true,
        enable_foreign_keys: true,
    };
    
    let db_manager = DatabaseManager::new(&config).await?;
    
    let start = Instant::now();
    let _stats = db_manager.get_stats().await?;
    let duration = start.elapsed();
    
    println!("Database stats retrieval: {}ms", duration.as_millis());
    
    // Stats retrieval should be fast
    assert!(duration.as_millis() < 100, 
           "Database stats took {}ms to retrieve", duration.as_millis());
    
    Ok(())
}
"