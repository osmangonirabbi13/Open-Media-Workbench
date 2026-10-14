use serde::{Deserialize, Serialize};

pub const RECOVERY_REPORT_SCHEMA_VERSION: u32 = 1;

/// Strategy/method selected for video recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryMethod {
    /// Losslessly copy readable elementary streams into a clean container without re-encoding.
    LosslessRemux,
    /// Isolate and extract intact video or audio streams when one or more streams are corrupted.
    StreamSalvage,
    /// Conservative timestamp regeneration (e.g. genpts, avoid_negative_ts) for timing anomalies.
    TimestampRebuild,
}

/// Processing lifecycle status of a recovery job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryStatus {
    Queued,
    Running,
    Validating,
    Completed,
    Partial,
    Failed,
    Cancelled,
}

/// Target stream selection when performing StreamSalvage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SalvageStreamType {
    /// Extract only the first readable video stream (drop audio).
    VideoOnly,
    /// Extract only the first readable audio stream (drop video).
    AudioOnly,
    /// Extract all readable streams while dropping corrupted tracks.
    AllReadable,
}

/// Depth of decode verification performed on output media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationLevel {
    /// Inspect container header, track presence, and duration without decoding frames.
    MetadataOnly,
    /// Bounded decode of initial and sampled frames using null-sink decode.
    SampledDecode,
    /// Full decode of entire file (applicable to short clips or deep verification).
    FullDecode,
}

/// Structured summary of a media stream in the source or recovered file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamReportInfo {
    pub index: usize,
    pub stream_type: String, // "video" | "audio" | "subtitle" | "unknown"
    pub codec_name: String,
    pub duration_seconds: Option<f64>,
    pub resolution: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
}

/// Outcome of post-recovery validation on the generated media file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryValidationResult {
    /// True if output passed minimal criteria (non-empty, valid container, playable stream).
    pub passed: bool,
    /// True if video/audio can be rendered by player.
    pub is_playable: bool,
    /// Depth of verification applied.
    pub validation_level: ValidationLevel,
    /// Descriptions of streams detected in the output file.
    pub streams_present: Vec<String>,
    /// Streams expected to be present based on the selected method.
    pub streams_expected: Vec<String>,
    /// True if any expected stream could not be preserved.
    pub stream_loss_detected: bool,
    /// Real duration discovered via FFprobe, if available.
    pub output_duration_seconds: Option<f64>,
    /// Explanatory summary of validation checks.
    pub details: String,
    /// Non-fatal warnings observed during inspection or decode.
    pub warnings: Vec<String>,
}

/// Client request to initiate a recovery job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRequest {
    /// Authorized asset ID in the active session.
    pub asset_id: String,
    /// Selected recovery method.
    pub method: RecoveryMethod,
    /// Desired output container extension (e.g. "mp4", "mkv", "mov", "avi"). Defaults to source or "mp4".
    pub output_container_preference: Option<String>,
    /// Stream target if salvage method is chosen.
    pub salvage_stream_type: Option<SalvageStreamType>,
    /// Optional custom destination path requested by the user.
    pub custom_destination: Option<String>,
    /// Optional reference to prior diagnostic report ID.
    pub diagnostic_report_id: Option<String>,
}

/// Canonical, versioned media recovery report detailing execution and validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReport {
    pub schema_version: u32,
    pub job_id: String,
    pub source_asset_id: String,
    pub source_file_name: String,
    pub recovery_method: RecoveryMethod,
    pub original_container: String,
    pub intended_output_container: String,
    pub actual_output_container: String,
    pub input_streams: Vec<StreamReportInfo>,
    pub output_streams: Vec<StreamReportInfo>,
    pub processing_status: RecoveryStatus,
    pub validation: RecoveryValidationResult,
    pub warnings: Vec<String>,
    pub observed_ffmpeg_errors: Vec<String>,
    pub output_file_size_bytes: Option<u64>,
    pub output_file_path: Option<String>,
    pub source_duration_seconds: Option<f64>,
    pub output_duration_seconds: Option<f64>,
    pub duration_difference_seconds: Option<f64>,
    pub started_at_epoch_ms: u64,
    pub completed_at_epoch_ms: u64,
    pub limitations: Vec<String>,
}
