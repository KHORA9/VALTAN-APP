//! Configuration management for Codex Core

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use directories::ProjectDirs;

/// Main configuration structure for Codex Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    /// AI configuration
    pub ai: AiConfig,
    /// Content management configuration
    pub content: ContentConfig,
    /// Update system configuration
    pub update: UpdateConfig,
    /// Application settings
    pub app: AppConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Path to the SQLite database file
    pub path: PathBuf,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Enable WAL mode for better performance
    pub enable_wal: bool,
    /// Enable foreign key constraints
    pub enable_foreign_keys: bool,
}

/// AI engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Path to the AI models directory
    pub models_dir: PathBuf,
    /// Primary model name/path
    pub primary_model: String,
    /// Maximum context length
    pub max_context_length: usize,
    /// Temperature for text generation
    pub temperature: f32,
    /// Top-p value for nucleus sampling
    pub top_p: f32,
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Device to use (cpu, cuda, metal)
    pub device: String,
    /// Enable model caching
    pub enable_caching: bool,
    /// Cache size in MB
    pub cache_size_mb: usize,
}

/// Content management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentConfig {
    /// Path to the content directory
    pub content_dir: PathBuf,
    /// Supported file extensions
    pub supported_extensions: Vec<String>,
    /// Maximum file size in MB
    pub max_file_size_mb: usize,
    /// Enable content compression
    pub enable_compression: bool,
    /// Compression level (1-9)
    pub compression_level: u32,
    /// Enable automatic indexing
    pub auto_index: bool,
    /// Batch size for indexing operations
    pub index_batch_size: usize,
}

/// Update system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Update server URL
    pub server_url: String,
    /// Check for updates automatically
    pub auto_check: bool,
    /// Update check interval in hours
    pub check_interval_hours: u64,
    /// Enable delta updates
    pub enable_delta_updates: bool,
    /// Update channel (stable, beta, nightly)
    pub channel: String,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Log level
    pub log_level: String,
    /// Enable telemetry (privacy-first, local only)
    pub enable_telemetry: bool,
    /// UI theme (light, dark, auto)
    pub theme: String,
    /// Language/locale
    pub locale: String,
}

impl Default for CodexConfig {
    fn default() -> Self {
        let project_dirs = ProjectDirs::from("com", "hanatra", "codex-vault")
            .expect("Failed to get project directories");

        Self {
            database: DatabaseConfig {
                path: project_dirs.data_dir().join("codex.db"),
                max_connections: 10,
                connection_timeout: 30,
                enable_wal: true,
                enable_foreign_keys: true,
            },
            ai: AiConfig {
                models_dir: project_dirs.data_dir().join("models"),
                primary_model: "llama-2-7b-chat.gguf".to_string(),
                max_context_length: 4096,
                temperature: 0.7,
                top_p: 0.9,
                max_tokens: 512,
                device: "cpu".to_string(),
                enable_caching: true,
                cache_size_mb: 512,
            },
            content: ContentConfig {
                content_dir: project_dirs.data_dir().join("content"),
                supported_extensions: vec![
                    "txt".to_string(),
                    "md".to_string(),
                    "pdf".to_string(),
                    "epub".to_string(),
                    "html".to_string(),
                    "json".to_string(),
                ],
                max_file_size_mb: 100,
                enable_compression: true,
                compression_level: 6,
                auto_index: true,
                index_batch_size: 100,
            },
            update: UpdateConfig {
                server_url: "https://updates.codex-vault.com".to_string(),
                auto_check: true,
                check_interval_hours: 24,
                enable_delta_updates: true,
                channel: "stable".to_string(),
            },
            app: AppConfig {
                name: "Codex Vault".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                log_level: "info".to_string(),
                enable_telemetry: false,
                theme: "auto".to_string(),
                locale: "en-US".to_string(),
            },
        }
    }
}

impl CodexConfig {
    /// Load configuration from the default location
    pub async fn load_default() -> Result<Self> {
        let project_dirs = ProjectDirs::from("com", "hanatra", "codex-vault")
            .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

        let config_path = project_dirs.config_dir().join("config.toml");
        
        if config_path.exists() {
            Self::load_from_file(config_path).await
        } else {
            let config = Self::default();
            config.save_to_default().await?;
            Ok(config)
        }
    }

    /// Load configuration from a specific file
    pub async fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to the default location
    pub async fn save_to_default(&self) -> Result<()> {
        let project_dirs = ProjectDirs::from("com", "hanatra", "codex-vault")
            .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

        let config_dir = project_dirs.config_dir();
        tokio::fs::create_dir_all(config_dir).await?;

        let config_path = config_dir.join("config.toml");
        self.save_to_file(config_path).await
    }

    /// Save configuration to a specific file
    pub async fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Save configuration (to default location)
    pub async fn save(&self) -> Result<()> {
        self.save_to_default().await
    }

    /// Ensure all required directories exist
    pub async fn ensure_directories(&self) -> Result<()> {
        // Create parent directory for database
        if let Some(parent) = self.database.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Create models directory
        tokio::fs::create_dir_all(&self.ai.models_dir).await?;

        // Create content directory
        tokio::fs::create_dir_all(&self.content.content_dir).await?;

        Ok(())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate database configuration
        if self.database.max_connections == 0 {
            return Err(anyhow::anyhow!("Database max_connections must be > 0"));
        }

        // Validate AI configuration
        if self.ai.max_context_length == 0 {
            return Err(anyhow::anyhow!("AI max_context_length must be > 0"));
        }

        if !(0.0..=1.0).contains(&self.ai.temperature) {
            return Err(anyhow::anyhow!("AI temperature must be between 0.0 and 1.0"));
        }

        if !(0.0..=1.0).contains(&self.ai.top_p) {
            return Err(anyhow::anyhow!("AI top_p must be between 0.0 and 1.0"));
        }

        // Validate content configuration
        if self.content.max_file_size_mb == 0 {
            return Err(anyhow::anyhow!("Content max_file_size_mb must be > 0"));
        }

        if !(1..=9).contains(&self.content.compression_level) {
            return Err(anyhow::anyhow!("Content compression_level must be between 1 and 9"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_default() {
        let config = CodexConfig::default();
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_config_save_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let original_config = CodexConfig::default();
        original_config.save_to_file(&config_path).await.unwrap();

        let loaded_config = CodexConfig::load_from_file(&config_path).await.unwrap();
        
        // Compare a few fields to ensure serialization/deserialization works
        assert_eq!(original_config.app.name, loaded_config.app.name);
        assert_eq!(original_config.ai.temperature, loaded_config.ai.temperature);
    }
}