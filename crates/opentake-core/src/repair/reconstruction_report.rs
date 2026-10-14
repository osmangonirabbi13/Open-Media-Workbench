use serde::{Deserialize, Serialize};

use super::recovery_report::{RecoveryValidationResult, StreamReportInfo};

pub const RECONSTRUCTION_REPORT_SCHEMA_VERSION: u32 = 1;

/// Request payload to initiate reference-based MP4/MOV reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructionRequest {
    /// Authorized asset ID of the damaged recording
    pub damaged_asset_id: String,
    /// Authorized asset ID of the healthy reference recording
    pub reference_asset_id: String,
    /// Preferred container extension for output (default: "mp4")
    pub output_container_preference: Option<String>,
    /// Selected reconstruction engine ("native", "untrunc", or "auto")
    pub selected_engine: Option<String>,
    /// Optional associated diagnostic report ID
    pub diagnostic_report_id: Option<String>,
}

/// Classification of donor reference compatibility with the damaged recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferenceCompatibilityClassification {
    /// High confidence match (same container, matching codec profiles, matching stream layout)
    Compatible,
    /// Plausible match (container match, plausible codecs, missing moov prevents full verification)
    PlausiblyCompatible,
    /// Incompatible (mismatched container family, conflicting codecs, or invalid donor)
    Incompatible,
    /// Insufficient evidence to establish compatibility
    InsufficientEvidence,
}

/// Track inventory detail extracted from the healthy reference recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceTrackInfo {
    pub track_id: u32,
    pub media_type: String,
    pub codec_name: String,
    pub codec_profile: Option<String>,
    pub resolution: Option<String>,
    pub frame_rate: Option<f64>,
    pub timebase: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub extradata_size: usize,
}

/// Compatibility analysis report between a healthy reference file and a damaged recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCompatibilityReport {
    pub schema_version: u32,
    pub reference_asset_id: String,
    pub damaged_asset_id: String,
    pub reference_container: String,
    pub damaged_container: String,
    pub tracks: Vec<ReferenceTrackInfo>,
    pub compatibility: ReferenceCompatibilityClassification,
    pub summary: String,
    pub warnings: Vec<String>,
    pub unknown_fields: Vec<String>,
}

/// Real-time progression lifecycle of a reconstruction job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReconstructionStatus {
    Queued,
    CheckingReference,
    Analyzing,
    Reconstructing,
    Validating,
    Completed,
    Partial,
    Failed,
    Cancelled,
}

/// Evidence-based audit report generated upon completion of a reconstruction job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructionReport {
    pub schema_version: u32,
    pub operation_id: String,
    pub engine_name: String,
    pub engine_version: String,
    pub damaged_asset_id: String,
    pub reference_asset_id: String,
    pub damaged_file_name: String,
    pub reference_file_name: String,
    pub reference_compatibility: ReferenceCompatibilityReport,
    pub attempted_mode: String,
    pub reconstructed_streams: Vec<StreamReportInfo>,
    pub recovered_duration_seconds: Option<f64>,
    pub validation: RecoveryValidationResult,
    pub output_file_size_bytes: Option<u64>,
    pub output_file_path: Option<String>,
    pub processing_status: ReconstructionStatus,
    pub failure_details: Option<String>,
    pub warnings: Vec<String>,
    pub uncertainties: Vec<String>,
    pub started_at_epoch_ms: u64,
    pub completed_at_epoch_ms: u64,
}
