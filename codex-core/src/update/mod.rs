//! Application update management module

use anyhow::Result;
use tracing::{info, debug, warn};

use crate::{CodexError, CodexResult};
use crate::config::UpdateConfig;

pub mod manager;
pub mod manifest;
pub mod downloader;
pub mod model_downloader;
pub use manager::*;
pub use manifest::*;
// Import specific items to avoid name conflicts
pub use downloader::{ModelDownloader as OriginalModelDownloader, DownloadResult, DownloadProgress as OriginalDownloadProgress};
pub use model_downloader::{ModelDownloader, DownloadProgress, DownloadStage};

/// Update manager for handling application updates
#[derive(Debug)]
pub struct UpdateManager {
    config: UpdateConfig,
    client: reqwest::Client,
}

impl UpdateManager {
    /// Create a new update manager
    pub async fn new(config: &UpdateConfig) -> Result<Self> {
        info!("Initializing update manager");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Codex-Vault/1.0")
            .build()?;

        Ok(Self {
            config: config.clone(),
            client,
        })
    }

    /// Check for available updates
    pub async fn check_for_updates(&self) -> CodexResult<Option<UpdateInfo>> {
        if !self.config.auto_check {
            debug!("Auto-check disabled, skipping update check");
            return Ok(None);
        }

        info!("Checking for updates from: {}", self.config.server_url);

        let manifest_url = format!("{}/manifest.json", self.config.server_url);
        
        match self.client.get(&manifest_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let manifest: UpdateManifest = response.json().await?;
                    
                    if self.is_newer_version(&manifest.version)? {
                        info!("Update available: {}", manifest.version);
                        
                        let update_info = UpdateInfo {
                            version: manifest.version,
                            description: manifest.description,
                            download_url: manifest.download_url,
                            file_size: manifest.file_size,
                            checksum: manifest.checksum,
                            release_date: manifest.release_date,
                            is_critical: manifest.is_critical,
                            min_version: manifest.min_version,
                        };
                        
                        return Ok(Some(update_info));
                    } else {
                        debug!("No updates available");
                        return Ok(None);
                    }
                } else {
                    warn!("Failed to fetch update manifest: {}", response.status());
                    return Ok(None);
                }
            }
            Err(e) => {
                warn!("Failed to check for updates: {}", e);
                return Ok(None);
            }
        }
    }

    /// Download and install an update
    pub async fn download_and_install_update(&self, update_info: &UpdateInfo) -> CodexResult<()> {
        info!("Downloading update: {}", update_info.version);

        // Download the update
        let update_file = self.download_update(update_info).await?;

        // Verify checksum
        self.verify_update_checksum(&update_file, &update_info.checksum).await?;

        // Install the update
        self.install_update(&update_file).await?;

        info!("Update installed successfully: {}", update_info.version);
        Ok(())
    }

    /// Download update file
    async fn download_update(&self, update_info: &UpdateInfo) -> CodexResult<Vec<u8>> {
        debug!("Downloading from: {}", update_info.download_url);

        let response = self.client
            .get(&update_info.download_url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CodexError::update(format!(
                "Failed to download update: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        
        if bytes.len() != update_info.file_size {
            return Err(CodexError::update(format!(
                "Downloaded file size mismatch: expected {}, got {}",
                update_info.file_size,
                bytes.len()
            )));
        }

        Ok(bytes.to_vec())
    }

    /// Verify update file checksum
    async fn verify_update_checksum(&self, file_data: &[u8], expected_checksum: &str) -> CodexResult<()> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        file_data.hash(&mut hasher);
        let calculated_checksum = format!("{:x}", hasher.finish());

        if calculated_checksum != expected_checksum {
            return Err(CodexError::update(format!(
                "Checksum verification failed: expected {}, got {}",
                expected_checksum,
                calculated_checksum
            )));
        }

        debug!("Checksum verification passed");
        Ok(())
    }

    /// Install the downloaded update
    async fn install_update(&self, _update_file: &[u8]) -> CodexResult<()> {
        // In a real implementation, this would:
        // 1. Extract the update file (if it's an archive)
        // 2. Backup current installation
        // 3. Replace application files
        // 4. Update configuration if needed
        // 5. Restart the application

        // For now, this is a placeholder
        info!("Update installation is not fully implemented (placeholder)");
        
        // In a Tauri application, you would typically use the built-in updater
        // which handles the platform-specific update process
        
        Ok(())
    }

    /// Check if a version is newer than the current version
    fn is_newer_version(&self, new_version: &str) -> CodexResult<bool> {
        let current_version = env!("CARGO_PKG_VERSION");
        
        // Simple version comparison (in a real implementation, use semver)
        let current_parts: Vec<u32> = current_version
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        
        let new_parts: Vec<u32> = new_version
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        if current_parts.len() != 3 || new_parts.len() != 3 {
            return Err(CodexError::validation("Invalid version format"));
        }

        // Compare major.minor.patch
        for i in 0..3 {
            if new_parts[i] > current_parts[i] {
                return Ok(true);
            } else if new_parts[i] < current_parts[i] {
                return Ok(false);
            }
        }

        Ok(false) // Versions are equal
    }

    /// Get current version
    pub fn get_current_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Get update configuration
    pub fn get_config(&self) -> &UpdateConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: UpdateConfig) {
        self.config = config;
    }

    /// Health check
    pub async fn health_check(&self) -> CodexResult<bool> {
        // Try to reach the update server
        match self.client.head(&self.config.server_url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false), // Server unreachable, but that's not necessarily a failure
        }
    }

    /// Shutdown update manager
    pub async fn shutdown(&self) -> CodexResult<()> {
        info!("Shutting down update manager");
        // No specific cleanup needed
        Ok(())
    }
}

/// Information about an available update
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub description: String,
    pub download_url: String,
    pub file_size: usize,
    pub checksum: String,
    pub release_date: chrono::DateTime<chrono::Utc>,
    pub is_critical: bool,
    pub min_version: Option<String>,
}

/// Update status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum UpdateStatus {
    /// No updates available
    UpToDate,
    /// Update available but not critical
    UpdateAvailable(UpdateInfo),
    /// Critical update available
    CriticalUpdate(UpdateInfo),
    /// Update check failed
    CheckFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let config = UpdateConfig::default();
        let manager = UpdateManager {
            config,
            client: reqwest::Client::new(),
        };

        // These tests assume current version is 0.1.0
        assert!(manager.is_newer_version("0.1.1").unwrap());
        assert!(manager.is_newer_version("0.2.0").unwrap());
        assert!(manager.is_newer_version("1.0.0").unwrap());
        assert!(!manager.is_newer_version("0.1.0").unwrap());
        assert!(!manager.is_newer_version("0.0.9").unwrap());
    }
}