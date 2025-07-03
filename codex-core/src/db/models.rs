//! Database models for Codex Core

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Document model representing stored content
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    /// Unique document identifier
    pub id: Uuid,
    /// Document title
    pub title: String,
    /// Document content (full text)
    pub content: String,
    /// Content summary/description
    pub summary: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Document source/origin
    pub source: Option<String>,
    /// Document URL or path
    pub url: Option<String>,
    /// Content type/format (markdown, html, pdf, etc.)
    pub content_type: String,
    /// Document category
    pub category: Option<String>,
    /// Document tags (JSON array)
    pub tags: Option<String>,
    /// Document language
    pub language: String,
    /// Reading time in minutes
    pub reading_time: Option<i32>,
    /// Difficulty level (1-5)
    pub difficulty_level: Option<i32>,
    /// File size in bytes
    pub file_size: Option<i64>,
    /// File hash for deduplication
    pub file_hash: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: Option<DateTime<Utc>>,
    /// View count
    pub view_count: i64,
    /// Favorite status
    pub is_favorite: bool,
    /// Archive status
    pub is_archived: bool,
    /// Soft delete status
    pub is_deleted: bool,
}

/// Vector embedding model for semantic search
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Embedding {
    /// Unique embedding identifier
    pub id: Uuid,
    /// Associated document ID
    pub document_id: Uuid,
    /// Embedding vector (stored as JSON)
    pub vector: String,
    /// Vector dimensions
    pub dimensions: i32,
    /// Embedding model used
    pub model: String,
    /// Chunk index (for long documents)
    pub chunk_index: i32,
    /// Text chunk that was embedded
    pub text_chunk: String,
    /// Start position in original text
    pub start_position: i32,
    /// End position in original text
    pub end_position: i32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Application settings model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    /// Setting key
    pub key: String,
    /// Setting value (JSON)
    pub value: String,
    /// Setting description
    pub description: Option<String>,
    /// Setting category
    pub category: String,
    /// Is setting user-configurable
    pub is_user_configurable: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// User bookmark model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bookmark {
    /// Unique bookmark identifier
    pub id: Uuid,
    /// Associated document ID
    pub document_id: Uuid,
    /// Bookmark title/note
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Position in document (character offset)
    pub position: Option<i32>,
    /// Selected text
    pub selected_text: Option<String>,
    /// Bookmark tags
    pub tags: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// User note model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Note {
    /// Unique note identifier
    pub id: Uuid,
    /// Associated document ID (optional for standalone notes)
    pub document_id: Option<Uuid>,
    /// Note title
    pub title: String,
    /// Note content (markdown)
    pub content: String,
    /// Note tags
    pub tags: Option<String>,
    /// Note color/category
    pub color: Option<String>,
    /// Is note pinned
    pub is_pinned: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Collection model for organizing documents
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Collection {
    /// Unique collection identifier
    pub id: Uuid,
    /// Collection name
    pub name: String,
    /// Collection description
    pub description: Option<String>,
    /// Collection color
    pub color: Option<String>,
    /// Collection icon
    pub icon: Option<String>,
    /// Is collection pinned
    pub is_pinned: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Junction table for document-collection relationships
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentCollection {
    /// Document ID
    pub document_id: Uuid,
    /// Collection ID
    pub collection_id: Uuid,
    /// Order within collection
    pub order_index: i32,
    /// Addition timestamp
    pub added_at: DateTime<Utc>,
}

/// Search history model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchHistory {
    /// Unique search identifier
    pub id: Uuid,
    /// Search query
    pub query: String,
    /// Search type (full_text, semantic, hybrid)
    pub search_type: String,
    /// Number of results returned
    pub result_count: i32,
    /// Search timestamp
    pub searched_at: DateTime<Utc>,
}

/// User reading progress model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReadingProgress {
    /// Document ID
    pub document_id: Uuid,
    /// Current position (percentage 0-100)
    pub progress_percentage: f32,
    /// Current scroll position
    pub scroll_position: Option<i32>,
    /// Reading session start time
    pub session_start: DateTime<Utc>,
    /// Total reading time in seconds
    pub total_reading_time: i32,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Document {
    /// Create a new document with default values
    pub fn new(title: String, content: String, content_type: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            content,
            summary: None,
            author: None,
            source: None,
            url: None,
            content_type,
            category: None,
            tags: None,
            language: "en".to_string(),
            reading_time: None,
            difficulty_level: None,
            file_size: None,
            file_hash: None,
            created_at: now,
            updated_at: now,
            last_accessed: None,
            view_count: 0,
            is_favorite: false,
            is_archived: false,
            is_deleted: false,
        }
    }

    /// Get tags as a vector
    pub fn get_tags(&self) -> Vec<String> {
        self.tags
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default()
    }

    /// Set tags from a vector
    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = Some(serde_json::to_string(&tags).unwrap_or_default());
    }
}

impl Embedding {
    /// Create a new embedding
    pub fn new(
        document_id: Uuid,
        vector: Vec<f32>,
        model: String,
        chunk_index: i32,
        text_chunk: String,
        start_position: i32,
        end_position: i32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            document_id,
            vector: serde_json::to_string(&vector).unwrap_or_default(),
            dimensions: vector.len() as i32,
            model,
            chunk_index,
            text_chunk,
            start_position,
            end_position,
            created_at: Utc::now(),
        }
    }

    /// Get vector as Vec<f32>
    pub fn get_vector(&self) -> Vec<f32> {
        serde_json::from_str(&self.vector).unwrap_or_default()
    }

    /// Set vector from Vec<f32>
    pub fn set_vector(&mut self, vector: Vec<f32>) {
        self.dimensions = vector.len() as i32;
        self.vector = serde_json::to_string(&vector).unwrap_or_default();
    }
}

impl Setting {
    /// Create a new setting
    pub fn new(key: String, value: String, category: String) -> Self {
        let now = Utc::now();
        Self {
            key,
            value,
            description: None,
            category,
            is_user_configurable: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get value as specific type
    pub fn get_value<T>(&self) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_str(&self.value).ok()
    }

    /// Set value from any serializable type
    pub fn set_value<T>(&mut self, value: &T) -> Result<(), serde_json::Error>
    where
        T: serde::Serialize,
    {
        self.value = serde_json::to_string(value)?;
        self.updated_at = Utc::now();
        Ok(())
    }
}