//! Database module for Codex Core
//!
//! This module provides SQLite database operations with optimized performance
//! for full-text search and vector embeddings.

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, migrate::MigrateDatabase, Sqlite};
use anyhow::Result;
use tracing::{info, debug, error};

use crate::{CodexError, CodexResult};
use crate::config::DatabaseConfig;

pub mod models;
pub mod queries;
pub mod connection;
pub mod seeder;
pub mod search;
pub mod vector_ops;

pub use models::*;
pub use queries::*;
pub use connection::*;
pub use seeder::*;
pub use search::*;
pub use vector_ops::*;

/// Database manager handling all SQLite operations
#[derive(Debug)]
pub struct DatabaseManager {
    pool: SqlitePool,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Create a new database manager with the given configuration
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Initializing database manager at {:?}", config.path);

        // Ensure parent directory exists
        if let Some(parent) = config.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Create database if it doesn't exist
        let database_url = format!("sqlite://{}", config.path.display());
        
        if !Sqlite::database_exists(&database_url).await.unwrap_or(false) {
            info!("Creating new database at {:?}", config.path);
            Sqlite::create_database(&database_url).await?;
        }

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout))
            .connect(&database_url)
            .await?;

        // Configure SQLite settings
        Self::configure_sqlite(&pool, config).await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        info!("Database manager initialized successfully");

        Ok(Self {
            pool,
            config: config.clone(),
        })
    }

    /// Configure SQLite-specific settings for optimal performance
    async fn configure_sqlite(pool: &SqlitePool, config: &DatabaseConfig) -> Result<()> {
        debug!("Configuring SQLite settings");

        let mut conn = pool.acquire().await?;

        // Enable WAL mode for better concurrency
        if config.enable_wal {
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&mut *conn)
                .await?;
        }

        // Enable foreign key constraints
        if config.enable_foreign_keys {
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&mut *conn)
                .await?;
        }

        // Optimize SQLite settings for performance
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&mut *conn)
            .await?;

        sqlx::query("PRAGMA cache_size = -64000") // 64MB cache
            .execute(&mut *conn)
            .await?;

        sqlx::query("PRAGMA temp_store = MEMORY")
            .execute(&mut *conn)
            .await?;

        sqlx::query("PRAGMA mmap_size = 268435456") // 256MB mmap
            .execute(&mut *conn)
            .await?;

        debug!("SQLite configuration complete");
        Ok(())
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> CodexResult<sqlx::pool::PoolConnection<sqlx::Sqlite>> {
        self.pool.acquire().await.map_err(CodexError::from)
    }

    /// Execute a transaction
    pub async fn transaction<F, R>(&self, f: F) -> CodexResult<R>
    where
        F: for<'c> FnOnce(&mut sqlx::Transaction<'c, sqlx::Sqlite>) -> std::pin::Pin<Box<dyn std::future::Future<Output = CodexResult<R>> + Send + 'c>>,
    {
        let mut tx = self.pool.begin().await?;
        let result = f(&mut tx).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Perform database health check
    pub async fn health_check(&self) -> CodexResult<bool> {
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("Database health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> CodexResult<DatabaseStats> {
        let mut conn = self.get_connection().await?;

        let document_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
            .fetch_one(&mut *conn)
            .await?;

        let embedding_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM embeddings")
            .fetch_one(&mut *conn)
            .await?;

        let db_size: (i64,) = sqlx::query_as("SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()")
            .fetch_one(&mut *conn)
            .await?;

        Ok(DatabaseStats {
            document_count: document_count.0 as u64,
            embedding_count: embedding_count.0 as u64,
            database_size_bytes: db_size.0 as u64,
        })
    }

    /// Optimize the database (VACUUM and ANALYZE)
    pub async fn optimize(&self) -> CodexResult<()> {
        info!("Optimizing database");
        
        let mut conn = self.get_connection().await?;
        
        sqlx::query("VACUUM").execute(&mut *conn).await?;
        sqlx::query("ANALYZE").execute(&mut *conn).await?;
        
        info!("Database optimization complete");
        Ok(())
    }

    /// Backup the database to a file
    pub async fn backup<P: AsRef<std::path::Path>>(&self, backup_path: P) -> CodexResult<()> {
        info!("Creating database backup at {:?}", backup_path.as_ref());
        
        // Simple file copy for SQLite
        tokio::fs::copy(&self.config.path, backup_path).await?;
        
        info!("Database backup complete");
        Ok(())
    }

    /// Shutdown the database manager
    pub async fn shutdown(&self) -> CodexResult<()> {
        info!("Shutting down database manager");
        self.pool.close().await;
        Ok(())
    }
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseStats {
    pub document_count: u64,
    pub embedding_count: u64,
    pub database_size_bytes: u64,
}