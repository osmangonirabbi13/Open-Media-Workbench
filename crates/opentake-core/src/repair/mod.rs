pub mod container;
pub mod diagnostics;
pub mod history;
pub mod iso_bmff;
pub mod reconstruction;
pub mod reconstruction_report;
pub mod recovery;
pub mod recovery_report;
pub mod recovery_review;
pub mod reference;
pub mod remux;
pub mod report;
pub mod reporting;
pub mod salvage;
pub mod streams;
pub mod timestamps;
pub mod validation;

pub use container::{identify_container, ContainerIdentificationResult};
pub use diagnostics::run_diagnostics;
pub use history::{
    RecoveryHistoryEntry, RecoveryHistoryStore, MAX_HISTORY_ENTRIES,
    RECOVERY_HISTORY_SCHEMA_VERSION,
};
pub use iso_bmff::{scan_iso_bmff_structure, IsoBmffScanResult, ScannedBox, MAX_TOP_LEVEL_BOXES};
pub use reconstruction::{
    discover_reconstruction_capabilities, run_reconstruction_pipeline, EngineExecutionEvidence,
    ReconstructionEngineCapability,
};
pub use reconstruction_report::{
    ReconstructionReport, ReconstructionRequest, ReconstructionStatus,
    ReferenceCompatibilityClassification, ReferenceCompatibilityReport, ReferenceTrackInfo,
    RECONSTRUCTION_REPORT_SCHEMA_VERSION,
};
pub use recovery::{finalize_recovered_file, run_recovery_pipeline};
pub use recovery_report::{
    RecoveryMethod, RecoveryReport, RecoveryRequest, RecoveryStatus, RecoveryValidationResult,
    SalvageStreamType, StreamReportInfo, ValidationLevel, RECOVERY_REPORT_SCHEMA_VERSION,
};
pub use recovery_review::{
    AssetMediaSummary, CheckItem, DurationComparison, RecoveryReview, TimelineEvidence,
    ValidationOutcome, ValidationSummary, RECOVERY_REVIEW_SCHEMA_VERSION,
};
pub use reference::analyze_reference_compatibility;
pub use remux::{execute_lossless_remux, RemuxExecutionResult};
pub use report::{
    AnalysisMode, AnalysisStatus, ByteRange, DetectedContainerInfo, DiagnosticFinding,
    DiagnosticReport, FindingCategory, InspectionCoverage, RepairRecommendation, Severity,
    DIAGNOSTIC_REPORT_SCHEMA_VERSION,
};
pub use reporting::{
    generate_recovery_html_report, generate_recovery_json_report, html_escape,
    sanitize_privacy_string,
};
pub use salvage::{execute_stream_salvage, SalvageExecutionResult};
pub use streams::{analyze_streams, StreamAnalysisResult};
pub use timestamps::{run_deep_decode_analysis, DecodeInspectionResult};
pub use validation::{validate_recovery_output, OutputValidationOutcome};
