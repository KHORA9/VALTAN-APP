use codex_core::db::{DatabaseManager, Search, ContentSeeder};
use codex_core::config::DatabaseConfig;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup test database
    let config = DatabaseConfig {
        path: "test_search.db".into(),
        max_connections: 5,
        connection_timeout: 30,
        enable_wal: true,
        enable_foreign_keys: true,
    };
    
    let db = DatabaseManager::new(&config).await?;
    let pool = db.pool();
    
    // Seed sample content
    ContentSeeder::seed_sample_content(pool).await?;
    
    // Test search performance
    let queries = ["quantum", "philosophy", "machine learning"];
    
    for query in queries {
        let start = Instant::now();
        let results = Search::fts5(pool, query, 10).await?;
        let duration = start.elapsed();
        
        println!("Query '{}': {}ms - {} results", 
                 query, duration.as_millis(), results.len());
        
        if duration.as_millis() >= 200 {
            println!("WARNING: Query exceeded 200ms target!");
        }
    }
    
    Ok(())
}