pub mod container;
pub mod diagnostics;
pub mod iso_bmff;
pub mod recovery;
pub mod recovery_report;
pub mod remux;
pub mod report;
pub mod salvage;
pub mod streams;
pub mod timestamps;
pub mod validation;

pub use container::{identify_container, ContainerIdentificationResult};
pub use diagnostics::run_diagnostics;
pub use iso_bmff::{scan_iso_bmff_structure, IsoBmffScanResult, ScannedBox, MAX_TOP_LEVEL_BOXES};
pub use recovery::{finalize_recovered_file, run_recovery_pipeline};
pub use recovery_report::{
    RecoveryMethod, RecoveryReport, RecoveryRequest, RecoveryStatus, RecoveryValidationResult,
    SalvageStreamType, StreamReportInfo, ValidationLevel, RECOVERY_REPORT_SCHEMA_VERSION,
};
pub use remux::{execute_lossless_remux, RemuxExecutionResult};
pub use report::{
    AnalysisMode, AnalysisStatus, ByteRange, DetectedContainerInfo, DiagnosticFinding,
    DiagnosticReport, FindingCategory, InspectionCoverage, RepairRecommendation, Severity,
    DIAGNOSTIC_REPORT_SCHEMA_VERSION,
};
pub use salvage::{execute_stream_salvage, SalvageExecutionResult};
pub use streams::{analyze_streams, StreamAnalysisResult};
pub use timestamps::{run_deep_decode_analysis, DecodeInspectionResult};
pub use validation::{validate_recovery_output, OutputValidationOutcome};
