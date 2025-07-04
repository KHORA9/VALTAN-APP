//! Content ingestion CLI for Codex Vault
//!
//! This tool provides a command-line interface for importing content files
//! into the Codex Vault knowledge repository with AI-enhanced metadata generation.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::fs;
use tracing::{info, warn, error, debug};
use tracing_subscriber;

use codex_core::{
    CodexError, CodexResult,
    config::{CodexConfig, ContentConfig, AiConfig, DatabaseConfig},
    db::DatabaseManager,
    ai::AiEngine,
    content::{ContentManager, BulkImportResult, ContentStats},
};

#[derive(Parser)]
#[command(name = "vault-cli")]
#[command(about = "Content ingestion CLI for Codex Vault")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Database path
    #[arg(short, long, default_value = "./codex.db")]
    database: PathBuf,
    
    /// Models directory for AI
    #[arg(short, long, default_value = "./models")]
    models_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Import content files or directories
    Import {
        /// Path or glob pattern to import
        #[arg(short, long)]
        path: String,
        
        /// Category to assign to imported documents
        #[arg(short, long)]
        category: Option<String>,
        
        /// Collection to add documents to
        #[arg(long)]
        collection: Option<String>,
        
        /// Import recursively
        #[arg(short, long)]
        recursive: bool,
        
        /// Force reimport even if file exists
        #[arg(short, long)]
        force: bool,
        
        /// Skip AI enhancement for faster import
        #[arg(long)]
        skip_ai: bool,
    },
    /// List imported content
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
        
        /// Output format (table, json, csv)
        #[arg(short, long, default_value = "table")]
        format: String,
        
        /// Limit number of results
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },
    /// Show import statistics
    Stats,
    /// Validate content files without importing
    Validate {
        /// Path to validate
        path: String,
    },
    /// Reindex all documents
    Reindex {
        /// Reindex all documents
        #[arg(short, long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() -> CodexResult<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .init();
    
    info!("Starting Codex Vault CLI");
    
    // Initialize configuration
    let config = create_config(&cli).await?;
    
    // Initialize database
    let db = Arc::new(DatabaseManager::new(&config.database).await?);
    info!("Connected to database: {}", cli.database.display());
    
    // Initialize AI engine
    let ai = Arc::new(AiEngine::new(&config.ai).await?);
    info!("AI engine initialized");
    
    // Initialize content manager
    let content_manager = ContentManager::new(Arc::clone(&db), Arc::clone(&ai), &config.content).await?;
    info!("Content manager initialized");
    
    // Execute command
    match cli.command {
        Commands::Import { path, category, collection, recursive, force, skip_ai } => {
            import_content(&content_manager, &path, category, collection, recursive, force, skip_ai).await?
        }
        Commands::List { category, format, limit } => {
            list_content(&content_manager, category, &format, limit).await?
        }
        Commands::Stats => {
            show_stats(&content_manager).await?
        }
        Commands::Validate { path } => {
            validate_content(&path).await?
        }
        Commands::Reindex { all } => {
            reindex_content(&content_manager, all).await?
        }
    }
    
    info!("Operation completed successfully");
    Ok(())
}

async fn create_config(cli: &Cli) -> CodexResult<CodexConfig> {
    let database_config = DatabaseConfig {
        url: format!("sqlite:{}", cli.database.display()),
        max_connections: 10,
        enable_wal: true,
        enable_foreign_keys: true,
        busy_timeout_ms: 30000,
    };
    
    let ai_config = AiConfig {
        models_dir: cli.models_dir.clone(),
        model_name: "test-llama-7b.gguf".to_string(),
        max_tokens: 512,
        temperature: 0.7,
        enable_caching: true,
        context_window: 4096,
        embedding_model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
        embedding_dimensions: 384,
    };
    
    let content_config = ContentConfig {
        content_dir: PathBuf::from("./content"),
        supported_extensions: vec![
            "md".to_string(), "markdown".to_string(), "txt".to_string(),
            "html".to_string(), "htm".to_string(), "json".to_string(),
        ],
        max_file_size_mb: 50,
        enable_ai_enhancement: true,
        chunk_size: 1000,
        chunk_overlap: 200,
    };
    
    Ok(CodexConfig {
        database: database_config,
        ai: ai_config,
        content: content_config,
    })
}

async fn import_content(
    content_manager: &ContentManager,
    path_pattern: &str,
    category: Option<String>,
    _collection: Option<String>,
    recursive: bool,
    _force: bool,
    _skip_ai: bool,
) -> CodexResult<()> {
    info!("Starting content import from: {}", path_pattern);
    
    // Expand glob pattern to get list of files
    let files = if recursive {
        discover_files_recursive(path_pattern).await?
    } else {
        discover_files(path_pattern).await?
    };
    
    if files.is_empty() {
        warn!("No files found matching pattern: {}", path_pattern);
        return Ok(());
    }
    
    info!("Found {} files to import", files.len());
    
    // Create progress bar
    let progress_bar = create_import_progress_bar(files.len() as u64);
    
    let mut successful_imports = 0;
    let mut failed_imports = 0;
    let mut imported_documents = Vec::new();
    let mut errors = Vec::new();
    
    // Import each file
    for (index, file_path) in files.iter().enumerate() {
        progress_bar.set_position(index as u64);
        progress_bar.set_message(format!("Importing: {}", file_path.file_name().unwrap_or_default().to_string_lossy()));
        
        match import_single_file(content_manager, file_path, &category).await {
            Ok(doc_id) => {
                successful_imports += 1;
                imported_documents.push(doc_id);
                debug!("Successfully imported: {:?}", file_path);
            }
            Err(e) => {
                failed_imports += 1;
                let error_msg = format!("{:?}: {}", file_path, e);
                errors.push(error_msg.clone());
                warn!("Failed to import {:?}: {}", file_path, e);
            }
        }
    }
    
    progress_bar.finish_with_message("Import completed!");
    
    // Show results
    println!("\nImport Results:");
    println!("===============");
    println!("Total files processed: {}", files.len());
    println!("Successful imports: {}", successful_imports);
    println!("Failed imports: {}", failed_imports);
    
    if !errors.is_empty() {
        println!("\nErrors:");
        for error in &errors[..std::cmp::min(5, errors.len())] {
            println!("  - {}", error);
        }
        if errors.len() > 5 {
            println!("  ... and {} more errors", errors.len() - 5);
        }
    }
    
    if successful_imports > 0 {
        println!("\nDocuments imported successfully:");
        for (i, doc_id) in imported_documents.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, doc_id);
        }
        if imported_documents.len() > 5 {
            println!("  ... and {} more documents", imported_documents.len() - 5);
        }
    }
    
    Ok(())
}

async fn import_single_file(
    content_manager: &ContentManager,
    file_path: &Path,
    category: &Option<String>,
) -> CodexResult<uuid::Uuid> {
    let doc_id = content_manager.import_document(file_path).await?;
    
    // Set category if provided
    if let Some(cat) = category {
        content_manager.categorize_document(doc_id, cat.clone()).await?;
    }
    
    Ok(doc_id)
}

async fn discover_files(pattern: &str) -> CodexResult<Vec<PathBuf>> {
    let path = Path::new(pattern);
    let mut files = Vec::new();
    
    if path.is_file() {
        // Single file
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        // Directory - get all supported files
        let mut entries = fs::read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            if entry_path.is_file() && is_supported_file(&entry_path) {
                files.push(entry_path);
            }
        }
    } else {
        // Try glob pattern (simple implementation)
        let parent = path.parent().unwrap_or(Path::new("."));
        if parent.exists() && parent.is_dir() {
            let mut entries = fs::read_dir(parent).await?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                if entry_path.is_file() && is_supported_file(&entry_path) {
                    // Simple pattern matching - check if filename contains pattern
                    if let Some(file_name) = entry_path.file_name() {
                        if let Some(file_str) = file_name.to_str() {
                            if pattern.contains("*") || file_str.contains(&pattern.replace("*", "")) {
                                files.push(entry_path);
                            }
                        }
                    }
                }
            }
        }
    }
    
    files.sort();
    Ok(files)
}

async fn discover_files_recursive(pattern: &str) -> CodexResult<Vec<PathBuf>> {
    let path = Path::new(pattern);
    let base_dir = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(Path::new("."))
    };
    
    let mut files = Vec::new();
    collect_files_recursive(base_dir, &mut files).await?;
    
    files.sort();
    Ok(files)
}

async fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> CodexResult<()> {
    let mut entries = fs::read_dir(dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        
        if path.is_file() && is_supported_file(&path) {
            files.push(path);
        } else if path.is_dir() {
            // Skip hidden directories
            if let Some(dir_name) = path.file_name() {
                if !dir_name.to_string_lossy().starts_with('.') {
                    collect_files_recursive(&path, files).await?;
                }
            }
        }
    }
    
    Ok(())
}

fn is_supported_file(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            let ext_lower = ext_str.to_lowercase();
            return matches!(ext_lower.as_str(), "md" | "markdown" | "txt" | "html" | "htm" | "json");
        }
    }
    false
}

async fn list_content(
    content_manager: &ContentManager,
    category: Option<String>,
    format: &str,
    limit: i64,
) -> CodexResult<()> {
    info!("Listing content with format: {}", format);
    
    let documents = if let Some(cat) = category {
        content_manager.get_documents_by_category(&cat, limit, 0).await?
    } else {
        content_manager.get_recent_documents(limit).await?
    };
    
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&documents)?;
            println!("{}", json);
        }
        "csv" => {
            println!("id,title,category,author,created_at,reading_time,view_count");
            for doc in documents {
                println!("{},{},{},{},{},{},{}",
                    doc.id,
                    doc.title.replace(',', ";"),
                    doc.category.unwrap_or_default(),
                    doc.author.unwrap_or_default(),
                    doc.created_at,
                    doc.reading_time.unwrap_or(0),
                    doc.view_count
                );
            }
        }
        _ => {
            // Table format (default)
            println!("Documents in Codex Vault");
            println!("========================");
            println!();
            
            if documents.is_empty() {
                println!("No documents found.");
                return Ok(());
            }
            
            for (i, doc) in documents.iter().enumerate() {
                println!("{}. {}", i + 1, doc.title);
                println!("   ID: {}", doc.id);
                if let Some(category) = &doc.category {
                    println!("   Category: {}", category);
                }
                if let Some(author) = &doc.author {
                    println!("   Author: {}", author);
                }
                if let Some(reading_time) = doc.reading_time {
                    println!("   Reading time: {} minutes", reading_time);
                }
                println!("   Views: {}", doc.view_count);
                println!("   Created: {}", doc.created_at);
                
                if let Some(summary) = &doc.summary {
                    let truncated = if summary.len() > 100 {
                        format!("{}...", &summary[..100])
                    } else {
                        summary.clone()
                    };
                    println!("   Summary: {}", truncated);
                }
                
                println!();
            }
        }
    }
    
    Ok(())
}

async fn show_stats(content_manager: &ContentManager) -> CodexResult<()> {
    info!("Gathering content statistics");
    
    let stats = content_manager.get_content_stats().await?;
    
    println!("Codex Vault Statistics");
    println!("======================");
    println!();
    println!("Documents: {}", stats.total_documents);
    println!("Embeddings: {}", stats.total_embeddings);
    println!("Indexed documents: {}", stats.indexed_documents);
    println!("Database size: {:.2} MB", stats.database_size_bytes as f64 / (1024.0 * 1024.0));
    
    // Get additional stats
    let recent_docs = content_manager.get_recent_documents(5).await?;
    let favorite_docs = content_manager.get_favorite_documents(i64::MAX).await?;
    
    println!();
    println!("Recent Activity:");
    if recent_docs.is_empty() {
        println!("  No recent documents");
    } else {
        for (i, doc) in recent_docs.iter().enumerate() {
            println!("  {}. {} ({})", i + 1, doc.title, doc.created_at);
        }
    }
    
    println!();
    println!("Favorites: {}", favorite_docs.len());
    
    Ok(())
}

async fn validate_content(path: &str) -> CodexResult<()> {
    info!("Validating content at: {}", path);
    
    let files = discover_files(path).await?;
    
    if files.is_empty() {
        println!("No files found to validate at: {}", path);
        return Ok(());
    }
    
    println!("Validation Results");
    println!("==================");
    println!();
    
    let mut valid_files = 0;
    let mut invalid_files = 0;
    
    for file_path in &files {
        print!("Checking {:?}... ", file_path.file_name().unwrap_or_default());
        
        match validate_single_file(file_path).await {
            Ok(_) => {
                println!("✓ Valid");
                valid_files += 1;
            }
            Err(e) => {
                println!("✗ Invalid: {}", e);
                invalid_files += 1;
            }
        }
    }
    
    println!();
    println!("Summary: {} valid, {} invalid files", valid_files, invalid_files);
    
    Ok(())
}

async fn validate_single_file(file_path: &Path) -> CodexResult<()> {
    // Check if file exists
    if !file_path.exists() {
        return Err(CodexError::not_found("File does not exist"));
    }
    
    // Check file extension
    if !is_supported_file(file_path) {
        return Err(CodexError::validation("Unsupported file type"));
    }
    
    // Check file size
    let metadata = fs::metadata(file_path).await?;
    let file_size_mb = metadata.len() / (1024 * 1024);
    if file_size_mb > 50 {
        return Err(CodexError::validation(format!("File too large: {} MB", file_size_mb)));
    }
    
    // Try to read file content
    let _content = fs::read_to_string(file_path).await
        .map_err(|_| CodexError::validation("Cannot read file as UTF-8"))?;
    
    Ok(())
}

async fn reindex_content(content_manager: &ContentManager, all: bool) -> CodexResult<()> {
    if !all {
        return Err(CodexError::validation("Currently only --all reindexing is supported"));
    }
    
    info!("Starting full reindex of all documents");
    
    println!("Reindexing all documents...");
    let progress_bar = create_simple_progress_bar();
    progress_bar.set_message("Reindexing documents...");
    
    content_manager.reindex_all_documents().await?;
    
    progress_bar.finish_with_message("Reindexing completed!");
    println!("All documents have been reindexed successfully.");
    
    Ok(())
}

fn create_import_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb
}

fn create_simple_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb
}