//! AI Inference Benchmark
//! 
//! This benchmark tests the local AI inference performance with the goal of
//! achieving <1s response time consistently.
//!
//! Usage: cargo run --release --example benchmark "What is Stoicism?"

use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{info, warn, error};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};

use codex_core::{
    ai::AiEngine,
    config::AiConfig,
    CodexResult, CodexError,
};

/// Performance metrics collected during benchmarking
#[derive(Debug)]
struct BenchmarkMetrics {
    prompt: String,
    response: String,
    inference_time: Duration,
    memory_before_mb: u64,
    memory_after_mb: u64,
    memory_peak_mb: u64,
    cpu_usage_percent: f32,
    tokens_generated: usize,
    tokens_per_second: f64,
    cache_hit: bool,
}

impl BenchmarkMetrics {
    fn print_report(&self) {
        println!("\n🚀 BENCHMARK RESULTS");
        println!("===================");
        println!("Prompt: {}", self.prompt);
        println!("Response Length: {} characters", self.response.len());
        println!("Tokens Generated: {}", self.tokens_generated);
        println!();
        
        println!("⏱️  PERFORMANCE METRICS");
        println!("Inference Time: {:.3}s", self.inference_time.as_secs_f64());
        println!("Tokens/Second: {:.1}", self.tokens_per_second);
        println!("Target (<1s): {}", if self.inference_time.as_secs_f64() < 1.0 { "✅ PASS" } else { "❌ FAIL" });
        println!();
        
        println!("💾 MEMORY USAGE");
        println!("Before: {} MB", self.memory_before_mb);
        println!("After: {} MB", self.memory_after_mb);
        println!("Peak: {} MB", self.memory_peak_mb);
        println!("Delta: +{} MB", self.memory_after_mb.saturating_sub(self.memory_before_mb));
        println!();
        
        println!("🖥️  SYSTEM METRICS");
        println!("CPU Usage: {:.1}%", self.cpu_usage_percent);
        println!("Cache Hit: {}", if self.cache_hit { "✅ Yes" } else { "❌ No" });
        println!();
        
        println!("📝 RESPONSE PREVIEW");
        println!("{}", self.response.chars().take(200).collect::<String>());
        if self.response.len() > 200 {
            println!("... (truncated)");
        }
    }

    fn is_performance_target_met(&self) -> bool {
        self.inference_time.as_secs_f64() < 1.0
    }
}

/// System resource monitor for tracking CPU and memory during inference
struct ResourceMonitor {
    system: System,
    initial_memory: u64,
    peak_memory: u64,
    cpu_usage: f32,
}

impl ResourceMonitor {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        let initial_memory = system.used_memory() / 1024 / 1024; // Convert to MB
        
        Self {
            system,
            initial_memory,
            peak_memory: initial_memory,
            cpu_usage: 0.0,
        }
    }
    
    fn update(&mut self) {
        self.system.refresh_all();
        
        let current_memory = self.system.used_memory() / 1024 / 1024;
        if current_memory > self.peak_memory {
            self.peak_memory = current_memory;
        }
        
        // Calculate average CPU usage
        self.cpu_usage = self.system.cpus().iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>() / self.system.cpus().len() as f32;
    }
    
    fn get_current_memory_mb(&self) -> u64 {
        self.system.used_memory() / 1024 / 1024
    }
    
    fn get_peak_memory_mb(&self) -> u64 {
        self.peak_memory
    }
    
    fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage
    }
}

#[tokio::main]
async fn main() -> CodexResult<()> {
    // Initialize enhanced logging with span tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("codex_core=debug".parse().unwrap()))
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("🚀 Starting AI inference benchmark");

    // Get prompt from command line args or use default
    let args: Vec<String> = env::args().collect();
    let prompt = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        "What is Stoicism?".to_string()
    };

    info!("Testing prompt: '{}'", prompt);

    // Initialize resource monitoring
    let mut monitor = ResourceMonitor::new();
    info!("Initial system memory: {} MB", monitor.get_current_memory_mb());

    // Create AI configuration
    let config = create_optimized_config().await?;
    
    // Initialize AI engine
    info!("Initializing AI engine...");
    let engine = AiEngine::new(&config).await?;
    
    // Perform health check
    let is_healthy = engine.health_check().await?;
    if !is_healthy {
        return Err(CodexError::ai_inference("AI engine health check failed"));
    }
    info!("✅ AI engine health check passed");

    // Record pre-inference metrics
    let memory_before = monitor.get_current_memory_mb();
    
    // Warm up with a simple query (won't count towards benchmark)
    info!("Warming up inference engine...");
    let _warmup = engine.infer("Hello").await?;
    info!("✅ Warmup completed");

    monitor.update();

    // Run the main benchmark
    info!("🏃 Starting benchmark inference...");
    let start_time = Instant::now();
    
    // Use timeout to ensure we don't wait forever
    let response = match timeout(Duration::from_secs(5), engine.infer(&prompt)).await {
        Ok(result) => result?,
        Err(_) => {
            error!("❌ Inference timeout after 5 seconds");
            return Err(CodexError::ai_inference("Inference timeout"));
        }
    };
    
    let inference_time = start_time.elapsed();
    
    // Update monitoring after inference
    monitor.update();
    let memory_after = monitor.get_current_memory_mb();

    // Estimate token count (rough approximation: 1 token ≈ 4 characters)
    let estimated_tokens = response.len() / 4;
    let tokens_per_second = if inference_time.as_secs_f64() > 0.0 {
        estimated_tokens as f64 / inference_time.as_secs_f64()
    } else {
        0.0
    };

    // Check if this was likely a cache hit (very fast response)
    let cache_hit = inference_time.as_millis() < 100;

    // Create metrics report
    let metrics = BenchmarkMetrics {
        prompt,
        response,
        inference_time,
        memory_before_mb: memory_before,
        memory_after_mb: memory_after,
        memory_peak_mb: monitor.get_peak_memory_mb(),
        cpu_usage_percent: monitor.get_cpu_usage(),
        tokens_generated: estimated_tokens,
        tokens_per_second,
        cache_hit,
    };

    // Print detailed benchmark report
    metrics.print_report();

    // Run additional performance tests if requested
    if env::var("EXTENDED_BENCHMARK").is_ok() {
        info!("\n🔄 Running extended performance tests...");
        run_extended_benchmarks(&engine, &mut monitor).await?;
    }

    // Final assessment
    if metrics.is_performance_target_met() {
        info!("🎉 BENCHMARK PASSED: Target <1s response time achieved!");
        std::process::exit(0);
    } else {
        warn!("⚠️  BENCHMARK WARNING: Target <1s response time not achieved");
        std::process::exit(1);
    }
}

async fn create_optimized_config() -> CodexResult<AiConfig> {
    let models_dir = PathBuf::from("../models");
    let primary_model = models_dir.join("test-llama-7b.gguf").to_string_lossy().to_string();
    
    // Verify model file exists
    if !PathBuf::from(&primary_model).exists() {
        return Err(CodexError::not_found(
            format!("Model file not found: {}. Please run the model download first.", primary_model)
        ));
    }

    let config = AiConfig {
        models_dir,
        primary_model,
        device: "cpu".to_string(), // Use CPU for consistent benchmarking
        max_tokens: 256,            // Limit for faster response
        temperature: 0.7,
        top_p: 0.9,
        enable_caching: true,
        cache_size: 1000,           // Cache up to 1000 entries
        context_window: 4096,
        batch_size: 1,              // Single batch for latency optimization
        embedding_model: None,
    };

    info!("Created optimized config: device={}, max_tokens={}, caching={}",
          config.device, config.max_tokens, config.enable_caching);

    Ok(config)
}

async fn run_extended_benchmarks(engine: &AiEngine, monitor: &mut ResourceMonitor) -> CodexResult<()> {
    let test_prompts = vec![
        "Explain quantum computing briefly",
        "What is machine learning?",
        "How does photosynthesis work?",
        "Define artificial intelligence",
        "What is the theory of relativity?",
    ];

    let mut total_time = Duration::ZERO;
    let mut successful_tests = 0;

    for (i, prompt) in test_prompts.iter().enumerate() {
        info!("Extended test {}/{}: '{}'", i + 1, test_prompts.len(), prompt);
        
        let start = Instant::now();
        match engine.infer(prompt).await {
            Ok(response) => {
                let elapsed = start.elapsed();
                total_time += elapsed;
                successful_tests += 1;
                
                info!("✅ Test {} completed in {:.3}s ({} chars)", 
                      i + 1, elapsed.as_secs_f64(), response.len());
            }
            Err(e) => {
                error!("❌ Test {} failed: {}", i + 1, e);
            }
        }
        
        monitor.update();
        
        // Small delay between tests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    if successful_tests > 0 {
        let avg_time = total_time / successful_tests as u32;
        info!("\n📊 EXTENDED BENCHMARK SUMMARY");
        info!("Successful tests: {}/{}", successful_tests, test_prompts.len());
        info!("Average response time: {:.3}s", avg_time.as_secs_f64());
        info!("Peak memory usage: {} MB", monitor.get_peak_memory_mb());
        info!("Final CPU usage: {:.1}%", monitor.get_cpu_usage());
    }

    Ok(())
}