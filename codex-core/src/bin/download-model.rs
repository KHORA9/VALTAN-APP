//! CLI utility for downloading and verifying AI models
//!
//! This tool provides a command-line interface for downloading AI models
//! with progress tracking, checksum verification, and integrity validation.

use std::path::PathBuf;
use std::sync::Arc;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Mutex;
use tracing::{info, warn, error};
use tracing_subscriber;

use codex_core::{
    CodexError, CodexResult,
    update::{
        manifest::{ModelManifest, ModelRegistry},
        model_downloader::{ModelDownloader, DownloadProgress, DownloadStage},
    },
    ai::engine::GGUFEngine,
};

#[derive(Parser)]
#[command(name = "download-model")]
#[command(about = "Download and verify AI models for Codex Vault")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Download directory
    #[arg(short, long, default_value = "./models")]
    download_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Download a specific model
    Download {
        /// Model name to download
        #[arg(short, long)]
        model: String,
        
        /// Force re-download even if model exists
        #[arg(short, long)]
        force: bool,
    },
    /// List available models
    List {
        /// Registry URL to fetch models from
        #[arg(short, long, default_value = "https://models.codex-vault.com/registry.json")]
        registry: String,
    },
    /// Verify an existing model
    Verify {
        /// Path to model file
        model_path: PathBuf,
        
        /// Expected SHA256 checksum
        #[arg(short, long)]
        checksum: Option<String>,
    },
    /// Show model information
    Info {
        /// Path to model file
        model_path: PathBuf,
    },
    /// Clean up downloaded models
    Clean {
        /// Remove all models
        #[arg(short, long)]
        all: bool,
        
        /// Specific model to remove
        #[arg(short, long)]
        model: Option<String>,
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
    
    // Ensure download directory exists
    tokio::fs::create_dir_all(&cli.download_dir).await
        .map_err(|e| CodexError::io(e))?;
    
    match cli.command {
        Commands::Download { model, force } => {
            download_model(&cli.download_dir, &model, force).await?
        }
        Commands::List { registry } => {
            list_models(&registry).await?
        }
        Commands::Verify { model_path, checksum } => {
            verify_model(&model_path, checksum).await?
        }
        Commands::Info { model_path } => {
            show_model_info(&model_path).await?
        }
        Commands::Clean { all, model } => {
            clean_models(&cli.download_dir, all, model).await?
        }
    }
    
    Ok(())
}

async fn download_model(download_dir: &PathBuf, model_name: &str, force: bool) -> CodexResult<()> {
    info!("Starting model download: {}", model_name);
    
    // For demonstration, create a sample Mistral 7B manifest
    let manifest = if model_name == "mistral-7b-instruct-q4k" {
        ModelManifest::mistral_7b_instruct_q4k()
    } else {
        return Err(CodexError::not_found(format!("Model not found: {}", model_name)));
    };
    
    // Validate manifest
    let validation = manifest.validate();
    if !validation.is_valid {
        error!("Model manifest validation failed:");
        for error in validation.errors {
            error!("  - {}", error);
        }
        return Err(CodexError::validation("Invalid model manifest"));
    }
    
    // Check if model already exists
    let target_path = manifest.get_local_path(download_dir);
    if target_path.exists() && !force {
        info!("Model already exists at: {}", target_path.display());
        
        // Verify existing model
        let is_valid = GGUFEngine::verify_checksum(&target_path, &manifest.sha256_checksum).await?;
        if is_valid {
            info!("Existing model is valid. Use --force to re-download.");
            return Ok(());
        } else {
            warn!("Existing model is corrupted, re-downloading...");
        }
    }
    
    // Create progress bar
    let progress_bar = Arc::new(Mutex::new(create_progress_bar(manifest.file_size)));
    let pb_clone = Arc::clone(&progress_bar);
    
    // Create downloader with progress callback
    let downloader = ModelDownloader::new(download_dir.clone())
        .with_progress_callback(Box::new(move |progress| {
            let pb = pb_clone.clone();
            tokio::spawn(async move {
                update_progress_bar(pb, progress).await;
            });
        }));
    
    // Download model
    match downloader.download_model(&manifest).await {
        Ok(downloaded_path) => {
            let pb = progress_bar.lock().await;
            pb.finish_with_message("Download completed successfully!");
            drop(pb);
            
            info!("Model downloaded to: {}", downloaded_path.display());
            
            // Show model information
            show_model_info_from_manifest(&manifest, &downloaded_path).await;
        }
        Err(e) => {
            let pb = progress_bar.lock().await;
            pb.abandon_with_message("Download failed!");
            drop(pb);
            
            return Err(e);
        }
    }
    
    Ok(())
}

async fn list_models(registry_url: &str) -> CodexResult<()> {
    info!("Fetching available models from registry...");
    
    let downloader = ModelDownloader::new(PathBuf::from("./temp"));
    
    match downloader.get_available_models(registry_url).await {
        Ok(registry) => {
            println!("Available Models:");
            println!("=================");
            
            for model in registry.models {
                println!();
                println!("Name: {}", model.name);
                println!("Version: {}", model.version);
                println!("Description: {}", model.description);
                println!("Architecture: {}", model.architecture);
                println!("Parameters: {}", model.parameter_count);
                println!("Quantization: {}", model.quantization);
                println!("File Size: {:.2} GB", model.file_size as f64 / (1024.0 * 1024.0 * 1024.0));
                println!("Context Length: {}", model.context_length);
                
                if let Some(ref performance) = model.performance {
                    println!("Performance: {:.1} tokens/sec ({})", 
                             performance.tokens_per_second, 
                             performance.reference_hardware);
                }
                
                println!("Compatible: {}", if model.is_compatible_with_system() { "Yes" } else { "No" });
                println!("Tags: {}", model.tags.join(", "));
            }
            
            println!();
            println!("Registry: {}", registry.metadata.name);
            println!("Last Updated: {}", registry.last_updated.format("%Y-%m-%d %H:%M:%S UTC"));
        }
        Err(e) => {
            error!("Failed to fetch model registry: {}", e);
            
            // Show default models as fallback
            println!("Default Available Models:");
            println!("========================");
            
            let default_registry = ModelRegistry::default_registry();
            for model in default_registry.models {
                println!("- {} ({})", model.name, model.description);
            }
        }
    }
    
    Ok(())
}

async fn verify_model(model_path: &PathBuf, expected_checksum: Option<String>) -> CodexResult<()> {
    info!("Verifying model: {}", model_path.display());
    
    if !model_path.exists() {
        return Err(CodexError::not_found(format!("Model file not found: {}", model_path.display())));
    }
    
    // Calculate actual checksum
    info!("Calculating SHA256 checksum...");
    let actual_checksum = GGUFEngine::calculate_checksum(model_path).await?;
    
    println!("File: {}", model_path.display());
    println!("SHA256: {}", actual_checksum);
    
    if let Some(expected) = expected_checksum {
        let is_valid = actual_checksum.eq_ignore_ascii_case(&expected);
        println!("Expected: {}", expected);
        println!("Valid: {}", if is_valid { "✓ Yes" } else { "✗ No" });
        
        if !is_valid {
            return Err(CodexError::validation("Checksum verification failed"));
        }
    }
    
    // Try to parse GGUF metadata
    match GGUFEngine::parse_gguf_metadata(model_path) {
        Ok(metadata) => {
            println!("GGUF Version: {}", metadata.version);
            println!("Tensor Count: {}", metadata.tensor_count);
            println!("Metadata Entries: {}", metadata.metadata_kv_count);
            
            // Show some key metadata
            if let Some(arch) = metadata.metadata.get("general.architecture") {
                println!("Architecture: {:?}", arch);
            }
            if let Some(name) = metadata.metadata.get("general.name") {
                println!("Model Name: {:?}", name);
            }
        }
        Err(e) => {
            warn!("Failed to parse GGUF metadata: {}", e);
        }
    }
    
    println!("✓ Model verification completed");
    Ok(())
}

async fn show_model_info(model_path: &PathBuf) -> CodexResult<()> {
    info!("Analyzing model: {}", model_path.display());
    
    if !model_path.exists() {
        return Err(CodexError::not_found(format!("Model file not found: {}", model_path.display())));
    }
    
    // Get file information
    let metadata = tokio::fs::metadata(model_path).await
        .map_err(|e| CodexError::io(e))?;
    
    println!("Model Information");
    println!("=================");
    println!("File: {}", model_path.display());
    println!("Size: {:.2} GB", metadata.len() as f64 / (1024.0 * 1024.0 * 1024.0));
    
    if let Ok(modified) = metadata.modified() {
        if let Ok(datetime) = modified.duration_since(std::time::UNIX_EPOCH) {
            let dt = chrono::DateTime::from_timestamp(datetime.as_secs() as i64, 0)
                .unwrap_or_default();
            println!("Modified: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
        }
    }
    
    // Parse GGUF information
    match GGUFEngine::parse_gguf_metadata(model_path) {
        Ok(gguf_metadata) => {
            println!();
            println!("GGUF Information");
            println!("================");
            println!("Version: {}", gguf_metadata.version);
            println!("Tensors: {}", gguf_metadata.tensor_count);
            
            // Convert to config
            match GGUFEngine::metadata_to_config(&gguf_metadata) {
                Ok(config) => {
                    println!();
                    println!("Model Configuration");
                    println!("==================");
                    println!("Vocabulary Size: {}", config.vocab_size);
                    println!("Hidden Size: {}", config.hidden_size);
                    println!("Layers: {}", config.num_hidden_layers);
                    println!("Attention Heads: {}", config.num_attention_heads);
                    println!("Context Length: {}", config.max_position_embeddings);
                    println!("RMS Norm Epsilon: {}", config.rms_norm_eps);
                    println!("RoPE Theta: {}", config.rope_theta);
                    
                    // Estimate parameters
                    let estimated_params = estimate_parameter_count(&config);
                    println!("Estimated Parameters: {:.1}B", estimated_params);
                }
                Err(e) => {
                    warn!("Failed to parse model configuration: {}", e);
                }
            }
            
            // Show some metadata
            println!();
            println!("Metadata");
            println!("========");
            for (key, value) in gguf_metadata.metadata.iter().take(10) {
                println!("{}: {:?}", key, value);
            }
            if gguf_metadata.metadata.len() > 10 {
                println!("... and {} more entries", gguf_metadata.metadata.len() - 10);
            }
        }
        Err(e) => {
            error!("Failed to parse GGUF file: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

async fn clean_models(download_dir: &PathBuf, all: bool, model_name: Option<String>) -> CodexResult<()> {
    info!("Cleaning models in: {}", download_dir.display());
    
    if all {
        // Remove all model files
        let mut entries = tokio::fs::read_dir(download_dir).await
            .map_err(|e| CodexError::io(e))?;
        
        let mut removed_count = 0;
        while let Some(entry) = entries.next_entry().await.map_err(|e| CodexError::io(e))? {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "gguf" || extension == "json" {
                    tokio::fs::remove_file(&path).await
                        .map_err(|e| CodexError::io(e))?;
                    println!("Removed: {}", path.display());
                    removed_count += 1;
                }
            }
        }
        
        println!("Removed {} files", removed_count);
    } else if let Some(model) = model_name {
        // Remove specific model
        let manifest = if model == "mistral-7b-instruct-q4k" {
            ModelManifest::mistral_7b_instruct_q4k()
        } else {
            return Err(CodexError::not_found(format!("Unknown model: {}", model)));
        };
        
        let downloader = ModelDownloader::new(download_dir.clone());
        downloader.remove_model(&manifest).await?;
        println!("Removed model: {}", model);
    } else {
        return Err(CodexError::validation("Must specify --all or --model"));
    }
    
    Ok(())
}

fn create_progress_bar(total_size: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    pb
}

async fn update_progress_bar(pb: Arc<Mutex<ProgressBar>>, progress: DownloadProgress) {
    let pb = pb.lock().await;
    
    pb.set_position(progress.downloaded_bytes);
    
    let message = match progress.stage {
        DownloadStage::Initializing => "Initializing...".to_string(),
        DownloadStage::Downloading => {
            if progress.speed_bps > 0 {
                format!("{}/s", humanize_bytes(progress.speed_bps))
            } else {
                "Downloading...".to_string()
            }
        }
        DownloadStage::Verifying => "Verifying checksum...".to_string(),
        DownloadStage::Completed => "Completed!".to_string(),
        DownloadStage::Failed(ref msg) => format!("Failed: {}", msg),
    };
    
    pb.set_message(message);
}

async fn show_model_info_from_manifest(manifest: &ModelManifest, path: &PathBuf) {
    println!();
    println!("Model Downloaded Successfully!");
    println!("==============================");
    println!("Name: {}", manifest.name);
    println!("Version: {}", manifest.version);
    println!("Architecture: {}", manifest.architecture);
    println!("Parameters: {}", manifest.parameter_count);
    println!("Quantization: {}", manifest.quantization);
    println!("Context Length: {}", manifest.context_length);
    println!("File Size: {:.2} GB", manifest.file_size as f64 / (1024.0 * 1024.0 * 1024.0));
    println!("Local Path: {}", path.display());
    
    if let Some(ref performance) = manifest.performance {
        println!("Performance: {:.1} tokens/sec on {}", 
                 performance.tokens_per_second, 
                 performance.reference_hardware);
    }
    
    println!();
    println!("Hardware Requirements:");
    println!("- Min RAM: {:.1} GB", manifest.hardware_requirements.min_ram_gb);
    println!("- Recommended RAM: {:.1} GB", manifest.hardware_requirements.recommended_ram_gb);
    if let Some(vram) = manifest.hardware_requirements.vram_gb {
        println!("- VRAM: {:.1} GB", vram);
    }
    println!("- Supported devices: {}", manifest.hardware_requirements.supported_devices.join(", "));
}

fn humanize_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn estimate_parameter_count(config: &candle_transformers::models::llama::LlamaConfig) -> f64 {
    let hidden_size = config.hidden_size;
    let vocab_size = config.vocab_size;
    let num_layers = config.num_hidden_layers;
    let intermediate_size = config.intermediate_size;
    
    // Simplified parameter count estimation
    let embedding_params = vocab_size * hidden_size;
    let attention_params = num_layers * hidden_size * hidden_size * 4; // Q, K, V, O projections
    let ffn_params = num_layers * hidden_size * intermediate_size * 2; // Up and down projections
    
    let total_params = embedding_params + attention_params + ffn_params;
    total_params as f64 / 1_000_000_000.0 // Convert to billions
}
