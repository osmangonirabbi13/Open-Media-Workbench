pub mod container;
pub mod diagnostics;
pub mod iso_bmff;
pub mod report;
pub mod streams;
pub mod timestamps;

pub use container::{identify_container, ContainerIdentificationResult};
pub use diagnostics::run_diagnostics;
pub use iso_bmff::{scan_iso_bmff_structure, IsoBmffScanResult, ScannedBox, MAX_TOP_LEVEL_BOXES};
pub use report::{
    AnalysisMode, AnalysisStatus, ByteRange, DetectedContainerInfo, DiagnosticFinding,
    DiagnosticReport, FindingCategory, InspectionCoverage, RepairRecommendation, Severity,
    DIAGNOSTIC_REPORT_SCHEMA_VERSION,
};
pub use streams::{analyze_streams, StreamAnalysisResult};
pub use timestamps::{run_deep_decode_analysis, DecodeInspectionResult};
