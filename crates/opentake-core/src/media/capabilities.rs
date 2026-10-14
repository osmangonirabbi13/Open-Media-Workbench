use serde::{Deserialize, Serialize};

/// High-level compatibility verdict of the local host's media processing toolchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaEngineCompatibility {
    /// Both FFmpeg and FFprobe are discovered, verified, and operational.
    Ready,
    /// Tools are operational but lack recommended capabilities (e.g. libx264 missing).
    Degraded,
    /// Tool versions are below minimum requirements or incompatible.
    Unsupported,
    /// One or both required binaries are missing from the system.
    Missing,
    /// An unrecoverable error occurred during toolchain evaluation.
    Error,
}

/// Diagnostic inspection details for a single tool binary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    pub executable_path: Option<String>,
    pub canonical_path: Option<String>,
    pub version: Option<String>,
    pub major_version: Option<u32>,
    pub is_gpl: bool,
    pub has_libx264: bool,
    pub has_libopus: bool,
    pub error: Option<String>,
}

/// Consolidated authoritative status response for OpenTake media engine diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaEngineStatus {
    pub overall_status: MediaEngineCompatibility,
    pub ffmpeg: ToolStatus,
    pub ffprobe: ToolStatus,
    pub warnings: Vec<String>,
    pub evaluated_at_epoch_ms: u64,
}
