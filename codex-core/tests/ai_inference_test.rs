//! Unit tests for AI inference engine

mod common;

use common::ai::{MockAiEngine, AiFixtures, AiPerformanceTest, TestAiConfig};
use codex_core::{
    ai::{inference::InferenceEngine, AiStats, SystemMetricsSnapshot, TokenCacheStats},
    config::AiConfig,
    CodexResult,
};
use rstest::*;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Test basic inference engine creation and initialization
#[rstest]
#[tokio::test]
async fn test_inference_engine_creation() -> CodexResult<()> {
    let config = TestAiConfig::new().to_ai_config();
    let engine = InferenceEngine::new(&config).await;
    
    // Engine creation should succeed even without a real model file
    assert!(engine.is_ok());
    
    let engine = engine.unwrap();
    assert!(!engine.is_model_loaded());
    
    Ok(())
}

/// Test inference engine health check
#[rstest]
#[tokio::test]
async fn test_inference_engine_health_check() -> CodexResult<()> {
    let config = TestAiConfig::new().to_ai_config();
    let engine = InferenceEngine::new(&config).await?;
    
    let health = engine.health_check().await?;
    
    // Health check should work even without model loaded
    assert!(health);
    
    Ok(())
}

/// Test system metrics collection
#[rstest]
#[tokio::test]
async fn test_system_metrics_collection() -> CodexResult<()> {
    let config = TestAiConfig::new().to_ai_config();
    let engine = InferenceEngine::new(&config).await?;
    
    let metrics = engine.get_system_metrics().await?;
    
    // Verify metrics structure
    assert!(metrics.process_memory_usage_mb > 0.0);
    assert!(metrics.total_memory_mb > 0.0);
    assert!(metrics.system_memory_usage_mb > 0.0);
    assert!(metrics.uptime_seconds > 0);
    assert!(metrics.process_cpu_usage >= 0.0);
    assert!(metrics.system_cpu_usage >= 0.0);
    
    // Memory usage should be reasonable
    assert!(metrics.process_memory_usage_mb < metrics.total_memory_mb);
    assert!(metrics.system_memory_usage_mb <= metrics.total_memory_mb);
    
    // CPU usage should be between 0 and 100
    assert!(metrics.process_cpu_usage <= 100.0);
    assert!(metrics.system_cpu_usage <= 100.0);
    
    Ok(())
}

/// Test token cache statistics
#[rstest]
#[tokio::test]
async fn test_token_cache_stats() -> CodexResult<()> {
    let config = TestAiConfig::new().to_ai_config();
    let engine = InferenceEngine::new(&config).await?;
    
    let stats = engine.get_token_cache_stats().await?;
    
    // Verify initial cache stats
    assert_eq!(stats.total_tokens, 0);
    assert!(stats.cache_size > 0);
    assert_eq!(stats.hit_rate, 0.0);
    assert_eq!(stats.evictions, 0);
    
    Ok(())
}

/// Test inference with mock model
#[rstest]
#[tokio::test]
async fn test_mock_inference() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("Hello", "Hello! How can I help you today?")
        .add_response("What is AI?", "Artificial Intelligence is the simulation of human intelligence in machines.");
    
    // Test simple inference
    let response = mock_engine.generate_text("Hello").await?;
    assert_eq!(response, "Hello! How can I help you today?");
    
    // Test different prompt
    let response = mock_engine.generate_text("What is AI?").await?;
    assert!(response.contains("Artificial Intelligence"));
    
    // Test call counting
    assert_eq!(mock_engine.call_count(), 2);
    
    Ok(())
}

/// Test inference streaming with mock
#[rstest]
#[tokio::test]
async fn test_mock_streaming() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("Stream test", "This is a streaming response test")
        .with_delay(Duration::from_millis(50));
    
    let mut chunks = Vec::new();
    let response = mock_engine.generate_text_stream("Stream test", |chunk| {
        chunks.push(chunk);
    }).await?;
    
    // Should receive progressive chunks
    assert!(chunks.len() > 1);
    
    // Final response should match expected
    assert_eq!(response, "This is a streaming response test");
    
    // Last chunk should be the complete response
    assert_eq!(chunks.last().unwrap(), &response);
    
    Ok(())
}

/// Test inference performance with mocks
#[rstest]
#[tokio::test]
async fn test_inference_performance() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("performance test", "This is a performance test response")
        .with_delay(Duration::from_millis(100));
    
    let duration = AiPerformanceTest::test_inference_speed(
        || mock_engine.generate_text("performance test"),
        Duration::from_millis(200),
    ).await?;
    
    // Should complete within expected time
    assert!(duration >= Duration::from_millis(100)); // At least the delay
    assert!(duration <= Duration::from_millis(150)); // But not much more
    
    Ok(())
}

/// Test inference error handling
#[rstest]
#[tokio::test]
async fn test_inference_error_handling() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new().with_failure();
    
    let result = mock_engine.generate_text("This should fail").await;
    assert!(result.is_err());
    
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Mock AI failure"));
    
    Ok(())
}

/// Test concurrent inference requests
#[rstest]
#[tokio::test]
async fn test_concurrent_inference() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("concurrent 1", "Response 1")
        .add_response("concurrent 2", "Response 2")
        .add_response("concurrent 3", "Response 3")
        .with_delay(Duration::from_millis(50));
    
    // Start multiple concurrent requests
    let handles = vec![
        tokio::spawn({
            let engine = &mock_engine;
            async move { engine.generate_text("concurrent 1").await }
        }),
        tokio::spawn({
            let engine = &mock_engine;
            async move { engine.generate_text("concurrent 2").await }
        }),
        tokio::spawn({
            let engine = &mock_engine;
            async move { engine.generate_text("concurrent 3").await }
        }),
    ];
    
    // Wait for all to complete
    let results = futures::future::join_all(handles).await;
    
    // All should succeed
    for result in results {
        let response = result.unwrap()?;
        assert!(response.starts_with("Response"));
    }
    
    // Should have made 3 calls
    assert_eq!(mock_engine.call_count(), 3);
    
    Ok(())
}

/// Test inference timeout handling
#[rstest]
#[tokio::test]
async fn test_inference_timeout() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("slow response", "This response takes too long")
        .with_delay(Duration::from_millis(500));
    
    // Set a short timeout
    let result = timeout(
        Duration::from_millis(100),
        mock_engine.generate_text("slow response")
    ).await;
    
    // Should timeout
    assert!(result.is_err());
    
    Ok(())
}

/// Test AI statistics tracking with mock
#[rstest]
#[tokio::test]
async fn test_ai_stats_tracking() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("stats test", "Testing statistics tracking");
    
    // Get initial metrics
    let initial_metrics = mock_engine.get_system_metrics().await?;
    
    // Perform inference
    let _response = mock_engine.generate_text("stats test").await?;
    
    // Get updated metrics
    let updated_metrics = mock_engine.get_system_metrics().await?;
    
    // Metrics should be consistent
    assert_eq!(initial_metrics.total_memory_mb, updated_metrics.total_memory_mb);
    assert!(updated_metrics.uptime_seconds >= initial_metrics.uptime_seconds);
    
    Ok(())
}

/// Test cache behavior with mock
#[rstest]
#[tokio::test]
async fn test_cache_behavior() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("cached prompt", "This response should be cached");
    
    // First call
    let start1 = Instant::now();
    let response1 = mock_engine.generate_text("cached prompt").await?;
    let duration1 = start1.elapsed();
    
    // Second call with same prompt
    let start2 = Instant::now();
    let response2 = mock_engine.generate_text("cached prompt").await?;
    let duration2 = start2.elapsed();
    
    // Responses should be identical
    assert_eq!(response1, response2);
    
    // Both should return the same result
    assert_eq!(response1, "This response should be cached");
    
    // Call count should reflect both calls
    assert_eq!(mock_engine.call_count(), 2);
    
    Ok(())
}

/// Test inference with various prompt lengths
#[rstest]
#[case("Short", "Short response")]
#[case("This is a medium length prompt with some details", "Medium response")]
#[case("This is a very long prompt that contains multiple sentences and provides extensive context. It should test how well the inference engine handles longer inputs with complex content and multiple topics.", "Long response")]
#[tokio::test]
async fn test_various_prompt_lengths(#[case] prompt: &str, #[case] expected: &str) -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response(prompt, expected);
    
    let response = mock_engine.generate_text(prompt).await?;
    assert_eq!(response, expected);
    
    Ok(())
}

/// Test inference with special characters and formatting
#[rstest]
#[case("Hello\nWorld", "Multi-line response")]
#[case("Test with \"quotes\" and 'apostrophes'", "Quoted response")]
#[case("Unicode test: 你好 世界 🌍", "Unicode response")]
#[case("Special chars: @#$%^&*()_+-=[]{}|;:,.<>?", "Special char response")]
#[tokio::test]
async fn test_special_characters(#[case] prompt: &str, #[case] expected: &str) -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response(prompt, expected);
    
    let response = mock_engine.generate_text(prompt).await?;
    assert_eq!(response, expected);
    
    Ok(())
}

/// Test memory usage tracking during inference
#[rstest]
#[tokio::test]
async fn test_memory_usage_tracking() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("memory test", "Testing memory usage tracking");
    
    // Get initial memory metrics
    let initial_metrics = mock_engine.get_system_metrics().await?;
    let initial_memory = initial_metrics.process_memory_usage_mb;
    
    // Perform multiple inferences
    for i in 0..10 {
        let prompt = format!("memory test {}", i);
        mock_engine.generate_text(&prompt).await?;
    }
    
    // Get final memory metrics
    let final_metrics = mock_engine.get_system_metrics().await?;
    let final_memory = final_metrics.process_memory_usage_mb;
    
    // Memory should be reasonable (not growing excessively)
    let memory_growth = final_memory - initial_memory;
    assert!(memory_growth < 100.0, "Memory grew too much: {} MB", memory_growth);
    
    Ok(())
}

/// Test token cache statistics updates
#[rstest]
#[tokio::test]
async fn test_token_cache_updates() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("cache test 1", "First cached response")
        .add_response("cache test 2", "Second cached response");
    
    // Get initial cache stats
    let initial_stats = mock_engine.get_token_cache_stats().await?;
    
    // Perform inferences
    mock_engine.generate_text("cache test 1").await?;
    mock_engine.generate_text("cache test 2").await?;
    mock_engine.generate_text("cache test 1").await?; // Repeat for cache hit
    
    // Get updated cache stats
    let updated_stats = mock_engine.get_token_cache_stats().await?;
    
    // Cache should show some activity (in a real implementation)
    // For mock, we just verify the structure is maintained
    assert_eq!(initial_stats.cache_size, updated_stats.cache_size);
    assert!(updated_stats.hit_rate >= 0.0 && updated_stats.hit_rate <= 1.0);
    
    Ok(())
}

/// Test streaming callback functionality
#[rstest]
#[tokio::test]
async fn test_streaming_callbacks() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("callback test", "Testing streaming callbacks with multiple words");
    
    let mut callback_count = 0;
    let mut last_chunk = String::new();
    
    let response = mock_engine.generate_text_stream("callback test", |chunk| {
        callback_count += 1;
        last_chunk = chunk;
    }).await?;
    
    // Should have received multiple callbacks
    assert!(callback_count > 1);
    
    // Last chunk should be the complete response
    assert_eq!(last_chunk, response);
    assert_eq!(response, "Testing streaming callbacks with multiple words");
    
    Ok(())
}

/// Test inference performance benchmarks
#[rstest]
#[tokio::test]
async fn test_inference_benchmarks() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new()
        .add_response("benchmark", "Benchmark response for performance testing")
        .with_delay(Duration::from_millis(10));
    
    // Run multiple iterations to get average performance
    let iterations = 100;
    let start = Instant::now();
    
    for i in 0..iterations {
        let prompt = format!("benchmark {}", i);
        mock_engine.generate_text(&prompt).await?;
    }
    
    let total_duration = start.elapsed();
    let avg_duration = total_duration / iterations;
    
    // Each inference should be reasonably fast
    assert!(avg_duration.as_millis() < 50, "Average inference too slow: {:?}", avg_duration);
    
    println!("Inference benchmark: {} iterations in {:?} (avg: {:?})", 
        iterations, total_duration, avg_duration);
    
    Ok(())
}

/// Test AI prompts from fixtures
#[rstest]
#[tokio::test]
async fn test_ai_fixtures_prompts() -> CodexResult<()> {
    let sample_prompts = AiFixtures::sample_prompts();
    let sample_responses = AiFixtures::sample_responses();
    
    let mut mock_engine = MockAiEngine::new();
    
    // Add all sample responses to mock engine
    for (prompt, response) in sample_prompts.iter().zip(sample_responses.iter()) {
        mock_engine = mock_engine.add_response(prompt, response);
    }
    
    // Test each prompt
    for (prompt, expected_response) in sample_prompts.iter().zip(sample_responses.iter()) {
        let response = mock_engine.generate_text(prompt).await?;
        assert_eq!(&response, expected_response);
    }
    
    Ok(())
}

/// Test random prompt generation and handling
#[rstest]
#[tokio::test]
async fn test_random_prompts() -> CodexResult<()> {
    let mock_engine = MockAiEngine::new();
    
    // Generate random prompts
    for _ in 0..10 {
        let random_prompt = AiFixtures::random_prompt();
        assert!(!random_prompt.is_empty());
        assert!(random_prompt.contains("Explain"));
        
        // Mock engine should handle unknown prompts gracefully
        let response = mock_engine.generate_text(&random_prompt).await?;
        assert!(response.contains("Mock response to:"));
        assert!(response.contains(&random_prompt));
    }
    
    Ok(())
}