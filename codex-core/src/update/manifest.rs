//! Update manifest handling

use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::{CodexError, CodexResult};

/// Update manifest containing release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Version number (semver format)
    pub version: String,
    /// Release description/changelog
    pub description: String,
    /// Download URL for the update
    pub download_url: String,
    /// File size in bytes
    pub file_size: usize,
    /// SHA256 checksum of the update file
    pub checksum: String,
    /// Release date
    pub release_date: chrono::DateTime<chrono::Utc>,
    /// Whether this is a critical security update
    pub is_critical: bool,
    /// Minimum version required for this update
    pub min_version: Option<String>,
    /// Platform-specific information
    pub platforms: Vec<PlatformInfo>,
    /// Release channel (stable, beta, nightly)
    pub channel: String,
    /// Release notes in markdown format
    pub release_notes: Option<String>,
    /// Update signature for verification
    pub signature: Option<String>,
}

/// Platform-specific update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Platform identifier (windows, macos, linux)
    pub platform: String,
    /// Architecture (x64, arm64)
    pub arch: String,
    /// Download URL for this platform
    pub download_url: String,
    /// File size for this platform
    pub file_size: usize,
    /// Checksum for this platform
    pub checksum: String,
    /// File format (exe, dmg, appimage, etc.)
    pub format: String,
}

/// Manifest validation result
#[derive(Debug, Clone)]
pub struct ManifestValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl UpdateManifest {
    /// Parse manifest from JSON string
    pub fn from_json(json: &str) -> CodexResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            CodexError::content_processing(format!("Failed to parse update manifest: {}", e))
        })
    }

    /// Convert manifest to JSON string
    pub fn to_json(&self) -> CodexResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            CodexError::content_processing(format!("Failed to serialize update manifest: {}", e))
        })
    }

    /// Validate the manifest
    pub fn validate(&self) -> ManifestValidation {
        let mut validation = ManifestValidation {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Validate version format (basic semver check)
        if !self.is_valid_semver(&self.version) {
            validation.errors.push(format!("Invalid version format: {}", self.version));
            validation.is_valid = false;
        }

        // Validate URLs
        if !self.download_url.starts_with("http://") && !self.download_url.starts_with("https://") {
            validation.errors.push("Invalid download URL".to_string());
            validation.is_valid = false;
        }

        // Validate file size
        if self.file_size == 0 {
            validation.errors.push("File size cannot be zero".to_string());
            validation.is_valid = false;
        }

        // Validate checksum format (should be 64 hex characters for SHA256)
        if self.checksum.len() != 64 || !self.checksum.chars().all(|c| c.is_ascii_hexdigit()) {
            validation.errors.push("Invalid checksum format (expected SHA256)".to_string());
            validation.is_valid = false;
        }

        // Validate platforms
        if self.platforms.is_empty() {
            validation.warnings.push("No platform-specific information provided".to_string());
        }

        for platform in &self.platforms {
            if platform.platform.is_empty() {
                validation.errors.push("Platform name cannot be empty".to_string());
                validation.is_valid = false;
            }

            if platform.arch.is_empty() {
                validation.errors.push("Architecture cannot be empty".to_string());
                validation.is_valid = false;
            }

            if !platform.download_url.starts_with("http://") && !platform.download_url.starts_with("https://") {
                validation.errors.push(format!("Invalid platform download URL for {}", platform.platform));
                validation.is_valid = false;
            }

            if platform.file_size == 0 {
                validation.errors.push(format!("Platform file size cannot be zero for {}", platform.platform));
                validation.is_valid = false;
            }
        }

        // Validate channel
        let valid_channels = ["stable", "beta", "nightly", "dev"];
        if !valid_channels.contains(&self.channel.as_str()) {
            validation.warnings.push(format!("Unknown channel: {}", self.channel));
        }

        // Validate minimum version if present
        if let Some(ref min_version) = self.min_version {
            if !self.is_valid_semver(min_version) {
                validation.errors.push(format!("Invalid minimum version format: {}", min_version));
                validation.is_valid = false;
            }
        }

        validation
    }

    /// Check if a version string is valid semver
    fn is_valid_semver(&self, version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        parts.iter().all(|part| part.parse::<u32>().is_ok())
    }

    /// Get platform-specific information for current platform
    pub fn get_platform_info(&self) -> Option<&PlatformInfo> {
        let current_platform = std::env::consts::OS;
        let current_arch = std::env::consts::ARCH;

        self.platforms.iter().find(|p| {
            p.platform == current_platform && p.arch == current_arch
        })
    }

    /// Check if this manifest is compatible with a given version
    pub fn is_compatible_with(&self, current_version: &str) -> bool {
        if let Some(ref min_version) = self.min_version {
            self.compare_versions(current_version, min_version) >= 0
        } else {
            true
        }
    }

    /// Compare two version strings (returns -1, 0, or 1)
    fn compare_versions(&self, v1: &str, v2: &str) -> i32 {
        let v1_parts: Vec<u32> = v1.split('.').filter_map(|s| s.parse().ok()).collect();
        let v2_parts: Vec<u32> = v2.split('.').filter_map(|s| s.parse().ok()).collect();

        for i in 0..3 {
            let p1 = v1_parts.get(i).unwrap_or(&0);
            let p2 = v2_parts.get(i).unwrap_or(&0);

            if p1 < p2 {
                return -1;
            } else if p1 > p2 {
                return 1;
            }
        }

        0
    }

    /// Get the appropriate download URL for current platform
    pub fn get_download_url(&self) -> String {
        if let Some(platform_info) = self.get_platform_info() {
            platform_info.download_url.clone()
        } else {
            self.download_url.clone()
        }
    }

    /// Get the appropriate file size for current platform
    pub fn get_file_size(&self) -> usize {
        if let Some(platform_info) = self.get_platform_info() {
            platform_info.file_size
        } else {
            self.file_size
        }
    }

    /// Get the appropriate checksum for current platform
    pub fn get_checksum(&self) -> String {
        if let Some(platform_info) = self.get_platform_info() {
            platform_info.checksum.clone()
        } else {
            self.checksum.clone()
        }
    }
}

impl Default for UpdateManifest {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            description: "Default update".to_string(),
            download_url: "https://example.com/update.zip".to_string(),
            file_size: 0,
            checksum: "0".repeat(64),
            release_date: chrono::Utc::now(),
            is_critical: false,
            min_version: None,
            platforms: Vec::new(),
            channel: "stable".to_string(),
            release_notes: None,
            signature: None,
        }
    }
}

/// Manifest builder for creating update manifests
pub struct ManifestBuilder {
    manifest: UpdateManifest,
}

impl ManifestBuilder {
    /// Create a new manifest builder
    pub fn new() -> Self {
        Self {
            manifest: UpdateManifest::default(),
        }
    }

    /// Set version
    pub fn version<S: Into<String>>(mut self, version: S) -> Self {
        self.manifest.version = version.into();
        self
    }

    /// Set description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.manifest.description = description.into();
        self
    }

    /// Set download URL
    pub fn download_url<S: Into<String>>(mut self, url: S) -> Self {
        self.manifest.download_url = url.into();
        self
    }

    /// Set file size
    pub fn file_size(mut self, size: usize) -> Self {
        self.manifest.file_size = size;
        self
    }

    /// Set checksum
    pub fn checksum<S: Into<String>>(mut self, checksum: S) -> Self {
        self.manifest.checksum = checksum.into();
        self
    }

    /// Set as critical update
    pub fn critical(mut self, is_critical: bool) -> Self {
        self.manifest.is_critical = is_critical;
        self
    }

    /// Set minimum version
    pub fn min_version<S: Into<String>>(mut self, min_version: S) -> Self {
        self.manifest.min_version = Some(min_version.into());
        self
    }

    /// Set channel
    pub fn channel<S: Into<String>>(mut self, channel: S) -> Self {
        self.manifest.channel = channel.into();
        self
    }

    /// Add platform information
    pub fn add_platform(mut self, platform: PlatformInfo) -> Self {
        self.manifest.platforms.push(platform);
        self
    }

    /// Set release notes
    pub fn release_notes<S: Into<String>>(mut self, notes: S) -> Self {
        self.manifest.release_notes = Some(notes.into());
        self
    }

    /// Build the manifest
    pub fn build(self) -> UpdateManifest {
        self.manifest
    }
}

impl Default for ManifestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_validation() {
        let manifest = UpdateManifest {
            version: "1.0.0".to_string(),
            description: "Test update".to_string(),
            download_url: "https://example.com/update.zip".to_string(),
            file_size: 1024,
            checksum: "a".repeat(64),
            release_date: chrono::Utc::now(),
            is_critical: false,
            min_version: None,
            platforms: Vec::new(),
            channel: "stable".to_string(),
            release_notes: None,
            signature: None,
        };

        let validation = manifest.validate();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
    }

    #[test]
    fn test_invalid_manifest() {
        let manifest = UpdateManifest {
            version: "invalid".to_string(),
            download_url: "not-a-url".to_string(),
            file_size: 0,
            checksum: "invalid".to_string(),
            ..Default::default()
        };

        let validation = manifest.validate();
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_version_comparison() {
        let manifest = UpdateManifest::default();
        
        assert_eq!(manifest.compare_versions("1.0.0", "1.0.0"), 0);
        assert_eq!(manifest.compare_versions("1.0.0", "1.0.1"), -1);
        assert_eq!(manifest.compare_versions("1.0.1", "1.0.0"), 1);
        assert_eq!(manifest.compare_versions("2.0.0", "1.9.9"), 1);
    }

    #[test]
    fn test_manifest_builder() {
        let manifest = ManifestBuilder::new()
            .version("2.0.0")
            .description("Major update")
            .critical(true)
            .channel("stable")
            .build();

        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.description, "Major update");
        assert!(manifest.is_critical);
        assert_eq!(manifest.channel, "stable");
    }
}