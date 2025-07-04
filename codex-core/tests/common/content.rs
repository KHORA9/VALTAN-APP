//! Content testing utilities and fixtures

use codex_core::{
    content::{ContentManager, SearchOptions, SearchType, SortBy, SortOrder},
    db::models::{Document, DocumentCreateRequest},
    CodexResult,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use fake::{Fake, Faker};
use fake::faker::lorem::en::*;

/// Content test environment with sample files
pub struct ContentTestEnv {
    pub temp_dir: TempDir,
    pub content_dir: PathBuf,
    pub sample_files: HashMap<String, PathBuf>,
}

impl ContentTestEnv {
    /// Create new content test environment
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let content_dir = temp_dir.path().join("content");
        std::fs::create_dir_all(&content_dir)?;
        
        let mut env = Self {
            temp_dir,
            content_dir,
            sample_files: HashMap::new(),
        };
        
        env.create_sample_files()?;
        Ok(env)
    }
    
    /// Create sample content files
    fn create_sample_files(&mut self) -> anyhow::Result<()> {
        // Markdown file
        let md_path = self.content_dir.join("philosophy.md");
        std::fs::write(&md_path, SAMPLE_MARKDOWN)?;
        self.sample_files.insert("markdown".to_string(), md_path);
        
        // HTML file
        let html_path = self.content_dir.join("science.html");
        std::fs::write(&html_path, SAMPLE_HTML)?;
        self.sample_files.insert("html".to_string(), html_path);
        
        // JSON file with YAML frontmatter
        let json_path = self.content_dir.join("technology.json");
        std::fs::write(&json_path, SAMPLE_JSON)?;
        self.sample_files.insert("json".to_string(), json_path);
        
        // Plain text file
        let txt_path = self.content_dir.join("literature.txt");
        std::fs::write(&txt_path, SAMPLE_TEXT)?;
        self.sample_files.insert("text".to_string(), txt_path);
        
        Ok(())
    }
    
    /// Get path to sample file by type
    pub fn get_sample_file(&self, file_type: &str) -> Option<&PathBuf> {
        self.sample_files.get(file_type)
    }
    
    /// Create a temporary file with content
    pub fn create_temp_file(&self, name: &str, content: &str) -> anyhow::Result<PathBuf> {
        let path = self.content_dir.join(name);
        std::fs::write(&path, content)?;
        Ok(path)
    }
}

/// Sample content for testing
const SAMPLE_MARKDOWN: &str = r#"---
title: "Introduction to Stoicism"
category: "Philosophy"
tags: ["stoicism", "ancient philosophy", "virtue ethics"]
difficulty: 2
reading_time: 15
---

# Introduction to Stoicism

Stoicism is a philosophical school that teaches the development of self-control and fortitude as a means of overcoming destructive emotions.

## Core Principles

1. **Virtue is the highest good**: The Stoics believed that virtue is the only true good.
2. **External things are indifferent**: Health, wealth, and reputation are neither good nor bad in themselves.
3. **Focus on what you can control**: We should focus our attention on what is within our control.

## Key Figures

- **Epictetus**: Former slave who became a prominent Stoic teacher
- **Marcus Aurelius**: Roman emperor and Stoic philosopher
- **Seneca**: Roman statesman and Stoic writer

Stoicism continues to influence modern psychology and self-help approaches today.
"#;

const SAMPLE_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>Quantum Computing Basics</title>
    <meta name="category" content="Science">
    <meta name="tags" content="quantum,computing,physics">
    <meta name="difficulty" content="5">
</head>
<body>
    <h1>Quantum Computing Basics</h1>
    
    <p>Quantum computing leverages quantum mechanics principles to process information in fundamentally different ways than classical computers.</p>
    
    <h2>Key Concepts</h2>
    <ul>
        <li><strong>Qubits</strong>: The basic unit of quantum information</li>
        <li><strong>Superposition</strong>: A qubit can exist in multiple states simultaneously</li>
        <li><strong>Entanglement</strong>: Qubits can be correlated in ways classical bits cannot</li>
    </ul>
    
    <h2>Applications</h2>
    <p>Quantum computing has potential applications in:</p>
    <ol>
        <li>Cryptography and security</li>
        <li>Drug discovery and molecular modeling</li>
        <li>Financial modeling and optimization</li>
        <li>Artificial intelligence and machine learning</li>
    </ol>
</body>
</html>"#;

const SAMPLE_JSON: &str = r#"{
    "metadata": {
        "title": "Modern Web Development",
        "category": "Technology",
        "tags": ["web", "javascript", "react", "development"],
        "difficulty": 3,
        "reading_time": 20
    },
    "content": {
        "introduction": "Modern web development has evolved significantly with the introduction of new frameworks and tools.",
        "sections": [
            {
                "title": "Frontend Frameworks",
                "content": "React, Vue, and Angular have revolutionized how we build user interfaces."
            },
            {
                "title": "Backend Technologies",
                "content": "Node.js, Python, and Rust provide powerful server-side capabilities."
            },
            {
                "title": "DevOps and Deployment",
                "content": "Docker, Kubernetes, and cloud platforms have simplified deployment and scaling."
            }
        ],
        "conclusion": "The web development landscape continues to evolve rapidly with new tools and best practices."
    }
}"#;

const SAMPLE_TEXT: &str = r#"The Art of Creative Writing

Creative writing is the art of crafting original works that express ideas, emotions, and stories through literary techniques. It encompasses various forms including novels, short stories, poetry, and screenplays.

Key Elements of Creative Writing:

Character Development: Creating believable, multi-dimensional characters that readers can connect with emotionally.

Plot Structure: Organizing events in a compelling sequence that maintains reader interest and builds toward a satisfying conclusion.

Setting and Atmosphere: Establishing the time, place, and mood that enhance the story's impact.

Voice and Style: Developing a unique narrative voice that reflects the writer's personality and serves the story's needs.

Dialogue: Writing realistic conversations that reveal character traits and advance the plot naturally.

The writing process typically involves:
1. Brainstorming and idea generation
2. Outlining and planning
3. First draft writing
4. Revision and editing
5. Final proofreading

Creative writing requires practice, patience, and persistence. Writers develop their craft through regular writing exercises, reading extensively, and seeking feedback from others.
"#;

/// Content search test utilities
pub struct SearchTestUtils;

impl SearchTestUtils {
    /// Create test search options
    pub fn search_options() -> SearchOptions {
        SearchOptions {
            query: "test query".to_string(),
            category: None,
            tags: vec![],
            search_type: SearchType::FullText,
            limit: 10,
            offset: 0,
            sort_by: SortBy::Relevance,
            sort_order: SortOrder::Desc,
            include_content: true,
        }
    }
    
    /// Create search options with specific parameters
    pub fn search_options_with(
        query: &str,
        category: Option<&str>,
        search_type: SearchType,
    ) -> SearchOptions {
        SearchOptions {
            query: query.to_string(),
            category: category.map(|s| s.to_string()),
            tags: vec![],
            search_type,
            limit: 10,
            offset: 0,
            sort_by: SortBy::Relevance,
            sort_order: SortOrder::Desc,
            include_content: true,
        }
    }
    
    /// Create semantic search options
    pub fn semantic_search_options(query: &str) -> SearchOptions {
        Self::search_options_with(query, None, SearchType::Semantic)
    }
    
    /// Create category filter options
    pub fn category_search_options(query: &str, category: &str) -> SearchOptions {
        Self::search_options_with(query, Some(category), SearchType::FullText)
    }
}

/// Content validation test utilities
pub struct ValidationTestUtils;

impl ValidationTestUtils {
    /// Validate document structure
    pub fn validate_document(doc: &Document) -> Vec<String> {
        let mut errors = Vec::new();
        
        if doc.title.trim().is_empty() {
            errors.push("Title cannot be empty".to_string());
        }
        
        if doc.content.trim().is_empty() {
            errors.push("Content cannot be empty".to_string());
        }
        
        if let Some(ref category) = doc.category {
            if !Self::is_valid_category(category) {
                errors.push(format!("Invalid category: {}", category));
            }
        }
        
        if let Some(reading_time) = doc.reading_time {
            if reading_time < 0 {
                errors.push("Reading time cannot be negative".to_string());
            }
        }
        
        if let Some(difficulty) = doc.difficulty_level {
            if !(1..=5).contains(&difficulty) {
                errors.push("Difficulty level must be between 1 and 5".to_string());
            }
        }
        
        errors
    }
    
    /// Check if category is valid
    fn is_valid_category(category: &str) -> bool {
        matches!(
            category,
            "Philosophy" | "Science" | "Technology" | "Health" | "Literature" | "History" | "Arts"
        )
    }
    
    /// Validate content quality
    pub fn validate_content_quality(content: &str) -> Vec<String> {
        let mut warnings = Vec::new();
        
        if content.len() < 100 {
            warnings.push("Content is quite short".to_string());
        }
        
        if content.len() > 50000 {
            warnings.push("Content is very long".to_string());
        }
        
        let word_count = content.split_whitespace().count();
        if word_count < 20 {
            warnings.push("Content has very few words".to_string());
        }
        
        // Check for basic formatting
        if !content.contains('\n') && content.len() > 500 {
            warnings.push("Long content should have paragraph breaks".to_string());
        }
        
        warnings
    }
}

/// Content generation utilities for testing
pub struct ContentGenerator;

impl ContentGenerator {
    /// Generate a random document
    pub fn random_document() -> DocumentCreateRequest {
        DocumentCreateRequest {
            title: Words(2..6).fake::<String>(),
            content: Paragraphs(5..15).fake::<Vec<String>>().join("\n\n"),
            summary: Some(Sentence(15..30).fake()),
            author: Some(Name().fake()),
            category: Some(Self::random_category()),
            tags: Some(Self::random_tags()),
            language: Some("en".to_string()),
            reading_time: Some((60..1800).fake()), // 1-30 minutes
            difficulty_level: Some((1..=5).fake()),
            source_url: None,
            file_path: None,
            file_hash: None,
        }
    }
    
    /// Generate random category
    fn random_category() -> String {
        ["Philosophy", "Science", "Technology", "Health", "Literature", "History", "Arts"]
            .choose(&mut rand::thread_rng())
            .unwrap()
            .to_string()
    }
    
    /// Generate random tags
    fn random_tags() -> Vec<String> {
        let count = (1..=4).fake::<usize>();
        (0..count).map(|_| Word().fake()).collect()
    }
    
    /// Generate documents for a specific category
    pub fn documents_for_category(category: &str, count: usize) -> Vec<DocumentCreateRequest> {
        (0..count)
            .map(|_| {
                let mut doc = Self::random_document();
                doc.category = Some(category.to_string());
                doc
            })
            .collect()
    }
}

/// Macro for content tests
#[macro_export]
macro_rules! content_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> CodexResult<()> {
            let env = ContentTestEnv::new()?;
            $body(env).await
        }
    };
}