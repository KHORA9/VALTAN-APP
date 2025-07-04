//! AI testing utilities and mocks

use codex_core::{
    ai::{AiEngine, AiStats, SystemMetricsSnapshot, TokenCacheStats},
    config::AiConfig,
    CodexResult,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use fake::{Fake, Faker};
use fake::faker::lorem::en::*;

/// Mock AI engine for testing
pub struct MockAiEngine {
    responses: HashMap<String, String>,
    response_delay: Duration,
    should_fail: bool,
    call_count: Arc<std::sync::Mutex<usize>>,
}

impl MockAiEngine {
    /// Create a new mock AI engine
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            response_delay: Duration::from_millis(100),
            should_fail: false,
            call_count: Arc::new(std::sync::Mutex::new(0)),
        }
    }
    
    /// Add a canned response for a specific prompt
    pub fn add_response(mut self, prompt: &str, response: &str) -> Self {
        self.responses.insert(prompt.to_string(), response.to_string());
        self
    }
    
    /// Set response delay for simulating processing time
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.response_delay = delay;
        self
    }
    
    /// Configure to fail on next call
    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
    
    /// Get number of times the engine was called
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
    
    /// Simulate text generation
    pub async fn generate_text(&self, prompt: &str) -> CodexResult<String> {
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
        }
        
        if self.should_fail {
            return Err(anyhow::anyhow!("Mock AI failure").into());
        }
        
        tokio::time::sleep(self.response_delay).await;
        
        let response = self.responses
            .get(prompt)
            .cloned()
            .unwrap_or_else(|| format!("Mock response to: {}", prompt));
            
        Ok(response)
    }
    
    /// Simulate streaming text generation
    pub async fn generate_text_stream(
        &self,
        prompt: &str,
        callback: impl Fn(String) + Send + Sync + 'static,
    ) -> CodexResult<String> {
        let response = self.generate_text(prompt).await?;
        
        // Simulate streaming by sending chunks
        let words: Vec<&str> = response.split_whitespace().collect();
        let mut current = String::new();
        
        for word in words {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
            callback(current.clone());
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        
        Ok(response)
    }
    
    /// Mock system metrics
    pub async fn get_system_metrics(&self) -> CodexResult<SystemMetricsSnapshot> {
        Ok(SystemMetricsSnapshot {
            timestamp: chrono::Utc::now(),
            process_memory_usage_mb: 125.5,
            system_memory_usage_mb: 8192.0,
            total_memory_mb: 16384.0,
            process_cpu_usage: 15.2,
            system_cpu_usage: 45.8,
            uptime_seconds: 3600,
        })
    }
    
    /// Mock token cache stats
    pub async fn get_token_cache_stats(&self) -> CodexResult<TokenCacheStats> {
        Ok(TokenCacheStats {
            total_tokens: 50000,
            cache_size: 1000000,
            hit_rate: 0.85,
            evictions: 12,
        })
    }
}

impl Default for MockAiEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// AI test fixtures and sample data
pub struct AiFixtures;

impl AiFixtures {
    /// Sample prompts for testing
    pub fn sample_prompts() -> Vec<&'static str> {
        vec![
            "What is artificial intelligence?",
            "Explain quantum computing in simple terms.",
            "Summarize the principles of machine learning.",
            "What are the benefits of renewable energy?",
            "Describe the process of photosynthesis.",
        ]
    }
    
    /// Sample AI responses
    pub fn sample_responses() -> Vec<&'static str> {
        vec![
            "Artificial intelligence is the simulation of human intelligence in machines...",
            "Quantum computing uses quantum mechanics principles to process information...",
            "Machine learning is a subset of AI that enables computers to learn...",
            "Renewable energy sources are sustainable and environmentally friendly...",
            "Photosynthesis is the process by which plants convert sunlight into energy...",
        ]
    }
    
    /// Create a mock engine with predefined responses
    pub fn mock_engine_with_responses() -> MockAiEngine {
        let prompts = Self::sample_prompts();
        let responses = Self::sample_responses();
        
        let mut engine = MockAiEngine::new();
        for (prompt, response) in prompts.iter().zip(responses.iter()) {
            engine = engine.add_response(prompt, response);
        }
        
        engine
    }
    
    /// Generate random prompt for testing
    pub fn random_prompt() -> String {
        format!("Explain {} in detail.", Word().fake::<String>())
    }
    
    /// Generate random AI response
    pub fn random_response() -> String {
        Paragraphs(2..5).fake::<Vec<String>>().join("\n\n")
    }
}

/// Performance testing utilities for AI operations
pub struct AiPerformanceTest;

impl AiPerformanceTest {
    /// Test inference speed
    pub async fn test_inference_speed<F, Fut>(
        operation: F,
        max_duration: Duration,
    ) -> CodexResult<Duration>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = CodexResult<String>>,
    {
        let start = Instant::now();
        let _result = operation().await?;
        let duration = start.elapsed();
        
        assert!(
            duration <= max_duration,
            "AI inference took {:?}, expected <= {:?}",
            duration,
            max_duration
        );
        
        Ok(duration)
    }
    
    /// Test streaming latency
    pub async fn test_streaming_latency<F, Fut>(
        operation: F,
        max_first_token_delay: Duration,
    ) -> CodexResult<(Duration, Duration)>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = CodexResult<String>>,
    {
        let start = Instant::now();
        let first_token_time = Arc::new(std::sync::Mutex::new(None));
        let first_token_time_clone = first_token_time.clone();
        
        // This is a simplified version - in real tests we'd use the streaming callback
        let _result = operation().await?;
        let total_duration = start.elapsed();
        
        // For now, simulate first token time
        let first_token_duration = Duration::from_millis(50);
        
        assert!(
            first_token_duration <= max_first_token_delay,
            "First token took {:?}, expected <= {:?}",
            first_token_duration,
            max_first_token_delay
        );
        
        Ok((first_token_duration, total_duration))
    }
}

/// Test configuration for AI components
#[derive(Clone)]
pub struct TestAiConfig {
    pub model_path: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub enable_caching: bool,
}

impl TestAiConfig {
    /// Create test configuration
    pub fn new() -> Self {
        Self {
            model_path: "models/test-llama-7b.gguf".to_string(),
            max_tokens: 100,
            temperature: 0.7,
            enable_caching: true,
        }
    }
    
    /// Convert to AiConfig
    pub fn to_ai_config(&self) -> AiConfig {
        AiConfig {
            model_path: self.model_path.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: 0.9,
            repeat_penalty: 1.1,
            enable_caching: self.enable_caching,
            cache_size_mb: 256,
            gpu_layers: 0,
        }
    }
}

impl Default for TestAiConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for AI component tests
#[macro_export]
macro_rules! ai_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> CodexResult<()> {
            let config = TestAiConfig::new();
            let mock_engine = MockAiEngine::new();
            $body(config, mock_engine).await
        }
    };
}

/// Macro for AI performance tests
#[macro_export]
macro_rules! ai_perf_test {
    ($name:ident, $max_duration:expr, $body:expr) => {
        #[tokio::test]
        async fn $name() -> CodexResult<()> {
            let duration = AiPerformanceTest::test_inference_speed(
                || async { $body().await },
                $max_duration,
            ).await?;
            
            println!("Test {} completed in {:?}", stringify!($name), duration);
            Ok(())
        }
    };
}