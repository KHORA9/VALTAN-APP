//! Model download and verification system
//!
//! This module provides functionality to download AI models with progress tracking,
//! checksum verification, and integrity validation.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use reqwest::Client;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{info, warn, debug};

use crate::{CodexError, CodexResult};
use super::manifest::{ModelManifest, ModelRegistry};
use crate::ai::engine::GGUFEngine;

/// Model download progress callback
pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Bytes downloaded so far
    pub downloaded_bytes: u64,
    /// Total bytes to download
    pub total_bytes: u64,
    /// Download speed in bytes per second
    pub speed_bps: u64,
    /// Estimated time remaining in seconds
    pub eta_seconds: u64,
    /// Progress percentage (0.0 - 1.0)
    pub progress: f64,
    /// Current stage of download
    pub stage: DownloadStage,
}

/// Download stages
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStage {
    /// Initializing download
    Initializing,
    /// Downloading model file
    Downloading,
    /// Verifying checksum
    Verifying,
    /// Download completed successfully
    Completed,
    /// Download failed
    Failed(String),
}

/// Model downloader with progress tracking and verification
pub struct ModelDownloader {
    client: Client,
    download_dir: PathBuf,
    progress_callback: Option<ProgressCallback>,
    chunk_size: usize,
    timeout: Duration,
}

impl ModelDownloader {
    /// Create a new model downloader
    pub fn new(download_dir: PathBuf) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Codex-Vault/1.0")
            .build()
            .unwrap();

        Self {
            client,
            download_dir,
            progress_callback: None,
            chunk_size: 8192, // 8KB chunks
            timeout: Duration::from_secs(300), // 5 minute timeout
        }
    }

    /// Set progress callback for download tracking
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Set chunk size for downloads
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Set timeout for downloads
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Download a model from manifest with verification
    pub async fn download_model(&self, manifest: &ModelManifest) -> CodexResult<PathBuf> {
        info!("Starting download of model: {}", manifest.name);

        // Ensure download directory exists
        tokio::fs::create_dir_all(&self.download_dir).await
            .map_err(|e| CodexError::io(e))?;

        // Get download URL and expected size
        let download_url = &manifest.download_url;
        let expected_size = manifest.file_size;
        let expected_checksum = &manifest.sha256_checksum;

        // Calculate target file path
        let target_path = manifest.get_local_path(&self.download_dir);
        
        // Check if file already exists and is valid
        if target_path.exists() {
            info!("Model file already exists, verifying integrity");
            if self.verify_existing_file(&target_path, expected_checksum).await? {
                info!("Existing model file is valid, skipping download");
                return Ok(target_path);
            } else {
                warn!("Existing model file is invalid, re-downloading");
                tokio::fs::remove_file(&target_path).await
                    .map_err(|e| CodexError::io(e))?;
            }
        }

        // Start download process
        self.notify_progress(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes: expected_size,
            speed_bps: 0,
            eta_seconds: 0,
            progress: 0.0,
            stage: DownloadStage::Initializing,
        });

        // Download the file
        let downloaded_path = self.download_file_with_progress(
            download_url,
            &target_path,
            expected_size,
        ).await?;

        // Verify checksum
        self.notify_progress(DownloadProgress {
            downloaded_bytes: expected_size,
            total_bytes: expected_size,
            speed_bps: 0,
            eta_seconds: 0,
            progress: 1.0,
            stage: DownloadStage::Verifying,
        });

        if !self.verify_checksum(&downloaded_path, expected_checksum).await? {
            // Remove invalid file
            tokio::fs::remove_file(&downloaded_path).await
                .map_err(|e| CodexError::io(e))?;
            
            let error_msg = "Checksum verification failed";
            self.notify_progress(DownloadProgress {
                downloaded_bytes: 0,
                total_bytes: expected_size,
                speed_bps: 0,
                eta_seconds: 0,
                progress: 0.0,
                stage: DownloadStage::Failed(error_msg.to_string()),
            });
            
            return Err(CodexError::validation(error_msg));
        }

        // Download dependencies (tokenizer, config files, etc.)
        for dependency in &manifest.dependencies {
            if dependency.required {
                self.download_dependency(dependency, &self.download_dir).await?;
            }
        }

        // Download completed successfully
        self.notify_progress(DownloadProgress {
            downloaded_bytes: expected_size,
            total_bytes: expected_size,
            speed_bps: 0,
            eta_seconds: 0,
            progress: 1.0,
            stage: DownloadStage::Completed,
        });

        info!("Model download completed successfully: {}", downloaded_path.display());
        Ok(downloaded_path)
    }

    /// Download a file with progress tracking
    async fn download_file_with_progress(
        &self,
        url: &str,
        target_path: &Path,
        expected_size: u64,
    ) -> CodexResult<PathBuf> {
        info!("Downloading from: {}", url);
        info!("Target path: {}", target_path.display());

        // Create the target file
        let mut file = File::create(target_path).await
            .map_err(|e| CodexError::io(e))?;

        // Start HTTP request
        let response = self.client
            .get(url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| CodexError::network(e))?;

        if !response.status().is_success() {
            return Err(CodexError::internal(
                format!("HTTP request failed: {}", response.status())
            ));
        }

        // Get content length
        let content_length = response.content_length().unwrap_or(expected_size);
        
        // Create progress tracking
        let mut downloaded = 0u64;
        let mut last_update = Instant::now();
        let mut speed_samples = Vec::new();
        
        self.notify_progress(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes: content_length,
            speed_bps: 0,
            eta_seconds: 0,
            progress: 0.0,
            stage: DownloadStage::Downloading,
        });

        // Download in chunks
        let mut stream = response.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| CodexError::network(e))?;
            
            // Write chunk to file
            file.write_all(&chunk).await
                .map_err(|e| CodexError::io(e))?;
            
            downloaded += chunk.len() as u64;
            
            // Update progress periodically
            let now = Instant::now();
            if now.duration_since(last_update) >= Duration::from_millis(100) {
                // Calculate speed
                let speed_bps = self.calculate_speed(&mut speed_samples, downloaded, now);
                
                // Calculate ETA
                let remaining_bytes = content_length.saturating_sub(downloaded);
                let eta_seconds = if speed_bps > 0 {
                    remaining_bytes / speed_bps
                } else {
                    0
                };
                
                let progress = if content_length > 0 {
                    downloaded as f64 / content_length as f64
                } else {
                    0.0
                };
                
                self.notify_progress(DownloadProgress {
                    downloaded_bytes: downloaded,
                    total_bytes: content_length,
                    speed_bps,
                    eta_seconds,
                    progress,
                    stage: DownloadStage::Downloading,
                });
                
                last_update = now;
            }
        }

        // Ensure all data is written
        file.flush().await
            .map_err(|e| CodexError::io(e))?;
        
        debug!("Download completed: {} bytes", downloaded);
        Ok(target_path.to_path_buf())
    }

    /// Calculate download speed with smoothing
    fn calculate_speed(&self, speed_samples: &mut Vec<(Instant, u64)>, downloaded: u64, now: Instant) -> u64 {
        speed_samples.push((now, downloaded));
        
        // Keep only last 10 samples (about 1 second)
        if speed_samples.len() > 10 {
            speed_samples.remove(0);
        }
        
        if speed_samples.len() < 2 {
            return 0;
        }
        
        let first = speed_samples.first().unwrap();
        let last = speed_samples.last().unwrap();
        
        let time_diff = last.0.duration_since(first.0).as_secs_f64();
        let bytes_diff = last.1.saturating_sub(first.1);
        
        if time_diff > 0.0 {
            (bytes_diff as f64 / time_diff) as u64
        } else {
            0
        }
    }

    /// Download a model dependency
    async fn download_dependency(
        &self,
        dependency: &super::manifest::ModelDependency,
        target_dir: &Path,
    ) -> CodexResult<()> {
        info!("Downloading dependency: {}", dependency.name);
        
        let target_path = target_dir.join(&dependency.name);
        
        // Check if dependency already exists and is valid
        if target_path.exists() {
            if self.verify_checksum(&target_path, &dependency.sha256_checksum).await? {
                info!("Dependency {} already exists and is valid", dependency.name);
                return Ok(());
            } else {
                warn!("Dependency {} exists but is invalid, re-downloading", dependency.name);
                tokio::fs::remove_file(&target_path).await
                    .map_err(|e| CodexError::io(e))?;
            }
        }
        
        self.download_file_with_progress(
            &dependency.download_url,
            &target_path,
            dependency.file_size,
        ).await?;
        
        if !self.verify_checksum(&target_path, &dependency.sha256_checksum).await? {
            tokio::fs::remove_file(&target_path).await
                .map_err(|e| CodexError::io(e))?;
            return Err(CodexError::validation(
                format!("Dependency {} checksum verification failed", dependency.name)
            ));
        }
        
        info!("Dependency {} downloaded successfully", dependency.name);
        Ok(())
    }

    /// Verify checksum of a file
    async fn verify_checksum(&self, file_path: &Path, expected_checksum: &str) -> CodexResult<bool> {
        debug!("Verifying checksum for: {}", file_path.display());
        
        let actual_checksum = GGUFEngine::calculate_checksum(file_path).await?;
        let is_valid = actual_checksum.eq_ignore_ascii_case(expected_checksum);
        
        if is_valid {
            debug!("Checksum verification passed");
        } else {
            warn!("Checksum verification failed: expected {}, got {}", 
                  expected_checksum, actual_checksum);
        }
        
        Ok(is_valid)
    }

    /// Verify existing file without re-downloading
    async fn verify_existing_file(&self, file_path: &Path, expected_checksum: &str) -> CodexResult<bool> {
        // Check file size first (quick check)
        let metadata = tokio::fs::metadata(file_path).await
            .map_err(|e| CodexError::io(e))?;
        
        if metadata.len() == 0 {
            return Ok(false);
        }
        
        // Verify checksum
        self.verify_checksum(file_path, expected_checksum).await
    }

    /// Notify progress callback if set
    fn notify_progress(&self, progress: DownloadProgress) {
        if let Some(ref callback) = self.progress_callback {
            callback(progress);
        }
    }

    /// Get available models from registry
    pub async fn get_available_models(&self, registry_url: &str) -> CodexResult<ModelRegistry> {
        info!("Fetching model registry from: {}", registry_url);
        
        let response = self.client
            .get(registry_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| CodexError::network(e))?;
        
        if !response.status().is_success() {
            return Err(CodexError::internal(
                format!("Failed to fetch registry: {}", response.status())
            ));
        }
        
        let registry_json = response.text().await
            .map_err(|e| CodexError::network(e))?;
        
        ModelRegistry::from_json(&registry_json)
    }

    /// Check if a model is already downloaded and valid
    pub async fn is_model_downloaded(&self, manifest: &ModelManifest) -> CodexResult<bool> {
        let target_path = manifest.get_local_path(&self.download_dir);
        
        if !target_path.exists() {
            return Ok(false);
        }
        
        // Verify integrity
        self.verify_existing_file(&target_path, &manifest.sha256_checksum).await
    }

    /// Remove a downloaded model
    pub async fn remove_model(&self, manifest: &ModelManifest) -> CodexResult<()> {
        let target_path = manifest.get_local_path(&self.download_dir);
        
        if target_path.exists() {
            tokio::fs::remove_file(&target_path).await
                .map_err(|e| CodexError::io(e))?;
            info!("Removed model: {}", target_path.display());
        }
        
        // Remove dependencies
        for dependency in &manifest.dependencies {
            let dep_path = self.download_dir.join(&dependency.name);
            if dep_path.exists() {
                tokio::fs::remove_file(&dep_path).await
                    .map_err(|e| CodexError::io(e))?;
                info!("Removed dependency: {}", dep_path.display());
            }
        }
        
        Ok(())
    }

    /// Get download directory
    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }

    /// Create progress bar for CLI usage
    pub fn create_progress_bar(&self, total_size: u64) -> ProgressBar {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        pb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_downloader_creation() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path().to_path_buf());
        
        assert_eq!(downloader.download_dir(), temp_dir.path());
        assert_eq!(downloader.chunk_size, 8192);
    }

    #[test]
    fn test_downloader_configuration() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path().to_path_buf())
            .with_chunk_size(16384)
            .with_timeout(Duration::from_secs(600));
        
        assert_eq!(downloader.chunk_size, 16384);
        assert_eq!(downloader.timeout, Duration::from_secs(600));
    }

    #[tokio::test]
    async fn test_checksum_verification() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path().to_path_buf());
        
        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        let test_content = b"Hello, World!";
        fs::write(&test_file, test_content).unwrap();
        
        // Calculate expected checksum using GGUFEngine method
        let expected_checksum = crate::ai::engine::GGUFEngine::calculate_checksum(&test_file).await.unwrap();
        
        // Verify checksum
        let is_valid = downloader.verify_checksum(&test_file, &expected_checksum).await.unwrap();
        assert!(is_valid);
        
        // Test with wrong checksum
        let wrong_checksum = "0".repeat(64);
        let is_invalid = downloader.verify_checksum(&test_file, &wrong_checksum).await.unwrap();
        assert!(!is_invalid);
    }
}
