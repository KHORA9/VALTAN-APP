//! Common test utilities and fixtures for codex-core tests
//!
//! This module provides shared test infrastructure including:
//! - Database setup and teardown
//! - Mock AI models and responses
//! - Sample data generation
//! - Test assertions and helpers

pub mod db;
pub mod ai;
pub mod content;
pub mod fixtures;

use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

/// Test environment that provides isolated temp directories and cleanup
pub struct TestEnv {
    pub temp_dir: TempDir,
    pub db_path: PathBuf,
    pub models_path: PathBuf,
    pub content_path: PathBuf,
}

impl TestEnv {
    /// Create a new test environment with temporary directories
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let base_path = temp_dir.path();
        
        let db_path = base_path.join("test.db");
        let models_path = base_path.join("models");
        let content_path = base_path.join("content");
        
        std::fs::create_dir_all(&models_path)?;
        std::fs::create_dir_all(&content_path)?;
        
        Ok(Self {
            temp_dir,
            db_path,
            models_path,
            content_path,
        })
    }
    
    /// Get database URL for testing
    pub fn db_url(&self) -> String {
        format!("sqlite:{}", self.db_path.display())
    }
    
    /// Generate a unique test ID
    pub fn test_id() -> String {
        Uuid::new_v4().to_string()[..8].to_string()
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create test environment")
    }
}

/// Macro for creating parameterized tests
#[macro_export]
macro_rules! test_cases {
    ($name:ident: $type:ty = $cases:expr) => {
        mod $name {
            use super::*;
            use rstest::rstest;
            
            #[rstest]
            #[case::generate_cases($cases)]
            fn test(#[case] case: $type) {
                // Test implementation here
            }
        }
    };
}

/// Assert that an error matches a specific pattern
#[macro_export]
macro_rules! assert_error_matches {
    ($result:expr, $pattern:pat) => {
        match $result {
            Err($pattern) => (),
            other => panic!("Expected error matching {}, got {:?}", stringify!($pattern), other),
        }
    };
}

/// Assert that a future times out within the given duration
#[macro_export]
macro_rules! assert_timeout {
    ($future:expr, $duration:expr) => {
        tokio::time::timeout($duration, $future)
            .await
            .expect_err("Operation should have timed out")
    };
}

/// Performance assertion helper
#[macro_export]
macro_rules! assert_performance {
    ($op:expr, $max_duration:expr) => {
        let start = std::time::Instant::now();
        let result = $op;
        let duration = start.elapsed();
        assert!(
            duration <= $max_duration,
            "Operation took {:?}, expected <= {:?}",
            duration,
            $max_duration
        );
        result
    };
}