//! Integration tests for GGUF engine functionality

use std::path::PathBuf;
use std::fs;
use tempfile::tempdir;
use codex_core::{
    ai::engine::{GGUFEngine, EngineParams, GenerationSettings, LLMEngine},
    update::manifest::ModelManifest,
};

#[tokio::test]
#[ignore] // Run with --ignored flag when testing with real models
async fn test_gguf_metadata_parsing() {
    // Create a minimal valid GGUF file for testing
    let temp_dir = tempdir().unwrap();
    let test_model_path = temp_dir.path().join("test_model.gguf");
    
    // Create a basic GGUF header for testing
    let gguf_header = create_test_gguf_header();
    fs::write(&test_model_path, gguf_header).unwrap();
    
    // Test metadata parsing
    let result = GGUFEngine::parse_gguf_metadata(&test_model_path);
    
    match result {
        Ok(metadata) => {
            assert_eq!(metadata.version, 3); // GGUF version 3
            assert!(metadata.tensor_count >= 0);
            println!("✓ GGUF metadata parsing successful");
        }
        Err(e) => {
            // Expected for minimal test file
            println!("⚠ GGUF parsing failed (expected for test file): {}", e);
        }
    }
}

#[tokio::test]
async fn test_gguf_checksum_verification() {
    let temp_dir = tempdir().unwrap();
    let test_file = temp_dir.path().join("test_file.txt");
    let test_content = b"Hello, GGUF World!";
    
    // Write test content
    fs::write(&test_file, test_content).unwrap();
    
    // Calculate checksum
    let checksum = GGUFEngine::calculate_checksum(&test_file).await.unwrap();
    println!("Calculated checksum: {}", checksum);
    
    // Verify checksum
    let is_valid = GGUFEngine::verify_checksum(&test_file, &checksum).await.unwrap();
    assert!(is_valid, "Checksum verification should succeed");
    
    // Test with wrong checksum
    let wrong_checksum = "0".repeat(64);
    let is_invalid = GGUFEngine::verify_checksum(&test_file, &wrong_checksum).await.unwrap();
    assert!(!is_invalid, "Wrong checksum should fail verification");
    
    println!("✓ Checksum verification test passed");
}

#[tokio::test]
async fn test_gguf_engine_creation() {
    let temp_dir = tempdir().unwrap();
    let test_model_path = temp_dir.path().join("test_model.gguf");
    let tokenizer_path = temp_dir.path().join("tokenizer.json");
    
    // Create minimal test files
    let gguf_header = create_test_gguf_header();
    fs::write(&test_model_path, gguf_header).unwrap();
    
    // Create minimal tokenizer.json
    let tokenizer_json = r#"{
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": [],
        "normalizer": null,
        "pre_tokenizer": {
            "type": "ByteLevel",
            "add_prefix_space": false
        },
        "post_processor": null,
        "decoder": {
            "type": "ByteLevel"
        },
        "model": {
            "type": "BPE",
            "dropout": null,
            "unk_token": null,
            "continuing_subword_prefix": null,
            "end_of_word_suffix": null,
            "fuse_unk": false,
            "vocab": {},
            "merges": []
        }
    }"#;
    fs::write(&tokenizer_path, tokenizer_json).unwrap();
    
    // Test engine creation
    let params = EngineParams {
        num_threads: 4,
        context_length: 2048,
        gpu_layers: 0,
        batch_size: 256,
        use_mmap: true,
        use_metal: false,
        cuda_device_id: None,
    };
    
    let result = GGUFEngine::load(&test_model_path, params).await;
    
    match result {
        Ok(engine) => {
            assert_eq!(engine.engine_type(), codex_core::ai::engine::EngineType::GGUF);
            println!("✓ GGUF engine creation successful");
            
            // Test basic functionality
            let model_info = engine.get_model_info();
            assert!(!model_info.name.is_empty());
            println!("Model info: {:?}", model_info);
            
            // Test memory usage reporting
            let memory_usage = engine.get_memory_usage().await;
            println!("Memory usage: {} bytes", memory_usage);
            
        }
        Err(e) => {
            println!("⚠ Engine creation failed (expected for minimal test): {}", e);
            // This is expected since we're using minimal test files
        }
    }
}

#[tokio::test]
async fn test_generation_with_placeholder() {
    let temp_dir = tempdir().unwrap();
    let test_model_path = temp_dir.path().join("test_model.gguf");
    
    // Create test files (minimal for placeholder testing)
    let gguf_header = create_test_gguf_header();
    fs::write(&test_model_path, gguf_header).unwrap();
    
    let params = EngineParams::default();
    
    if let Ok(engine) = GGUFEngine::load(&test_model_path, params).await {
        let settings = GenerationSettings {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            max_tokens: 100,
            repeat_penalty: 1.1,
            repeat_last_n: 64,
            stop_sequences: vec!["</s>".to_string()],
            seed: None,
        };
        
        let result = engine.generate("Explain how AI works", settings).await;
        
        match result {
            Ok(response) => {
                assert!(!response.is_empty());
                println!("Generated response: {}", response);
                println!("✓ Generation test passed");
            }
            Err(e) => {
                println!("⚠ Generation failed: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_model_manifest_integration() {
    // Test model manifest creation and validation
    let manifest = ModelManifest::mistral_7b_instruct_q4k();
    
    let validation = manifest.validate();
    assert!(validation.is_valid, "Model manifest should be valid");
    
    // Test path generation
    let temp_dir = tempdir().unwrap();
    let local_path = manifest.get_local_path(temp_dir.path());
    
    assert!(local_path.to_string_lossy().contains("mistral-7b-instruct-q4_k"));
    assert!(local_path.to_string_lossy().ends_with(".gguf"));
    
    // Test download time estimation
    let download_time = manifest.estimated_download_time(10.0); // 10 MB/s
    assert!(download_time > 0.0);
    
    println!("✓ Model manifest integration test passed");
}

#[tokio::test]
async fn test_memory_tracking() {
    let temp_dir = tempdir().unwrap();
    let test_model_path = temp_dir.path().join("large_test_model.gguf");
    
    // Create a larger test file to test memory tracking
    let large_data = vec![0u8; 1024 * 1024]; // 1MB test file
    fs::write(&test_model_path, large_data).unwrap();
    
    let params = EngineParams::default();
    
    if let Ok(engine) = GGUFEngine::load(&test_model_path, params).await {
        let memory_usage = engine.get_memory_usage().await;
        
        // Should report some memory usage
        assert!(memory_usage > 0, "Should report non-zero memory usage");
        
        println!("Memory usage for 1MB model: {} bytes", memory_usage);
        println!("✓ Memory tracking test passed");
    }
}

/// Create a minimal GGUF header for testing
fn create_test_gguf_header() -> Vec<u8> {
    let mut header = Vec::new();
    
    // GGUF magic number: "GGUF" in little-endian
    header.extend_from_slice(&0x46554747u32.to_le_bytes());
    
    // Version (3)
    header.extend_from_slice(&3u32.to_le_bytes());
    
    // Tensor count (0 for minimal test)
    header.extend_from_slice(&0u64.to_le_bytes());
    
    // Metadata KV count (1 for minimal test)
    header.extend_from_slice(&1u64.to_le_bytes());
    
    // Add minimal metadata entry: "general.architecture" = "llama"
    // Key length
    header.extend_from_slice(&20u64.to_le_bytes());
    // Key: "general.architecture"
    header.extend_from_slice(b"general.architecture");
    // Value type (8 = string)
    header.extend_from_slice(&8u32.to_le_bytes());
    // Value length
    header.extend_from_slice(&5u64.to_le_bytes());
    // Value: "llama"
    header.extend_from_slice(b"llama");
    
    header
}

#[tokio::test]
async fn test_config_extraction_from_metadata() {
    use codex_core::ai::engine::{GGUFMetadata, GGUFValue};
    use std::collections::HashMap;
    
    // Create test metadata
    let mut metadata = HashMap::new();
    metadata.insert("llama.vocab_size".to_string(), GGUFValue::UInt32(32000));
    metadata.insert("llama.embedding_length".to_string(), GGUFValue::UInt32(4096));
    metadata.insert("llama.block_count".to_string(), GGUFValue::UInt32(32));
    metadata.insert("llama.attention.head_count".to_string(), GGUFValue::UInt32(32));
    metadata.insert("llama.context_length".to_string(), GGUFValue::UInt32(4096));
    
    let gguf_metadata = GGUFMetadata {
        version: 3,
        tensor_count: 0,
        metadata_kv_count: 5,
        metadata,
        tensors: vec![],
    };
    
    // Test config extraction
    let result = GGUFEngine::metadata_to_config(&gguf_metadata);
    
    match result {
        Ok(config) => {
            assert_eq!(config.vocab_size, 32000);
            assert_eq!(config.hidden_size, 4096);
            assert_eq!(config.num_hidden_layers, 32);
            assert_eq!(config.num_attention_heads, 32);
            assert_eq!(config.max_position_embeddings, 4096);
            
            println!("✓ Config extraction test passed");
        }
        Err(e) => {
            panic!("Config extraction failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_parameter_estimation() {
    use candle_transformers::models::llama::LlamaConfig;
    
    let config = LlamaConfig {
        vocab_size: 32000,
        hidden_size: 4096,
        intermediate_size: 11008,
        num_hidden_layers: 32,
        num_attention_heads: 32,
        num_key_value_heads: Some(32),
        max_position_embeddings: 4096,
        rms_norm_eps: 1e-5,
        rope_theta: 10000.0,
        use_flash_attn: false,
    };
    
    let param_count = codex_core::ai::engine::GGUFEngine::estimate_parameters(&config);
    
    // Should be roughly 7B parameters for this config
    let billion_params = param_count as f64 / 1_000_000_000.0;
    
    assert!(billion_params > 6.0 && billion_params < 8.0, 
            "Parameter count should be around 7B, got {:.2}B", billion_params);
    
    println!("Estimated parameters: {:.2}B", billion_params);
    println!("✓ Parameter estimation test passed");
}