use serde::{Deserialize, Serialize};

/// Mode of diagnostics executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisMode {
    /// Fast structural and signature scan + stream metadata probing.
    Quick,
    /// Quick scan + bounded packet/timestamp decode validation.
    Deep,
}

/// Completion state of the diagnostic job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisStatus {
    Completed,
    Partial,
    Cancelled,
    Failed,
}

/// Severity classification of a diagnostic finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Category grouping for diagnostic findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FindingCategory {
    ContainerSignature,
    BoxStructure,
    StreamIntegrity,
    TimestampContinuity,
    DecodeSample,
}

/// Byte range in the media file associated with a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByteRange {
    pub start: u64,
    pub length: u64,
}

/// Structured, evidence-backed diagnostic finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticFinding {
    pub code: String,
    pub severity: Severity,
    pub category: FindingCategory,
    pub summary: String,
    pub evidence: String,
    pub affected_stream: Option<String>,
    pub media_time_range_ms: Option<(u64, u64)>,
    pub byte_range: Option<ByteRange>,
    pub confidence: String,
    pub recommended_action: Option<String>,
}

/// A prospective future recovery recommendation based on observed evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairRecommendation {
    pub strategy_id: String,
    pub title: String,
    pub description: String,
    pub confidence: String,
    pub required_evidence: Vec<String>,
    pub target_step: u32,
    pub available_now: bool,
}

/// Metrics on how thoroughly the media file was inspected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionCoverage {
    pub header_scanned_bytes: u64,
    pub total_file_bytes: u64,
    pub box_count_found: usize,
    pub probe_executed: bool,
    pub decode_sampled: bool,
    pub decode_sample_frames: Option<u64>,
}

/// Details about the container type identified from signatures and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedContainerInfo {
    pub format_type: String,
    pub extension: String,
    pub signature_matched: bool,
    pub signature_details: String,
    pub mismatch_warning: Option<String>,
}

/// Canonical versioned diagnostic report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub schema_version: u32,
    pub analysis_id: String,
    pub asset_id: String,
    pub file_name: String,
    pub detected_container: DetectedContainerInfo,
    pub analysis_mode: AnalysisMode,
    pub started_at_epoch_ms: u64,
    pub completed_at_epoch_ms: u64,
    pub analysis_status: AnalysisStatus,
    pub findings: Vec<DiagnosticFinding>,
    pub recommendations: Vec<RepairRecommendation>,
    pub inspection_coverage: InspectionCoverage,
    pub tool_versions: Option<String>,
}

pub const DIAGNOSTIC_REPORT_SCHEMA_VERSION: u32 = 1;
