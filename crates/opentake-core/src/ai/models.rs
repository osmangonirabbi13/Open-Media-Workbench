//! Canonical Model Data Contracts for Open Media Workbench AI infrastructure.

use serde::{Deserialize, Serialize};

/// Capability categories for AI models in Open Media Workbench.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiModelCapability {
    SpeechToText,
    TextGeneration,
    Translation,
    TextToSpeech,
    SpeechAlignment,
    SpeakerDetection,
    VisionAnalysis,
}

/// Operational status of an AI model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiModelStatus {
    NotInstalled,
    Installing,
    Installed,
    Verifying,
    Ready,
    Corrupted,
    Incompatible,
    Failed,
    Removing,
}

/// State of a model download job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiDownloadState {
    Queued,
    Downloading,
    Verifying,
    Complete,
    Failed,
    Cancelled,
}

/// Immutable metadata manifest describing a verified AI model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelManifest {
    pub id: String,
    pub name: String,
    pub family: String,
    pub version: String,
    pub capability: AiModelCapability,
    pub supported_languages: Vec<String>,
    pub runtime_backend: String,
    pub runtime_format: String,
    pub source_repository: String,
    pub artifact_url: Option<String>,
    pub artifact_filename: String,
    pub expected_size_bytes: u64,
    pub expected_sha256: Option<String>,
    pub license_name: String,
    pub license_url: String,
    pub model_card_url: String,
    pub min_ram_bytes: u64,
    pub cpu_compatible: bool,
    pub gpu_recommended: bool,
    pub installable: bool,
    pub description: String,
    pub schema_version: u32,
}

/// Persistent record of an installed AI model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInstallation {
    pub model_id: String,
    pub installed_at: String,
    pub installed_version: String,
    pub artifact_filename: String,
    pub actual_size_bytes: u64,
    pub verified_sha256: String,
    pub in_use: bool,
}

/// Current progress of an active model download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelDownloadProgress {
    pub model_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub progress_pct: f64,
    pub state: AiDownloadState,
    pub error: Option<String>,
}

/// System hardware and environment capabilities report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiHardwareInfo {
    pub os: String,
    pub arch: String,
    pub logical_cpus: usize,
    pub total_ram_bytes: Option<u64>,
    pub available_ram_bytes: Option<u64>,
    pub gpu_detected: bool,
    pub gpu_device_name: Option<String>,
    pub gpu_backend_installed: bool,
    pub gpu_runtime_usable: bool,
    pub storage_total_bytes: Option<u64>,
    pub storage_available_bytes: Option<u64>,
    pub supported_runtime_backends: Vec<String>,
}

/// Disk and cache usage for the managed AI model repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStorageUsage {
    pub base_dir: String,
    pub total_models_size_bytes: u64,
    pub installed_models_count: usize,
    pub staging_size_bytes: u64,
    pub available_disk_bytes: Option<u64>,
}

/// Combined model summary combining static manifest and live runtime status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelSummary {
    pub manifest: AiModelManifest,
    pub status: AiModelStatus,
    pub installation: Option<AiModelInstallation>,
    pub download_progress: Option<AiModelDownloadProgress>,
    pub runtime_available: bool,
}

/// Validates that a model ID contains only safe alphanumeric and delimiter characters.
pub fn validate_model_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    // Disallow path traversal patterns, slashes, backslashes, colons
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !id.contains("..")
        && !id.starts_with('.')
        && !id.ends_with('.')
}
