//! Model downloader with verification and progress tracking

use std::path::{Path, PathBuf};
use reqwest::Client;
use sha2::{Sha256, Digest};
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::Context;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn, error, debug};

use crate::{CodexError, CodexResult};
use super::manifest::ModelManifest;

/// Model downloader with verification and progress tracking
pub struct ModelDownloader {
    client: Client,
    download_dir: PathBuf,
    verify_checksums: bool,
}

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_mbps: f32,
    pub eta_seconds: u64,
}

/// Download result information
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub local_path: PathBuf,
    pub verified: bool,
    pub file_size: u64,
    pub download_time_seconds: f64,
    pub average_speed_mbps: f32,
}

impl ModelDownloader {
    /// Create a new model downloader
    pub fn new<P: AsRef<Path>>(download_dir: P, verify_checksums: bool) -> CodexResult<Self> {
        let client = Client::builder()
            .user_agent("CodexVault/1.0")
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .map_err(|e| CodexError::network(e))?;

        Ok(Self {
            client,
            download_dir: download_dir.as_ref().to_path_buf(),
            verify_checksums,
        })
    }

    /// Download a model with progress tracking
    pub async fn download_model(
        &self,
        manifest: &ModelManifest,
        progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> CodexResult<DownloadResult> {
        info!("Starting download of model: {}", manifest.name);
        
        // Ensure download directory exists
        fs::create_dir_all(&self.download_dir).await
            .context("Failed to create download directory")?;

        let local_path = manifest.get_local_path(&self.download_dir);
        
        // Check if file already exists and is valid
        if self.is_file_valid(&local_path, &manifest.sha256_checksum).await? {
            info!("Model already exists and is valid: {}", local_path.display());
            return Ok(DownloadResult {
                local_path,
                verified: true,
                file_size: manifest.file_size,
                download_time_seconds: 0.0,
                average_speed_mbps: 0.0,
            });
        }

        let start_time = std::time::Instant::now();
        
        // Download the model file
        let _download_result = self.download_file(
            &manifest.download_url,
            &local_path,
            manifest.file_size,
            progress_callback,
        ).await?;

        // Verify checksum if enabled
        let verified = if self.verify_checksums {
            info!("Verifying model checksum...");
            let is_valid = self.verify_file_checksum(&local_path, &manifest.sha256_checksum).await?;
            if !is_valid {
                error!("Checksum verification failed for {}", manifest.name);
                // Delete invalid file
                if let Err(e) = fs::remove_file(&local_path).await {
                    warn!("Failed to remove invalid file: {}", e);
                }
                return Err(CodexError::validation("Downloaded file checksum verification failed"));
            }
            info!("Model checksum verified successfully");
            true
        } else {
            false
        };

        // Download dependencies
        for dep in &manifest.dependencies {
            if dep.required {
                info!("Downloading required dependency: {}", dep.name);
                let dep_path = self.download_dir.join(&dep.name);
                self.download_file(&dep.download_url, &dep_path, dep.file_size, None).await?;
                
                if self.verify_checksums {
                    let is_valid = self.verify_file_checksum(&dep_path, &dep.sha256_checksum).await?;
                    if !is_valid {
                        error!("Dependency checksum verification failed: {}", dep.name);
                        return Err(CodexError::validation(
                            format!("Dependency {} checksum verification failed", dep.name)
                        ));
                    }
                }
            }
        }

        let elapsed = start_time.elapsed();
        let average_speed = (manifest.file_size as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64();

        info!("Model download completed: {} ({:.2} MB/s)", 
              manifest.name, average_speed);

        Ok(DownloadResult {
            local_path,
            verified,
            file_size: manifest.file_size,
            download_time_seconds: elapsed.as_secs_f64(),
            average_speed_mbps: average_speed as f32,
        })
    }

    /// Download a single file with progress tracking
    async fn download_file(
        &self,
        url: &str,
        local_path: &Path,
        expected_size: u64,
        progress_callback: Option<Box<dyn Fn(DownloadProgress) + Send + Sync>>,
    ) -> CodexResult<()> {
        debug!("Downloading {} to {}", url, local_path.display());

        // Create progress bar
        let progress_bar = ProgressBar::new(expected_size);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .map_err(|e| CodexError::internal(format!("Failed to create progress style: {}", e)))?
                .progress_chars("#>-")
        );

        // Start download
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to send download request")?;

        if !response.status().is_success() {
            return Err(CodexError::internal(
                format!("Download failed with status: {}", response.status())
            ));
        }

        // Verify content length
        let content_length = response.content_length().unwrap_or(0);
        if content_length != expected_size && expected_size > 0 {
            warn!("Content length mismatch: expected {}, got {}", expected_size, content_length);
        }

        // Create temporary file
        let temp_path = local_path.with_extension("tmp");
        let mut file = tokio::fs::File::create(&temp_path).await
            .context("Failed to create temporary file")?;

        // Download with progress tracking
        let mut downloaded = 0u64;
        let start_time = std::time::Instant::now();
        let mut stream = response.bytes_stream();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read chunk")?;
            
            file.write_all(&chunk).await
                .context("Failed to write chunk")?;
            
            downloaded += chunk.len() as u64;
            progress_bar.set_position(downloaded);

            // Call progress callback if provided
            if let Some(ref callback) = progress_callback {
                let elapsed = start_time.elapsed().as_secs_f64();
                let speed_mbps = if elapsed > 0.0 {
                    (downloaded as f64 / 1024.0 / 1024.0) / elapsed
                } else {
                    0.0
                };
                
                let eta = if speed_mbps > 0.0 {
                    ((expected_size - downloaded) as f64 / 1024.0 / 1024.0) / speed_mbps
                } else {
                    0.0
                };

                callback(DownloadProgress {
                    downloaded_bytes: downloaded,
                    total_bytes: expected_size,
                    speed_mbps: speed_mbps as f32,
                    eta_seconds: eta as u64,
                });
            }
        }

        file.flush().await.context("Failed to flush file")?;
        drop(file);

        progress_bar.finish_with_message("Download completed");

        // Move temporary file to final location
        fs::rename(&temp_path, local_path).await
            .context("Failed to move downloaded file")?;

        Ok(())
    }

    /// Check if a file exists and has the correct checksum
    async fn is_file_valid(&self, path: &Path, expected_checksum: &str) -> CodexResult<bool> {
        if !path.exists() {
            return Ok(false);
        }

        if !self.verify_checksums {
            return Ok(true);
        }

        self.verify_file_checksum(path, expected_checksum).await
    }

    /// Verify file checksum
    async fn verify_file_checksum(&self, path: &Path, expected_checksum: &str) -> CodexResult<bool> {
        let contents = fs::read(path).await
            .context("Failed to read file for checksum verification")?;

        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();
        let actual_checksum = format!("{:x}", result);

        Ok(actual_checksum.eq_ignore_ascii_case(expected_checksum))
    }

    /// Get download directory
    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }

    /// List downloaded models
    pub async fn list_downloaded_models(&self) -> CodexResult<Vec<PathBuf>> {
        let mut models = Vec::new();
        
        if !self.download_dir.exists() {
            return Ok(models);
        }

        let mut entries = fs::read_dir(&self.download_dir).await
            .context("Failed to read download directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "gguf" || ext == "safetensors" || ext == "pt" || ext == "onnx" {
                        models.push(path);
                    }
                }
            }
        }

        Ok(models)
    }

    /// Delete a downloaded model
    pub async fn delete_model(&self, model_path: &Path) -> CodexResult<()> {
        if !model_path.starts_with(&self.download_dir) {
            return Err(CodexError::permission_denied("Path is outside download directory"));
        }

        fs::remove_file(model_path).await
            .context("Failed to delete model file")?;

        info!("Deleted model: {}", model_path.display());
        Ok(())
    }

    /// Get disk space usage of downloaded models
    pub async fn get_storage_usage(&self) -> CodexResult<u64> {
        let mut total_size = 0u64;

        let models = self.list_downloaded_models().await?;
        for model_path in models {
            if let Ok(metadata) = fs::metadata(&model_path).await {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::update::manifest::ModelManifest;

    #[tokio::test]
    async fn test_downloader_creation() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path(), true);
        assert!(downloader.is_ok());
    }

    #[tokio::test]
    async fn test_checksum_verification() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path(), true).unwrap();
        
        // Create test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"Hello, World!").await.unwrap();
        
        // Expected SHA-256 of "Hello, World!"
        let expected_checksum = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        
        let is_valid = downloader.verify_file_checksum(&test_file, expected_checksum).await.unwrap();
        assert!(is_valid);
        
        // Test with wrong checksum
        let wrong_checksum = "0000000000000000000000000000000000000000000000000000000000000000";
        let is_valid = downloader.verify_file_checksum(&test_file, wrong_checksum).await.unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_storage_usage() {
        let temp_dir = tempdir().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path(), true).unwrap();
        
        // Initially should be 0
        let usage = downloader.get_storage_usage().await.unwrap();
        assert_eq!(usage, 0);
        
        // Create a test model file
        let model_file = temp_dir.path().join("test.gguf");
        fs::write(&model_file, b"test model data").await.unwrap();
        
        let usage = downloader.get_storage_usage().await.unwrap();
        assert!(usage > 0);
    }
}