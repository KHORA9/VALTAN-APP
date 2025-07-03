//! # Codex Core Library
//!
//! Core library for Codex Vault Next-Gen - an offline AI-powered knowledge repository
//! with enterprise-grade performance and premium user experience.
//!
//! ## Features
//!
//! - **Database Operations**: SQLite with FTS5 for fast full-text search
//! - **AI Inference**: Local LLM with sub-second response times
//! - **Content Management**: Document parsing, indexing, and search
//! - **Update System**: Secure auto-updates with delta compression
//!
//! ## Architecture
//!
//! The library follows a modular architecture with clean separation of concerns:
//! - `db`: Database operations and models
//! - `ai`: AI inference and embeddings
//! - `content`: Content processing and search
//! - `update`: Application update management

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

pub mod db;
pub mod ai;
pub mod content;
pub mod update;
pub mod error;
pub mod config;

pub use error::{CodexError, CodexResult};
pub use config::CodexConfig;

/// Main application state containing all core components
#[derive(Clone)]
pub struct CodexCore {
    /// Database connection pool and operations
    pub db: Arc<db::DatabaseManager>,
    /// AI inference engine
    pub ai: Arc<ai::AiEngine>,
    /// Content management system
    pub content: Arc<content::ContentManager>,
    /// Update manager
    pub update: Arc<update::UpdateManager>,
    /// Application configuration
    pub config: Arc<RwLock<CodexConfig>>,
}

impl CodexCore {
    /// Initialize the Codex Core library with default configuration
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use codex_core::CodexCore;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let core = CodexCore::new().await?;
    ///     // Use core for operations...
    ///     Ok(())
    /// }
    /// ```
    pub async fn new() -> Result<Self> {
        let config = CodexConfig::load_default().await?;
        Self::with_config(config).await
    }

    /// Initialize the Codex Core library with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Custom configuration for the application
    pub async fn with_config(config: CodexConfig) -> Result<Self> {
        tracing::info!("Initializing Codex Core library");

        // Initialize database manager
        let db = Arc::new(db::DatabaseManager::new(&config.database).await?);
        
        // Initialize AI engine
        let ai = Arc::new(ai::AiEngine::new(&config.ai).await?);
        
        // Initialize content manager
        let content = Arc::new(content::ContentManager::new(
            Arc::clone(&db),
            Arc::clone(&ai),
            &config.content,
        ).await?);
        
        // Initialize update manager
        let update = Arc::new(update::UpdateManager::new(&config.update).await?);

        let config = Arc::new(RwLock::new(config));

        tracing::info!("Codex Core library initialized successfully");

        Ok(Self {
            db,
            ai,
            content,
            update,
            config,
        })
    }

    /// Shutdown the core library gracefully
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Codex Core library");
        
        // Shutdown components in reverse order
        self.update.shutdown().await?;
        self.content.shutdown().await?;
        self.ai.shutdown().await?;
        self.db.shutdown().await?;
        
        tracing::info!("Codex Core library shutdown complete");
        Ok(())
    }

    /// Get the current configuration (read-only)
    pub async fn get_config(&self) -> CodexConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration
    pub async fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut CodexConfig) -> Result<()>,
    {
        let mut config = self.config.write().await;
        updater(&mut *config)?;
        config.save().await?;
        Ok(())
    }

    /// Perform a health check on all components
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let db_health = self.db.health_check().await?;
        let ai_health = self.ai.health_check().await?;
        let content_health = self.content.health_check().await?;
        let update_health = self.update.health_check().await?;

        Ok(HealthStatus {
            database: db_health,
            ai: ai_health,
            content: content_health,
            update: update_health,
            overall: db_health && ai_health && content_health && update_health,
        })
    }
}

/// Health status for all core components
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub database: bool,
    pub ai: bool,
    pub content: bool,
    pub update: bool,
    pub overall: bool,
}

/// Initialize tracing/logging for the library
pub fn init_tracing() -> Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "codex_core=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_core_initialization() {
        let temp_dir = tempdir().unwrap();
        let mut config = CodexConfig::default();
        config.database.path = temp_dir.path().join("test.db");
        
        let core = CodexCore::with_config(config).await;
        assert!(core.is_ok());

        let core = core.unwrap();
        let health = core.health_check().await.unwrap();
        assert!(health.overall);

        core.shutdown().await.unwrap();
    }
}