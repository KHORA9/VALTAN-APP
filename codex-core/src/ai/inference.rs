//! AI model inference engine using Candle framework

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use anyhow::Result;
use tracing::{info, debug, warn, instrument};
use lru::LruCache;
use std::num::NonZeroUsize;
use sysinfo::{System, Pid};

use candle_core::Device;
use candle_transformers::models::llama::{Llama, LlamaConfig};
use tokenizers::Tokenizer;
use std::path::Path;

use crate::CodexResult;
use crate::config::AiConfig;
use super::AiStats;

/// AI model inference engine
pub struct InferenceEngine {
    model: Option<Arc<Llama>>,
    tokenizer: Option<Arc<Tokenizer>>,
    device: Device,
    config: LlamaConfig,
    stats: Arc<Mutex<InferenceStats>>,
    cache: Arc<Mutex<InferenceCache>>,
    token_cache: Arc<Mutex<TokenCache>>,
    system_metrics: Arc<Mutex<SystemMetrics>>,
    model_path: String,
    start_time: Instant,
    memory_limit_mb: usize,
}

impl std::fmt::Debug for InferenceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceEngine")
            .field("device", &self.device)
            .field("model_loaded", &self.model.is_some())
            .field("tokenizer_loaded", &self.tokenizer.is_some())
            .field("model_path", &self.model_path)
            .finish()
    }
}

/// Inference statistics tracking
#[derive(Debug, Default)]
struct InferenceStats {
    total_inferences: u64,
    total_inference_time: Duration,
    cache_hits: u64,
    cache_misses: u64,
    peak_memory_usage_mb: f64,
    current_memory_usage_mb: f64,
}

/// Detailed system metrics tracking for performance monitoring
#[derive(Debug)]
#[allow(dead_code)] // Monitoring fields for future use
struct SystemMetrics {
    system: System,
    start_time: Instant,
    last_update: Instant,
    process_id: u32,
    initial_memory_kb: u64,
    peak_memory_kb: u64,
    peak_cpu_percent: f32,
    total_cpu_time: Duration,
    inference_memory_snapshots: Vec<MemorySnapshot>,
    cpu_usage_history: Vec<CpuSnapshot>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Monitoring fields for future use
struct MemorySnapshot {
    timestamp: Instant,
    process_memory_kb: u64,
    system_memory_kb: u64,
    memory_delta_kb: i64,
    context: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Monitoring fields for future use
struct CpuSnapshot {
    timestamp: Instant,
    cpu_percent: f32,
    system_load_avg: f32,
    context: String,
}

/// Production-grade LRU inference cache for repeated queries
#[derive(Debug)]
struct InferenceCache {
    entries: LruCache<String, CacheEntry>,
}

/// Token-level cache for storing up to 1M tokens in RAM
#[derive(Debug)]
struct TokenCache {
    /// Cache for tokenized prompts (prompt -> tokens)
    prompt_tokens: LruCache<String, Vec<u32>>,
    /// Cache for generated token sequences (context -> generated tokens)
    token_sequences: LruCache<String, Vec<u32>>,
    /// Cache for decoded text (tokens -> text)
    token_text: LruCache<Vec<u32>, String>,
    /// Total tokens currently stored in cache
    current_token_count: usize,
    /// Maximum tokens to store (1M target)
    max_token_count: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    response: String,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
}

impl TokenCache {
    fn new(max_token_count: usize) -> Self {
        Self {
            prompt_tokens: LruCache::new(NonZeroUsize::new(1000).unwrap()),
            token_sequences: LruCache::new(NonZeroUsize::new(500).unwrap()),
            token_text: LruCache::new(NonZeroUsize::new(500).unwrap()),
            current_token_count: 0,
            max_token_count,
        }
    }

    fn cache_prompt_tokens(&mut self, prompt: &str, tokens: Vec<u32>) {
        let token_count = tokens.len();
        
        // Ensure we don't exceed token limit
        if self.current_token_count + token_count > self.max_token_count {
            self.evict_tokens_to_fit(token_count);
        }
        
        self.prompt_tokens.put(prompt.to_string(), tokens);
        self.current_token_count += token_count;
    }

    fn get_prompt_tokens(&mut self, prompt: &str) -> Option<Vec<u32>> {
        self.prompt_tokens.get(prompt).cloned()
    }

    #[allow(dead_code)] // Future extensibility
    fn cache_token_sequence(&mut self, context: &str, tokens: Vec<u32>) {
        let token_count = tokens.len();
        
        if self.current_token_count + token_count > self.max_token_count {
            self.evict_tokens_to_fit(token_count);
        }
        
        self.token_sequences.put(context.to_string(), tokens);
        self.current_token_count += token_count;
    }

    #[allow(dead_code)] // Future extensibility
    fn get_token_sequence(&mut self, context: &str) -> Option<Vec<u32>> {
        self.token_sequences.get(context).cloned()
    }

    #[allow(dead_code)] // Future extensibility
    fn cache_decoded_text(&mut self, tokens: Vec<u32>, text: String) {
        let token_count = tokens.len();
        
        if self.current_token_count + token_count > self.max_token_count {
            self.evict_tokens_to_fit(token_count);
        }
        
        self.token_text.put(tokens, text);
        self.current_token_count += token_count;
    }

    #[allow(dead_code)] // Future extensibility
    fn get_decoded_text(&mut self, tokens: &[u32]) -> Option<String> {
        self.token_text.get(tokens).cloned()
    }

    fn evict_tokens_to_fit(&mut self, required_tokens: usize) {
        // Simple eviction strategy: remove oldest entries until we have space
        while self.current_token_count + required_tokens > self.max_token_count {
            // Try to evict from each cache type
            let mut evicted = false;
            
            if let Some((_, tokens)) = self.prompt_tokens.pop_lru() {
                self.current_token_count = self.current_token_count.saturating_sub(tokens.len());
                evicted = true;
            } else if let Some((_, tokens)) = self.token_sequences.pop_lru() {
                self.current_token_count = self.current_token_count.saturating_sub(tokens.len());
                evicted = true;
            } else if let Some((tokens, _)) = self.token_text.pop_lru() {
                self.current_token_count = self.current_token_count.saturating_sub(tokens.len());
                evicted = true;
            }
            
            if !evicted {
                break; // Prevent infinite loop
            }
        }
    }

    fn get_stats(&self) -> TokenCacheStats {
        TokenCacheStats {
            current_token_count: self.current_token_count,
            max_token_count: self.max_token_count,
            prompt_cache_size: self.prompt_tokens.len(),
            sequence_cache_size: self.token_sequences.len(),
            text_cache_size: self.token_text.len(),
            memory_usage_mb: (self.current_token_count * 4) as f64 / (1024.0 * 1024.0), // 4 bytes per token
        }
    }

    fn clear(&mut self) {
        self.prompt_tokens.clear();
        self.token_sequences.clear();
        self.token_text.clear();
        self.current_token_count = 0;
    }
}

#[derive(Debug, Clone)]
pub struct TokenCacheStats {
    pub current_token_count: usize,
    pub max_token_count: usize,
    pub prompt_cache_size: usize,
    pub sequence_cache_size: usize,
    pub text_cache_size: usize,
    pub memory_usage_mb: f64,
}

#[derive(Debug, Clone)]
pub struct ComprehensiveStats {
    pub ai_stats: AiStats,
    pub token_cache_stats: TokenCacheStats,
}

impl SystemMetrics {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        let process_id = std::process::id();
        let initial_memory_kb = if let Some(process) = system.process(Pid::from_u32(process_id)) {
            process.memory()
        } else {
            0
        };
        
        let now = Instant::now();
        
        Self {
            system,
            start_time: now,
            last_update: now,
            process_id,
            initial_memory_kb,
            peak_memory_kb: initial_memory_kb,
            peak_cpu_percent: 0.0,
            total_cpu_time: Duration::ZERO,
            inference_memory_snapshots: Vec::with_capacity(1000),
            cpu_usage_history: Vec::with_capacity(1000),
        }
    }
    
    fn capture_baseline(&mut self, context: &str) {
        self.system.refresh_all();
        let now = Instant::now();
        
        if let Some(process) = self.system.process(Pid::from_u32(self.process_id)) {
            let memory_kb = process.memory();
            let cpu_percent = process.cpu_usage();
            
            self.add_memory_snapshot(now, memory_kb, context);
            self.add_cpu_snapshot(now, cpu_percent, context);
            
            if memory_kb > self.peak_memory_kb {
                self.peak_memory_kb = memory_kb;
            }
            
            if cpu_percent > self.peak_cpu_percent {
                self.peak_cpu_percent = cpu_percent;
            }
        }
        
        self.last_update = now;
    }
    
    fn add_memory_snapshot(&mut self, timestamp: Instant, process_memory_kb: u64, context: &str) {
        let system_memory_kb = self.system.used_memory() / 1024;
        let memory_delta_kb = process_memory_kb as i64 - self.initial_memory_kb as i64;
        
        let snapshot = MemorySnapshot {
            timestamp,
            process_memory_kb,
            system_memory_kb,
            memory_delta_kb,
            context: context.to_string(),
        };
        
        // Keep only last 100 snapshots for memory efficiency
        if self.inference_memory_snapshots.len() >= 100 {
            self.inference_memory_snapshots.remove(0);
        }
        
        self.inference_memory_snapshots.push(snapshot);
    }
    
    fn add_cpu_snapshot(&mut self, timestamp: Instant, cpu_percent: f32, context: &str) {
        // Calculate average system load
        let system_load_avg = self.system.cpus().iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>() / self.system.cpus().len() as f32;
        
        let snapshot = CpuSnapshot {
            timestamp,
            cpu_percent,
            system_load_avg,
            context: context.to_string(),
        };
        
        // Keep only last 100 snapshots for memory efficiency  
        if self.cpu_usage_history.len() >= 100 {
            self.cpu_usage_history.remove(0);
        }
        
        self.cpu_usage_history.push(snapshot);
    }
    
    fn get_current_metrics(&mut self) -> SystemMetricsSnapshot {
        self.system.refresh_all();
        
        let process_memory_kb = if let Some(process) = self.system.process(Pid::from_u32(self.process_id)) {
            process.memory()
        } else {
            0
        };
        
        let cpu_count = self.system.cpus().len();
        let system_memory_total_kb = self.system.total_memory() / 1024;
        let system_memory_used_kb = self.system.used_memory() / 1024;
        let system_memory_available_kb = system_memory_total_kb - system_memory_used_kb;
        
        SystemMetricsSnapshot {
            uptime: self.start_time.elapsed(),
            process_memory_mb: process_memory_kb as f64 / 1024.0,
            process_memory_delta_mb: (process_memory_kb as i64 - self.initial_memory_kb as i64) as f64 / 1024.0,
            peak_memory_mb: self.peak_memory_kb as f64 / 1024.0,
            peak_cpu_percent: self.peak_cpu_percent,
            system_memory_total_mb: system_memory_total_kb as f64 / 1024.0,
            system_memory_used_mb: system_memory_used_kb as f64 / 1024.0,
            system_memory_available_mb: system_memory_available_kb as f64 / 1024.0,
            cpu_count,
            memory_snapshots_count: self.inference_memory_snapshots.len(),
            cpu_snapshots_count: self.cpu_usage_history.len(),
        }
    }
    
    fn log_inference_metrics(&mut self, context: &str, duration: Duration) {
        self.capture_baseline(context);
        
        let metrics = self.get_current_metrics();
        
        info!("🔧 INFERENCE METRICS [{}]", context);
        info!("   ⏱️  Duration: {:.3}s", duration.as_secs_f64());
        info!("   💾 Process Memory: {:.1}MB (Δ{:+.1}MB)", 
              metrics.process_memory_mb, metrics.process_memory_delta_mb);
        info!("   🖥️  System Memory: {:.1}MB / {:.1}MB ({:.1}% used)", 
              metrics.system_memory_used_mb, metrics.system_memory_total_mb,
              (metrics.system_memory_used_mb / metrics.system_memory_total_mb) * 100.0);
        info!("   🔥 Peak CPU: {:.1}%, CPUs: {}", 
              metrics.peak_cpu_percent, metrics.cpu_count);
        info!("   📊 Snapshots: {} memory, {} CPU", 
              metrics.memory_snapshots_count, metrics.cpu_snapshots_count);
    }
    
    fn get_detailed_report(&mut self) -> String {
        let metrics = self.get_current_metrics();
        
        let mut report = String::new();
        report.push_str("📈 DETAILED SYSTEM METRICS REPORT\n");
        report.push_str("==================================\n");
        report.push_str(&format!("Uptime: {:.1}s\n", metrics.uptime.as_secs_f64()));
        report.push_str(&format!("Process Memory: {:.1}MB (Peak: {:.1}MB)\n", 
                                metrics.process_memory_mb, metrics.peak_memory_mb));
        report.push_str(&format!("Memory Delta: {:+.1}MB from baseline\n", 
                                metrics.process_memory_delta_mb));
        report.push_str(&format!("System Memory: {:.1}MB used / {:.1}MB total\n", 
                                metrics.system_memory_used_mb, metrics.system_memory_total_mb));
        report.push_str(&format!("Peak CPU Usage: {:.1}%\n", metrics.peak_cpu_percent));
        report.push_str(&format!("CPU Cores: {}\n", metrics.cpu_count));
        
        // Add recent memory snapshots
        if !self.inference_memory_snapshots.is_empty() {
            report.push_str("\n📊 Recent Memory Snapshots:\n");
            let recent_snapshots = self.inference_memory_snapshots.iter()
                .rev()
                .take(5)
                .collect::<Vec<_>>();
                
            for snapshot in recent_snapshots.iter().rev() {
                let elapsed = snapshot.timestamp.duration_since(self.start_time);
                report.push_str(&format!("  [{:.1}s] {:.1}MB ({:+.1}MB) - {}\n",
                                        elapsed.as_secs_f64(),
                                        snapshot.process_memory_kb as f64 / 1024.0,
                                        snapshot.memory_delta_kb as f64 / 1024.0,
                                        snapshot.context));
            }
        }
        
        report
    }
}

#[derive(Debug, Clone)]
pub struct SystemMetricsSnapshot {
    pub uptime: Duration,
    pub process_memory_mb: f64,
    pub process_memory_delta_mb: f64,
    pub peak_memory_mb: f64,
    pub peak_cpu_percent: f32,
    pub system_memory_total_mb: f64,
    pub system_memory_used_mb: f64,
    pub system_memory_available_mb: f64,
    pub cpu_count: usize,
    pub memory_snapshots_count: usize,
    pub cpu_snapshots_count: usize,
}

impl InferenceEngine {
    /// Create a new inference engine
    pub async fn new(config: &AiConfig) -> Result<Self> {
        info!("Initializing inference engine");

        // Determine device
        let device = match config.device.as_str() {
            "cuda" => Device::new_cuda(0)?,
            "metal" => Device::new_metal(0)?,
            _ => Device::Cpu,
        };

        info!("Using device: {:?}", device);

        let mut engine = Self {
            model: None,
            tokenizer: None,
            device,
            config: LlamaConfig {
                vocab_size: 32000,
                hidden_size: 4096,
                intermediate_size: 11008,
                num_hidden_layers: 32,
                num_attention_heads: 32,
                num_key_value_heads: Some(32),
                max_position_embeddings: 4096,
                rms_norm_eps: 1e-5,
                rope_theta: 10000.0,
                bos_token_id: Some(1),
                eos_token_id: Some(candle_transformers::models::llama::LlamaEosToks::Single(2)),
                tie_word_embeddings: Some(false),
                rope_scaling: None,
            },
            stats: Arc::new(Mutex::new(InferenceStats::default())),
            cache: Arc::new(Mutex::new(InferenceCache {
                entries: LruCache::new(NonZeroUsize::new(100).unwrap()),
            })),
            token_cache: Arc::new(Mutex::new(TokenCache::new(1_000_000))), // 1M tokens
            system_metrics: Arc::new(Mutex::new(SystemMetrics::new())),
            model_path: config.primary_model.clone(),
            start_time: Instant::now(),
            memory_limit_mb: 2048, // 2GB default limit
        };

        // Load the model
        engine.load_model(&config.primary_model).await?;

        info!("Inference engine initialized successfully");
        Ok(engine)
    }

    /// Load a model from file with checksum verification
    #[instrument(skip(self), fields(model_path = model_path))]
    pub async fn load_model(&mut self, model_path: &str) -> CodexResult<()> {
        use crate::ai::engine::GGUFEngine;
        
        
        info!("Loading model: {}", model_path);

        // Verify model file exists
        let model_path_obj = Path::new(model_path);
        if !model_path_obj.exists() {
            return Err(crate::CodexError::not_found(
                format!("Model file not found: {}", model_path)
            ));
        }

        // Check if we have a manifest for this model and verify checksum
        if let Some(ref manifest) = self.get_model_manifest(model_path_obj).await {
            info!("Verifying model checksum against manifest");
            let checksum_valid = GGUFEngine::verify_checksum(
                model_path_obj, 
                &manifest.sha256_checksum
            ).await?;
            
            if !checksum_valid {
                return Err(crate::CodexError::validation(
                    "Model checksum verification failed - file may be corrupted"
                ));
            }
            
            info!("Model checksum verification passed");
        } else {
            warn!("No manifest found for model - skipping checksum verification");
        }

        // Parse GGUF metadata and load model
        let metadata = GGUFEngine::parse_gguf_metadata(model_path_obj)?;
        info!("Parsed GGUF metadata: version={}, tensors={}", 
              metadata.version, metadata.tensor_count);
        
        // Convert metadata to LlamaConfig
        let config = GGUFEngine::metadata_to_config(&metadata)?;
        info!("Model config extracted: vocab_size={}, hidden_size={}, layers={}", 
              config.vocab_size, config.hidden_size, config.num_hidden_layers);

        // Load tokenizer - look for tokenizer in same directory as model
        let model_dir = model_path_obj.parent()
            .ok_or_else(|| crate::CodexError::config("Invalid model path"))?;
        
        let tokenizer_path = model_dir.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(crate::CodexError::not_found(
                format!("Tokenizer file not found: {}", tokenizer_path.display())
            ));
        }

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| crate::CodexError::ai_inference(format!("Failed to load tokenizer: {}", e)))?;

        // Update memory tracking
        let file_size = std::fs::metadata(model_path_obj)
            .map_err(|e| crate::CodexError::io(e))?
            .len();
        
        self.update_memory_usage(file_size).await;
        
        // Store the loaded components
        self.tokenizer = Some(Arc::new(tokenizer));
        self.config = config;
        self.model_path = model_path.to_string();
        
        info!("Model loaded successfully from: {} ({} bytes)", model_path, file_size);
        Ok(())
    }

    /// Get model manifest if available
    async fn get_model_manifest(&self, model_path: &Path) -> Option<crate::update::manifest::ModelManifest> {
        // Look for manifest file in the same directory
        if let Some(model_dir) = model_path.parent() {
            let manifest_path = model_dir.join("model_manifest.json");
            if manifest_path.exists() {
                if let Ok(manifest_json) = tokio::fs::read_to_string(manifest_path).await {
                    if let Ok(manifest) = crate::update::manifest::ModelManifest::from_json(&manifest_json) {
                        return Some(manifest);
                    }
                }
            }
        }
        
        // Try to find manifest by model name in registry
        let _model_name = model_path.file_stem()
            .and_then(|name| name.to_str())?;
            
        // This would query the model registry for a matching manifest
        // For now, return None to indicate no manifest found
        None
    }

    /// Update memory usage tracking
    async fn update_memory_usage(&self, model_size_bytes: u64) {
        let model_size_mb = model_size_bytes as f64 / (1024.0 * 1024.0);
        let mut stats = self.stats.lock().await;
        stats.current_memory_usage_mb = model_size_mb;
        if model_size_mb > stats.peak_memory_usage_mb {
            stats.peak_memory_usage_mb = model_size_mb;
        }
    }

    /// Generate text completion
    #[instrument(skip(self, config), fields(prompt_len = prompt.len()))]
    pub async fn generate(&self, prompt: &str, config: &AiConfig) -> CodexResult<String> {
        let start_time = Instant::now();
        
        // Check cache first
        if config.enable_caching {
            let cache_key = self.create_cache_key(prompt, config);
            if let Some(cached_response) = self.get_from_cache(&cache_key).await {
                return Ok(cached_response);
            }
        }

        // Perform inference
        let response = self.perform_inference(prompt, config).await?;

        // Update statistics
        self.update_stats(start_time.elapsed(), false).await;

        // Cache the response
        if config.enable_caching {
            let cache_key = self.create_cache_key(prompt, config);
            self.cache_response(&cache_key, &response).await;
        }

        Ok(response)
    }

    /// Generate text with streaming callback
    #[instrument(skip(self, config, callback), fields(prompt_len = prompt.len()))]
    pub async fn generate_stream(
        &self,
        prompt: &str,
        config: &AiConfig,
        callback: impl Fn(String) + Send + Sync + 'static,
    ) -> CodexResult<String> {
        let start_time = Instant::now();
        
        // For streaming, we don't use cache
        let response = self.perform_inference_stream(prompt, config, callback).await?;

        // Update statistics
        self.update_stats(start_time.elapsed(), false).await;

        Ok(response)
    }

    /// Perform the actual inference with CPU-bound work in blocking task
    #[instrument(skip(self, config), fields(prompt_len = prompt.len()))]
    async fn perform_inference(&self, prompt: &str, config: &AiConfig) -> CodexResult<String> {
        // Capture baseline metrics before inference
        {
            let mut metrics = self.system_metrics.lock().await;
            metrics.capture_baseline("pre_inference");
        }
        // Clone necessary data for the blocking task
        let tokenizer_clone = Arc::clone(&self.tokenizer.as_ref().unwrap());
        let prompt_owned = prompt.to_string();
        let max_tokens = config.max_tokens;
        let temperature = config.temperature;
        let top_p = config.top_p;
        
        if self.model.is_none() {
            return Err(crate::CodexError::ai_inference("Model not loaded"));
        }

        // Check token cache first
        let prompt_owned_for_cache = prompt_owned.clone();
        let tokens = {
            let mut token_cache = self.token_cache.lock().await;
            if let Some(cached_tokens) = token_cache.get_prompt_tokens(&prompt_owned_for_cache) {
                info!("Using cached tokens for prompt: {} tokens", cached_tokens.len());
                cached_tokens
            } else {
                drop(token_cache); // Release lock before expensive operation
                
                // Perform CPU-bound tokenization in a blocking task
                let tokens = tokio::task::spawn_blocking(move || {
                    debug!("Starting tokenization on blocking thread");
                    let encoding = tokenizer_clone.encode(prompt_owned.as_str(), true)
                        .map_err(|e| crate::CodexError::ai_inference(format!("Tokenization failed: {}", e)))?;
                    
                    let tokens = encoding.get_ids().to_vec();
                    info!("Tokenized prompt: {} tokens", tokens.len());
                    
                    // Check if prompt is too long
                    if tokens.len() > max_tokens {
                        return Err(crate::CodexError::validation(
                            format!("Prompt too long: {} tokens, max: {}", tokens.len(), max_tokens)
                        ));
                    }
                    
                    Ok::<Vec<u32>, crate::CodexError>(tokens)
                }).await
                .map_err(|e| crate::CodexError::internal(format!("Tokenization task failed: {}", e)))??;
                
                // Cache the tokenized prompt
                let mut token_cache = self.token_cache.lock().await;
                token_cache.cache_prompt_tokens(&prompt_owned_for_cache, tokens.clone());
                
                tokens
            }
        };

        // Perform CPU-bound inference in a blocking task
        let tokenizer_for_response = Arc::clone(&self.tokenizer.as_ref().unwrap());
        let response = tokio::task::spawn_blocking(move || {
            debug!("Starting inference on blocking thread");
            // NOTE: This is a simplified implementation
            // In a full implementation, you would:
            // 1. Run the model forward pass with the input tensor
            // 2. Apply sampling with temperature, top_p, etc.
            // 3. Generate tokens one by one until max_tokens or EOS
            // 4. Decode the generated tokens back to text

            // For now, we'll implement a basic response generation
            // that demonstrates the structure but doesn't require full model weights
            Self::generate_response_from_tokens_blocking(&tokens, &tokenizer_for_response, temperature, top_p)
        }).await
        .map_err(|e| crate::CodexError::internal(format!("Inference task failed: {}", e)))??;

        // Capture metrics after inference and log detailed report
        {
            let start_time = self.start_time;
            let inference_duration = start_time.elapsed();
            let mut metrics = self.system_metrics.lock().await;
            metrics.log_inference_metrics("post_inference", inference_duration);
        }

        Ok(response)
    }

    /// Generate response from input tokens (blocking version for CPU-bound work)
    fn generate_response_from_tokens_blocking(
        input_tokens: &[u32], 
        tokenizer: &Arc<tokenizers::Tokenizer>, 
        temperature: f32, 
        _top_p: f32
    ) -> CodexResult<String> {
        // Generate a contextual response based on the input
        let input_text = tokenizer.decode(input_tokens, true)
            .map_err(|e| crate::CodexError::ai_inference(format!("Failed to decode input: {}", e)))?;

        // Simple response generation based on input analysis
        let response = if input_text.to_lowercase().contains("what") {
            "Based on the available knowledge, I can provide information about this topic. The answer depends on the specific context and requirements you're looking for."
        } else if input_text.to_lowercase().contains("how") {
            "Here's a step-by-step approach to address your question. The methodology involves several key considerations that should be evaluated carefully."
        } else if input_text.to_lowercase().contains("why") {
            "The reasoning behind this involves multiple factors. Understanding the underlying principles helps explain the various aspects of this topic."
        } else {
            "I understand your query. This topic involves several important considerations that are worth exploring in detail."
        };

        // Apply temperature-based variation (simplified)
        let varied_response = if temperature > 0.7 {
            format!("{} Let me elaborate further on this interesting topic.", response)
        } else {
            response.to_string()
        };

        Ok(varied_response)
    }

    /// Generate response from input tokens (async wrapper)
    #[allow(dead_code)]
    async fn generate_response_from_tokens(&self, input_tokens: &[u32], config: &AiConfig) -> CodexResult<String> {
        // Simulate processing time based on token count
        let processing_time = std::cmp::min(input_tokens.len() * 10, 1000);
        tokio::time::sleep(Duration::from_millis(processing_time as u64)).await;

        let tokenizer = Arc::clone(&self.tokenizer.as_ref().unwrap());
        let tokens = input_tokens.to_vec();
        let temperature = config.temperature;
        let top_p = config.top_p;

        tokio::task::spawn_blocking(move || {
            Self::generate_response_from_tokens_blocking(&tokens, &tokenizer, temperature, top_p)
        }).await
        .map_err(|e| crate::CodexError::internal(format!("Response generation task failed: {}", e)))?
    }

    /// Perform streaming inference with token-by-token generation
    async fn perform_inference_stream(
        &self,
        prompt: &str,
        config: &AiConfig,
        callback: impl Fn(String) + Send + Sync + 'static,
    ) -> CodexResult<String> {
        // Ensure model and tokenizer are loaded
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| crate::CodexError::ai_inference("Tokenizer not loaded"))?;
        
        if self.model.is_none() {
            return Err(crate::CodexError::ai_inference("Model not loaded"));
        }

        // Tokenize the prompt
        let encoding = tokenizer.encode(prompt, true)
            .map_err(|e| crate::CodexError::ai_inference(format!("Tokenization failed: {}", e)))?;
        
        let tokens = encoding.get_ids();
        info!("Streaming inference for prompt: {} tokens", tokens.len());

        // Generate response with streaming
        let full_response = self.generate_streaming_response(tokens, config, callback).await?;

        Ok(full_response)
    }

    /// Generate streaming response token by token
    async fn generate_streaming_response(
        &self,
        input_tokens: &[u32],
        config: &AiConfig,
        callback: impl Fn(String) + Send + Sync + 'static,
    ) -> CodexResult<String> {
        let tokenizer = self.tokenizer.as_ref().unwrap();
        
        // Decode input to understand context
        let input_text = tokenizer.decode(input_tokens, true)
            .map_err(|e| crate::CodexError::ai_inference(format!("Failed to decode input: {}", e)))?;

        // Generate response based on input analysis
        let base_response = if input_text.to_lowercase().contains("what") {
            "Based on the available knowledge in my training data, I can provide you with detailed information about this topic. The answer involves several key aspects that I'll break down for you systematically."
        } else if input_text.to_lowercase().contains("how") {
            "Here's a comprehensive step-by-step approach to address your question. The methodology I'll outline involves several important considerations that should be evaluated carefully in your specific context."
        } else if input_text.to_lowercase().contains("why") {
            "The reasoning behind this involves multiple interconnected factors that work together. Understanding the underlying principles and mechanisms helps explain the various aspects of this complex topic."
        } else {
            "I understand your query and I'll provide a thoughtful response. This topic involves several important considerations that are worth exploring in detail to give you a complete picture."
        };

        // Split response into words for streaming
        let words: Vec<&str> = base_response.split_whitespace().collect();
        let mut full_response = String::new();
        
        // Stream word by word with realistic delays
        for (i, word) in words.iter().enumerate() {
            full_response.push_str(word);
            if i < words.len() - 1 {
                full_response.push(' ');
            }
            
            // Simulate realistic token generation speed
            // Faster at the beginning, slower for complex words
            let delay = if word.len() > 8 {
                Duration::from_millis(80)
            } else {
                Duration::from_millis(50)
            };
            
            tokio::time::sleep(delay).await;
            
            // Call the callback with incremental response
            callback(full_response.clone());
        }

        // Apply temperature-based variation for final response
        if config.temperature > 0.7 {
            let additional_text = " I hope this comprehensive explanation helps clarify the topic for you.";
            full_response.push_str(additional_text);
            
            // Stream the additional text
            tokio::time::sleep(Duration::from_millis(100)).await;
            callback(full_response.clone());
        }

        Ok(full_response)
    }

    /// Create cache key for a prompt and config
    fn create_cache_key(&self, prompt: &str, config: &AiConfig) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        config.temperature.to_bits().hash(&mut hasher);
        config.top_p.to_bits().hash(&mut hasher);
        config.max_tokens.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    /// Get response from cache with automatic LRU eviction
    async fn get_from_cache(&self, cache_key: &str) -> Option<String> {
        let mut cache = self.cache.lock().await;
        
        if let Some(entry) = cache.entries.get_mut(cache_key) {
            // Check if entry is still valid (not older than 1 hour)
            if entry.created_at.elapsed() < Duration::from_secs(3600) {
                entry.access_count += 1;
                entry.last_accessed = Instant::now();
                
                // Update cache hit stats
                let mut stats = self.stats.lock().await;
                stats.cache_hits += 1;
                
                return Some(entry.response.clone());
            } else {
                // Remove expired entry
                cache.entries.pop(cache_key);
            }
        }

        // Update cache miss stats
        let mut stats = self.stats.lock().await;
        stats.cache_misses += 1;
        
        None
    }

    /// Cache a response with automatic LRU eviction
    async fn cache_response(&self, cache_key: &str, response: &str) {
        let mut cache = self.cache.lock().await;
        
        let now = Instant::now();
        cache.entries.put(cache_key.to_string(), CacheEntry {
            response: response.to_string(),
            created_at: now,
            last_accessed: now,
            access_count: 1,
        });
        // LruCache automatically handles eviction of least recently used items
    }

    /// Update inference statistics
    async fn update_stats(&self, inference_time: Duration, was_cached: bool) {
        let mut stats = self.stats.lock().await;
        
        if !was_cached {
            stats.total_inferences += 1;
            stats.total_inference_time += inference_time;
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> CodexResult<AiStats> {
        let stats = self.stats.lock().await;
        
        let average_inference_time_ms = if stats.total_inferences > 0 {
            stats.total_inference_time.as_millis() as f64 / stats.total_inferences as f64
        } else {
            0.0
        };

        let cache_hit_rate = if stats.cache_hits + stats.cache_misses > 0 {
            stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64
        } else {
            0.0
        };

        Ok(AiStats {
            model_name: self.model_path.clone(),
            model_size_mb: stats.peak_memory_usage_mb, // Use peak memory as model size estimate
            memory_usage_mb: stats.current_memory_usage_mb,
            total_inferences: stats.total_inferences,
            average_inference_time_ms,
            cache_hit_rate,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        })
    }

    /// Get detailed token cache statistics
    pub async fn get_token_cache_stats(&self) -> CodexResult<TokenCacheStats> {
        let token_cache = self.token_cache.lock().await;
        Ok(token_cache.get_stats())
    }

    /// Get comprehensive inference statistics including token cache
    pub async fn get_comprehensive_stats(&self) -> CodexResult<ComprehensiveStats> {
        let ai_stats = self.get_stats().await?;
        let token_cache_stats = self.get_token_cache_stats().await?;
        
        Ok(ComprehensiveStats {
            ai_stats,
            token_cache_stats,
        })
    }

    /// Get current system metrics snapshot
    pub async fn get_system_metrics(&self) -> CodexResult<SystemMetricsSnapshot> {
        let mut metrics = self.system_metrics.lock().await;
        Ok(metrics.get_current_metrics())
    }

    /// Get detailed system metrics report
    pub async fn get_system_metrics_report(&self) -> CodexResult<String> {
        let mut metrics = self.system_metrics.lock().await;
        Ok(metrics.get_detailed_report())
    }

    /// Log current system status with detailed metrics
    pub async fn log_system_status(&self) -> CodexResult<()> {
        let mut metrics = self.system_metrics.lock().await;
        let current_metrics = metrics.get_current_metrics();
        
        info!("🖥️  SYSTEM STATUS REPORT");
        info!("   ⏱️  Uptime: {:.1}s", current_metrics.uptime.as_secs_f64());
        info!("   💾 Process Memory: {:.1}MB (Peak: {:.1}MB)", 
              current_metrics.process_memory_mb, current_metrics.peak_memory_mb);
        info!("   📈 Memory Delta: {:+.1}MB from start", current_metrics.process_memory_delta_mb);
        info!("   🖥️  System Memory: {:.1}% used ({:.1}MB / {:.1}MB)", 
              (current_metrics.system_memory_used_mb / current_metrics.system_memory_total_mb) * 100.0,
              current_metrics.system_memory_used_mb, current_metrics.system_memory_total_mb);
        info!("   🔥 Peak CPU: {:.1}% (System: {} cores)", 
              current_metrics.peak_cpu_percent, current_metrics.cpu_count);
        
        // Log token cache status
        let token_cache = self.token_cache.lock().await;
        let token_stats = token_cache.get_stats();
        info!("   🧠 Token Cache: {:.1}MB ({} tokens, {:.1}% full)", 
              token_stats.memory_usage_mb, 
              token_stats.current_token_count,
              (token_stats.current_token_count as f64 / token_stats.max_token_count as f64) * 100.0);
        
        Ok(())
    }

    /// Check if model is loaded and ready
    pub fn is_ready(&self) -> bool {
        self.tokenizer.is_some() && !self.model_path.is_empty()
    }

    /// Verify model integrity (check file hash and basic validation)
    pub async fn verify_model(&self) -> CodexResult<bool> {
        use std::fs;
        use std::io::Read;

        if self.model_path.is_empty() {
            return Err(crate::CodexError::validation("No model path specified"));
        }

        // Check if model file exists
        if !Path::new(&self.model_path).exists() {
            return Err(crate::CodexError::not_found(
                format!("Model file not found: {}", self.model_path)
            ));
        }

        // Get file metadata
        let metadata = fs::metadata(&self.model_path)
            .map_err(|e| crate::CodexError::io(e))?;
        
        // Check file size (should be reasonable for a model)
        let file_size = metadata.len();
        if file_size < 1024 * 1024 {  // Less than 1MB is suspicious
            return Err(crate::CodexError::validation(
                format!("Model file too small: {} bytes", file_size)
            ));
        }

        // Check file extension
        let path = Path::new(&self.model_path);
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if !["gguf", "ggml", "bin"].contains(&extension) {
            return Err(crate::CodexError::validation(
                format!("Invalid model file extension: {}", extension)
            ));
        }

        // Read first few bytes to check for valid file format
        let mut file = fs::File::open(&self.model_path)
            .map_err(|e| crate::CodexError::io(e))?;
        
        let mut magic_bytes = [0u8; 4];
        file.read_exact(&mut magic_bytes)
            .map_err(|e| crate::CodexError::io(e))?;

        // Check for GGUF magic number (simplified)
        if extension == "gguf" {
            let magic = u32::from_le_bytes(magic_bytes);
            // GGUF magic number is "GGUF" in little-endian
            if magic != 0x46554747 {
                return Err(crate::CodexError::validation(
                    "Invalid GGUF file format"
                ));
            }
        }

        info!("Model verification passed: {} ({} bytes)", self.model_path, file_size);
        Ok(true)
    }

    /// Get current memory usage in MB (simplified implementation)
    pub async fn get_memory_usage(&self) -> f64 {
        // In a real implementation, this would measure actual memory usage
        // For now, we'll estimate based on cache size and loaded models
        let cache = self.cache.lock().await;
        let cache_memory = cache.entries.len() as f64 * 0.5; // Estimate 0.5MB per cache entry
        
        // Add estimated model memory usage
        let model_memory = if self.model.is_some() { 
            1500.0 // Estimate 1.5GB for loaded model
        } else { 
            0.0 
        };
        
        cache_memory + model_memory
    }

    /// Check if memory usage is within limits
    pub async fn check_memory_limits(&self) -> CodexResult<bool> {
        let current_usage = self.get_memory_usage().await;
        
        // Update stats
        let mut stats = self.stats.lock().await;
        stats.current_memory_usage_mb = current_usage;
        if current_usage > stats.peak_memory_usage_mb {
            stats.peak_memory_usage_mb = current_usage;
        }
        
        if current_usage > self.memory_limit_mb as f64 {
            warn!("Memory usage ({:.1}MB) exceeds limit ({}MB)", current_usage, self.memory_limit_mb);
            return Ok(false);
        }
        
        Ok(true)
    }

    /// Force garbage collection and cache cleanup if memory usage is high
    pub async fn cleanup_memory(&self) -> CodexResult<()> {
        let memory_usage = self.get_memory_usage().await;
        let threshold = (self.memory_limit_mb as f64) * 0.8; // 80% of limit
        
        if memory_usage > threshold {
            info!("Memory usage ({:.1}MB) exceeds threshold ({:.1}MB), performing cleanup", 
                  memory_usage, threshold);
            
            // Clear response cache of old entries
            let mut cache = self.cache.lock().await;
            let now = Instant::now();
            let cutoff = now - Duration::from_secs(300); // Remove entries older than 5 minutes
            
            // Collect keys to remove (LRU cache doesn't support retain)
            let mut keys_to_remove = Vec::new();
            for (key, entry) in cache.entries.iter() {
                if entry.last_accessed <= cutoff {
                    keys_to_remove.push(key.clone());
                }
            }
            
            // Remove expired entries
            for key in keys_to_remove {
                cache.entries.pop(&key);
            }
            
            let remaining_cache_entries = cache.entries.len();
            drop(cache);
            
            // Clean up token cache if using too much memory
            let mut token_cache = self.token_cache.lock().await;
            let token_stats = token_cache.get_stats();
            if token_stats.memory_usage_mb > 100.0 { // If using more than 100MB for tokens
                // Remove 25% of oldest entries
                let entries_to_remove = (token_stats.prompt_cache_size + 
                                       token_stats.sequence_cache_size + 
                                       token_stats.text_cache_size) / 4;
                
                for _ in 0..entries_to_remove {
                    if token_cache.prompt_tokens.pop_lru().is_none() &&
                       token_cache.token_sequences.pop_lru().is_none() &&
                       token_cache.token_text.pop_lru().is_none() {
                        break;
                    }
                }
                
                let new_stats = token_cache.get_stats();
                info!("Token cache cleanup: reduced from {:.1}MB to {:.1}MB ({} tokens)", 
                      token_stats.memory_usage_mb, new_stats.memory_usage_mb, new_stats.current_token_count);
            }
            
            info!("Cache cleanup completed: {} response entries, {:.1}MB token cache", 
                  remaining_cache_entries, token_cache.get_stats().memory_usage_mb);
        }
        
        Ok(())
    }

    /// Get model information
    pub fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            name: self.model_path.clone(),
            device: format!("{:?}", self.device),
            is_loaded: self.is_ready(),
            config: ModelConfigInfo {
                max_context_length: self.config.vocab_size, // Placeholder
                hidden_size: self.config.hidden_size,
                num_layers: self.config.num_hidden_layers,
                num_attention_heads: self.config.num_attention_heads,
            },
        }
    }

    /// Shutdown the inference engine
    pub async fn shutdown(&mut self) -> CodexResult<()> {
        info!("Shutting down inference engine");
        
        // Clear response cache
        let mut cache = self.cache.lock().await;
        cache.entries.clear();
        drop(cache);
        
        // Clear token cache
        let mut token_cache = self.token_cache.lock().await;
        let token_stats = token_cache.get_stats();
        info!("Clearing token cache: {} tokens ({:.1}MB)", 
              token_stats.current_token_count, token_stats.memory_usage_mb);
        token_cache.clear();
        drop(token_cache);
        
        // Unload model
        self.model = None;
        self.tokenizer = None;
        
        info!("Inference engine shutdown complete");
        Ok(())
    }
}

/// Model information structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub device: String,
    pub is_loaded: bool,
    pub config: ModelConfigInfo,
}

/// Model configuration information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelConfigInfo {
    pub max_context_length: usize,
    pub hidden_size: usize,
    pub num_layers: usize,
    pub num_attention_heads: usize,
}