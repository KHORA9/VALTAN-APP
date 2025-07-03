//! Update manifest handling

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

// =============================================================================
// MODEL MANIFEST SYSTEM
// =============================================================================

/// Model manifest containing AI model information and download details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    /// Model name and identifier
    pub name: String,
    /// Model version
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Model format (gguf, safetensors, pytorch)
    pub format: ModelFormat,
    /// Model family/architecture (llama2, mistral, etc.)
    pub architecture: String,
    /// Parameter count (7b, 13b, 70b)
    pub parameter_count: String,
    /// Quantization level (q4_k_m, q8_0, fp16, etc.)
    pub quantization: String,
    /// Download URL for the model file
    pub download_url: String,
    /// Model file size in bytes
    pub file_size: u64,
    /// SHA-256 checksum for verification
    pub sha256_checksum: String,
    /// Context length supported by model
    pub context_length: usize,
    /// Recommended hardware requirements
    pub hardware_requirements: HardwareRequirements,
    /// License information
    pub license: String,
    /// Model creation/release date
    pub release_date: chrono::DateTime<chrono::Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Performance metrics
    pub performance: Option<ModelPerformance>,
    /// Dependencies (tokenizer, config files)
    pub dependencies: Vec<ModelDependency>,
}

/// Model file format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelFormat {
    GGUF,
    Safetensors,
    PyTorch,
    ONNX,
}

/// Hardware requirements for model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequirements {
    /// Minimum RAM in GB
    pub min_ram_gb: f32,
    /// Recommended RAM in GB
    pub recommended_ram_gb: f32,
    /// VRAM requirement in GB (for GPU acceleration)
    pub vram_gb: Option<f32>,
    /// Minimum CPU cores
    pub min_cpu_cores: usize,
    /// Supported devices
    pub supported_devices: Vec<String>,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    /// Tokens per second on reference hardware
    pub tokens_per_second: f32,
    /// Reference hardware description
    pub reference_hardware: String,
    /// Perplexity score (lower is better)
    pub perplexity: Option<f32>,
    /// BLEU score for translation tasks
    pub bleu_score: Option<f32>,
    /// Model accuracy percentage
    pub accuracy: Option<f32>,
}

/// Model dependency (tokenizer, config, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDependency {
    /// Dependency name
    pub name: String,
    /// Dependency type (tokenizer, config, vocab)
    pub dependency_type: String,
    /// Download URL
    pub download_url: String,
    /// File size in bytes
    pub file_size: u64,
    /// SHA-256 checksum
    pub sha256_checksum: String,
    /// Whether this dependency is required
    pub required: bool,
}

impl ModelManifest {
    /// Create a new model manifest for Mistral 7B Instruct Q4_K
    pub fn mistral_7b_instruct_q4k() -> Self {
        Self {
            name: "mistral-7b-instruct-q4_k".to_string(),
            version: "v0.1".to_string(),
            description: "Mistral 7B Instruct model quantized to Q4_K for optimal performance/quality balance".to_string(),
            format: ModelFormat::GGUF,
            architecture: "mistral".to_string(),
            parameter_count: "7b".to_string(),
            quantization: "q4_k_m".to_string(),
            download_url: "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GGUF/resolve/main/mistral-7b-instruct-v0.1.q4_K_M.gguf".to_string(),
            file_size: 4_368_439_552, // ~4.1GB
            sha256_checksum: "1ee6114517d2f770425c880e645aa1c6e92e5f55d2adf854f769b30eed4a434b".to_string(),
            context_length: 8192,
            hardware_requirements: HardwareRequirements {
                min_ram_gb: 6.0,
                recommended_ram_gb: 8.0,
                vram_gb: Some(6.0),
                min_cpu_cores: 4,
                supported_devices: vec!["cpu".to_string(), "cuda".to_string(), "metal".to_string()],
            },
            license: "Apache 2.0".to_string(),
            release_date: chrono::DateTime::parse_from_rfc3339("2023-09-27T00:00:00Z").unwrap().with_timezone(&chrono::Utc),
            tags: vec!["instruct".to_string(), "chat".to_string(), "general".to_string()],
            performance: Some(ModelPerformance {
                tokens_per_second: 25.0,
                reference_hardware: "M1 MacBook Pro 16GB".to_string(),
                perplexity: Some(3.8),
                bleu_score: None,
                accuracy: Some(75.2),
            }),
            dependencies: vec![
                ModelDependency {
                    name: "tokenizer.json".to_string(),
                    dependency_type: "tokenizer".to_string(),
                    download_url: "https://huggingface.co/mistralai/Mistral-7B-Instruct-v0.1/resolve/main/tokenizer.json".to_string(),
                    file_size: 1_842_622,
                    sha256_checksum: "dadfd56d766715c61d2ef780a525ab43b8e6da4de6865bda3d95fdef5e134055".to_string(),
                    required: true,
                },
            ],
        }
    }

    /// Parse model manifest from JSON
    pub fn from_json(json: &str) -> CodexResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            CodexError::content_processing(format!("Failed to parse model manifest: {}", e))
        })
    }

    /// Convert manifest to JSON
    pub fn to_json(&self) -> CodexResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            CodexError::content_processing(format!("Failed to serialize model manifest: {}", e))
        })
    }

    /// Validate the model manifest
    pub fn validate(&self) -> ManifestValidation {
        let mut validation = ManifestValidation {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Validate required fields
        if self.name.is_empty() {
            validation.errors.push("Model name cannot be empty".to_string());
            validation.is_valid = false;
        }

        if self.version.is_empty() {
            validation.errors.push("Model version cannot be empty".to_string());
            validation.is_valid = false;
        }

        // Validate download URL
        if !self.download_url.starts_with("http://") && !self.download_url.starts_with("https://") {
            validation.errors.push("Invalid download URL".to_string());
            validation.is_valid = false;
        }

        // Validate file size
        if self.file_size == 0 {
            validation.errors.push("File size cannot be zero".to_string());
            validation.is_valid = false;
        }

        // Validate checksum (SHA-256 should be 64 hex characters)
        if self.sha256_checksum.len() != 64 || !self.sha256_checksum.chars().all(|c| c.is_ascii_hexdigit()) {
            validation.errors.push("Invalid SHA-256 checksum format".to_string());
            validation.is_valid = false;
        }

        // Validate context length
        if self.context_length == 0 {
            validation.errors.push("Context length must be greater than 0".to_string());
            validation.is_valid = false;
        }

        // Validate hardware requirements
        if self.hardware_requirements.min_ram_gb <= 0.0 {
            validation.errors.push("Minimum RAM must be greater than 0".to_string());
            validation.is_valid = false;
        }

        if self.hardware_requirements.recommended_ram_gb < self.hardware_requirements.min_ram_gb {
            validation.errors.push("Recommended RAM cannot be less than minimum RAM".to_string());
            validation.is_valid = false;
        }

        // Validate dependencies
        for (i, dep) in self.dependencies.iter().enumerate() {
            if dep.name.is_empty() {
                validation.errors.push(format!("Dependency {} name cannot be empty", i));
                validation.is_valid = false;
            }

            if dep.sha256_checksum.len() != 64 || !dep.sha256_checksum.chars().all(|c| c.is_ascii_hexdigit()) {
                validation.errors.push(format!("Invalid checksum for dependency {}", dep.name));
                validation.is_valid = false;
            }
        }

        validation
    }

    /// Check if this model can run on the current system
    pub fn is_compatible_with_system(&self) -> bool {
        // Check RAM requirements (simplified check)
        let min_ram_bytes = (self.hardware_requirements.min_ram_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        
        // In a real implementation, you would check actual system RAM
        // For now, assume 8GB minimum
        let system_ram_bytes = 8u64 * 1024 * 1024 * 1024; // 8GB
        
        if min_ram_bytes > system_ram_bytes {
            return false;
        }

        // Check if current device is supported
        let current_device = if cfg!(feature = "cuda") {
            "cuda"
        } else if cfg!(feature = "metal") {
            "metal"
        } else {
            "cpu"
        };

        self.hardware_requirements.supported_devices.contains(&current_device.to_string())
    }

    /// Get the local file path where this model should be stored
    pub fn get_local_path(&self, models_dir: &Path) -> PathBuf {
        let filename = format!("{}-{}.{}", 
            self.name, 
            self.version, 
            match self.format {
                ModelFormat::GGUF => "gguf",
                ModelFormat::Safetensors => "safetensors",
                ModelFormat::PyTorch => "pt",
                ModelFormat::ONNX => "onnx",
            }
        );
        models_dir.join(filename)
    }

    /// Get estimated download time in minutes for given speed (MB/s)
    pub fn estimated_download_time(&self, speed_mbps: f32) -> f32 {
        let file_size_mb = self.file_size as f32 / (1024.0 * 1024.0);
        (file_size_mb / speed_mbps) / 60.0 // Convert to minutes
    }
}

/// Model registry containing multiple model manifests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    /// Registry version
    pub version: String,
    /// Last updated timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// Available models
    pub models: Vec<ModelManifest>,
    /// Registry metadata
    pub metadata: ModelRegistryMetadata,
}

/// Registry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryMetadata {
    /// Registry name
    pub name: String,
    /// Registry URL
    pub url: String,
    /// Minimum client version required
    pub min_client_version: String,
    /// Registry description
    pub description: String,
}

impl ModelRegistry {
    /// Create a default registry with recommended models
    pub fn default_registry() -> Self {
        Self {
            version: "1.0.0".to_string(),
            last_updated: chrono::Utc::now(),
            models: vec![
                ModelManifest::mistral_7b_instruct_q4k(),
            ],
            metadata: ModelRegistryMetadata {
                name: "Codex Vault Official Models".to_string(),
                url: "https://models.codex-vault.com/registry.json".to_string(),
                min_client_version: "0.1.0".to_string(),
                description: "Official model registry for Codex Vault AI models".to_string(),
            },
        }
    }

    /// Find a model by name
    pub fn find_model(&self, name: &str) -> Option<&ModelManifest> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Get models compatible with current system
    pub fn compatible_models(&self) -> Vec<&ModelManifest> {
        self.models.iter()
            .filter(|m| m.is_compatible_with_system())
            .collect()
    }

    /// Get models by architecture
    pub fn models_by_architecture(&self, architecture: &str) -> Vec<&ModelManifest> {
        self.models.iter()
            .filter(|m| m.architecture == architecture)
            .collect()
    }

    /// Parse registry from JSON
    pub fn from_json(json: &str) -> CodexResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            CodexError::content_processing(format!("Failed to parse model registry: {}", e))
        })
    }

    /// Convert registry to JSON
    pub fn to_json(&self) -> CodexResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            CodexError::content_processing(format!("Failed to serialize model registry: {}", e))
        })
    }
}

#[cfg(test)]
mod model_tests {
    use super::*;

    #[test]
    fn test_mistral_model_manifest() {
        let manifest = ModelManifest::mistral_7b_instruct_q4k();
        let validation = manifest.validate();
        
        assert!(validation.is_valid, "Validation errors: {:?}", validation.errors);
        assert_eq!(manifest.format, ModelFormat::GGUF);
        assert_eq!(manifest.architecture, "mistral");
        assert!(!manifest.dependencies.is_empty());
    }

    #[test]
    fn test_model_registry() {
        let registry = ModelRegistry::default_registry();
        
        assert!(!registry.models.is_empty());
        assert!(registry.find_model("mistral-7b-instruct-q4_k").is_some());
        
        let compatible = registry.compatible_models();
        assert!(!compatible.is_empty());
    }

    #[test]
    fn test_model_local_path() {
        let manifest = ModelManifest::mistral_7b_instruct_q4k();
        let models_dir = Path::new("/tmp/models");
        let local_path = manifest.get_local_path(models_dir);
        
        assert!(local_path.to_string_lossy().contains("mistral-7b-instruct-q4_k"));
        assert!(local_path.to_string_lossy().ends_with(".gguf"));
    }

    #[test]
    fn test_download_time_estimation() {
        let manifest = ModelManifest::mistral_7b_instruct_q4k();
        let time_fast = manifest.estimated_download_time(10.0); // 10 MB/s
        let time_slow = manifest.estimated_download_time(1.0);  // 1 MB/s
        
        assert!(time_fast < time_slow);
        assert!(time_fast > 0.0);
    }
}