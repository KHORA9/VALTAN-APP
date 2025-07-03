//! Error handling for the Codex Core library

use thiserror::Error;

/// Result type alias for Codex Core operations
pub type CodexResult<T> = Result<T, CodexError>;

/// Main error type for Codex Core operations
#[derive(Error, Debug)]
pub enum CodexError {
    /// Database-related errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// AI inference errors
    #[error("AI inference error: {0}")]
    AiInference(String),

    /// Content processing errors
    #[error("Content processing error: {0}")]
    ContentProcessing(String),

    /// Update system errors
    #[error("Update system error: {0}")]
    Update(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Network errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found errors
    #[error("Not found: {0}")]
    NotFound(String),

    /// Permission errors
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// Migration errors
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

impl CodexError {
    /// Create a new AI inference error
    pub fn ai_inference<S: Into<String>>(msg: S) -> Self {
        Self::AiInference(msg.into())
    }

    /// Create a new content processing error
    pub fn content_processing<S: Into<String>>(msg: S) -> Self {
        Self::ContentProcessing(msg.into())
    }

    /// Create a new update system error
    pub fn update<S: Into<String>>(msg: S) -> Self {
        Self::Update(msg.into())
    }

    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a new not found error
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create a new permission denied error
    pub fn permission_denied<S: Into<String>>(msg: S) -> Self {
        Self::PermissionDenied(msg.into())
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if this is a database error
    pub fn is_database_error(&self) -> bool {
        matches!(self, Self::Database(_))
    }

    /// Check if this is an AI inference error
    pub fn is_ai_error(&self) -> bool {
        matches!(self, Self::AiInference(_))
    }

    /// Check if this is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    /// Check if this is a validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(self, Self::Validation(_))
    }
}