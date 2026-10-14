use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::container::{identify_container, ContainerIdentificationResult};
use super::iso_bmff::{scan_iso_bmff_structure, IsoBmffScanResult};
use super::report::{
    AnalysisMode, AnalysisStatus, DetectedContainerInfo, DiagnosticFinding, DiagnosticReport,
    FindingCategory, InspectionCoverage, RepairRecommendation, Severity,
    DIAGNOSTIC_REPORT_SCHEMA_VERSION,
};
use super::streams::analyze_streams;
use super::timestamps::run_deep_decode_analysis;

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Runs a deterministic, read-only diagnostic analysis on an authorized media file.
///
/// Under no circumstances does this function modify the target media file.
/// If `cancellation` is triggered, the operation halts as soon as feasible and returns
/// a partial report with status `AnalysisStatus::Cancelled`.
pub async fn run_diagnostics(
    media_path: &Path,
    asset_id: &str,
    file_name: &str,
    mode: AnalysisMode,
    cancellation: Option<&CancellationToken>,
) -> DiagnosticReport {
    let started_at = current_epoch_ms();
    let analysis_id = format!("diag-{}", Uuid::new_v4());

    let mut all_findings: Vec<DiagnosticFinding> = Vec::new();

    // Check cancellation early
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            return build_cancelled_report(
                analysis_id,
                asset_id,
                file_name,
                mode,
                started_at,
                all_findings,
            );
        }
    }

    let total_file_bytes = std::fs::metadata(media_path).map(|m| m.len()).unwrap_or(0);

    // 1. Container & Signature inspection
    let container_res = match identify_container(media_path, total_file_bytes) {
        Ok(res) => res,
        Err(e) => {
            let finding = DiagnosticFinding {
                code: "CONTAINER_READ_IO_ERROR".to_string(),
                severity: Severity::Critical,
                category: FindingCategory::ContainerSignature,
                summary: "Failed to read file for container identification".to_string(),
                evidence: format!("I/O Error: {e}"),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: None,
                confidence: "Certain".to_string(),
                recommended_action: Some(
                    "Check storage accessibility and file permissions.".to_string(),
                ),
            };
            all_findings.push(finding);
            ContainerIdentificationResult {
                container_info: DetectedContainerInfo {
                    format_type: "Unreadable".to_string(),
                    extension: Path::new(file_name)
                        .extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    signature_matched: false,
                    signature_details: format!("Failed to open file: {e}"),
                    mismatch_warning: None,
                },
                findings: Vec::new(),
                is_iso_bmff: false,
                is_mkv: false,
                is_avi: false,
            }
        }
    };
    all_findings.extend(container_res.findings.clone());

    let mut header_scanned_bytes = 0u64;
    let mut box_count_found = 0usize;

    // Check cancellation
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            return build_cancelled_report(
                analysis_id,
                asset_id,
                file_name,
                mode,
                started_at,
                all_findings,
            );
        }
    }

    // 2. ISO BMFF structure inspection (for MP4 / MOV / 3GP / fragmented)
    let should_scan_iso_bmff = container_res.is_iso_bmff
        || container_res
            .container_info
            .extension
            .eq_ignore_ascii_case("mp4")
        || container_res
            .container_info
            .extension
            .eq_ignore_ascii_case("mov")
        || container_res
            .container_info
            .extension
            .eq_ignore_ascii_case("m4v");

    let mut iso_bmff_result = None;
    if should_scan_iso_bmff && total_file_bytes > 0 {
        match scan_iso_bmff_structure(media_path, total_file_bytes) {
            Ok(bmff_res) => {
                all_findings.extend(bmff_res.findings.clone());
                box_count_found = bmff_res.boxes.len();
                header_scanned_bytes = bmff_res.scanned_bytes;
                iso_bmff_result = Some(bmff_res);
            }
            Err(e) => {
                all_findings.push(DiagnosticFinding {
                    code: "MP4_SCAN_IO_ERROR".to_string(),
                    severity: Severity::Error,
                    category: FindingCategory::BoxStructure,
                    summary: "I/O error during ISO BMFF box scan".to_string(),
                    evidence: format!("File read failure: {e}"),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: None,
                    confidence: "Certain".to_string(),
                    recommended_action: None,
                });
            }
        }
    }

    // Check cancellation
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            return build_cancelled_report(
                analysis_id,
                asset_id,
                file_name,
                mode,
                started_at,
                all_findings,
            );
        }
    }

    // 3. Stream analysis via FFprobe
    let stream_result = analyze_streams(media_path, total_file_bytes, cancellation).await;
    all_findings.extend(stream_result.findings);

    // Check cancellation
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            return build_cancelled_report(
                analysis_id,
                asset_id,
                file_name,
                mode,
                started_at,
                all_findings,
            );
        }
    }

    // 4. Timestamp & decode check (Deep mode only)
    let mut decode_sampled = false;
    let mut decode_sample_frames = None;

    if mode == AnalysisMode::Deep && total_file_bytes > 0 {
        let decode_result = run_deep_decode_analysis(media_path, cancellation).await;
        all_findings.extend(decode_result.findings);
        decode_sampled = decode_result.decode_sampled;
        decode_sample_frames = decode_result.sample_duration_secs;
    }

    // Check cancellation one final time
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            return build_cancelled_report(
                analysis_id,
                asset_id,
                file_name,
                mode,
                started_at,
                all_findings,
            );
        }
    }

    // 5. Synthesize evidence-based recovery recommendations (Phase 8)
    let recommendations = synthesize_recommendations(
        &container_res,
        total_file_bytes,
        &iso_bmff_result,
        &all_findings,
    );

    let completed_at = current_epoch_ms();

    DiagnosticReport {
        schema_version: DIAGNOSTIC_REPORT_SCHEMA_VERSION,
        analysis_id,
        asset_id: asset_id.to_string(),
        file_name: file_name.to_string(),
        detected_container: container_res.container_info,
        analysis_mode: mode,
        started_at_epoch_ms: started_at,
        completed_at_epoch_ms: completed_at,
        analysis_status: AnalysisStatus::Completed,
        findings: all_findings,
        recommendations,
        inspection_coverage: InspectionCoverage {
            header_scanned_bytes,
            total_file_bytes,
            box_count_found,
            probe_executed: stream_result.probe_succeeded,
            decode_sampled,
            decode_sample_frames,
        },
        tool_versions: Some("FFmpeg/FFprobe integrated toolchain".to_string()),
    }
}

fn build_cancelled_report(
    analysis_id: String,
    asset_id: &str,
    file_name: &str,
    mode: AnalysisMode,
    started_at: u64,
    findings: Vec<DiagnosticFinding>,
) -> DiagnosticReport {
    DiagnosticReport {
        schema_version: DIAGNOSTIC_REPORT_SCHEMA_VERSION,
        analysis_id,
        asset_id: asset_id.to_string(),
        file_name: file_name.to_string(),
        detected_container: super::report::DetectedContainerInfo {
            format_type: "Unknown".to_string(),
            extension: Path::new(file_name)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default(),
            signature_matched: false,
            signature_details: "Analysis cancelled by user".to_string(),
            mismatch_warning: None,
        },
        analysis_mode: mode,
        started_at_epoch_ms: started_at,
        completed_at_epoch_ms: current_epoch_ms(),
        analysis_status: AnalysisStatus::Cancelled,
        findings,
        recommendations: Vec::new(),
        inspection_coverage: InspectionCoverage {
            header_scanned_bytes: 0,
            total_file_bytes: 0,
            box_count_found: 0,
            probe_executed: false,
            decode_sampled: false,
            decode_sample_frames: None,
        },
        tool_versions: None,
    }
}

/// Conservative, evidence-based recommendations synthesis (Phase 8).
fn synthesize_recommendations(
    container_res: &ContainerIdentificationResult,
    file_size: u64,
    iso_bmff: &Option<IsoBmffScanResult>,
    findings: &[DiagnosticFinding],
) -> Vec<RepairRecommendation> {
    let mut recs = Vec::new();

    if file_size == 0 {
        recs.push(RepairRecommendation {
            strategy_id: "INSPECT_FILE_SOURCE".to_string(),
            title: "Inspect File Source Integrity".to_string(),
            description: "The file is 0 bytes in length. No media data or container structure exists. Verify transfer from source camera or storage card.".to_string(),
            confidence: "Certain".to_string(),
            required_evidence: vec!["File size is exactly 0 bytes".to_string()],
            target_step: 12,
            available_now: true,
        });
        return recs;
    }

    let has_missing_moov = findings
        .iter()
        .any(|f| f.code == "MP4_MOOV_MISSING" || f.code == "PROBE_MOOV_ATOM_NOT_FOUND");

    let has_mdat = if let Some(bmff) = iso_bmff {
        bmff.has_mdat
    } else {
        findings.iter().any(|f| f.code == "CONTAINER_MDAT_PRESENT")
    };

    let has_corrupt_nals = findings
        .iter()
        .any(|f| f.code == "DECODE_BITSTREAM_NAL_CORRUPT");
    let has_timestamp_irregularities = findings
        .iter()
        .any(|f| f.code == "TIMESTAMP_NON_MONOTONIC_DTS");
    let has_truncated_box = findings.iter().any(|f| {
        f.code == "BOX_SIZE_TRUNCATED"
            || f.code == "MP4_HEADER_TRUNCATED_AT_OFFSET"
            || f.code == "AVI_RIFF_TRUNCATED"
    });

    // 1. Reference-based MP4 Reconstruction (STEP 14)
    if has_missing_moov && has_mdat {
        recs.push(RepairRecommendation {
            strategy_id: "REFERENCE_BASED_RECONSTRUCTION".to_string(),
            title: "Reference-Based MP4/MOV Reconstruction".to_string(),
            description: "The container media payload (mdat) is present, but the crucial index atom (moov) is missing (likely an unfinalized or interrupted recording). In STEP 14, a healthy donor file recorded on the same device can be used to synthesize a matching moov header and reconstruct track indexes.".to_string(),
            confidence: "High".to_string(),
            required_evidence: vec![
                "mdat payload atom detected".to_string(),
                "moov atom absent from container".to_string(),
            ],
            target_step: 14,
            available_now: false,
        });
    }

    // 2. Container Remux / Header Rebuild (STEP 13)
    if (has_timestamp_irregularities || has_truncated_box) && !has_missing_moov {
        recs.push(RepairRecommendation {
            strategy_id: "CONTAINER_REMUX".to_string(),
            title: "Lossless Container Remux".to_string(),
            description: "The audio/video elementary streams appear intact, but the outer container has timing irregularities or header truncation. Losslessly repackaging the streams into a freshly muxed MP4 or MKV container may resolve playback issues.".to_string(),
            confidence: "Moderate".to_string(),
            required_evidence: vec![
                "Valid stream headers detected".to_string(),
                "Container timing anomalies or trailing truncation observed".to_string(),
            ],
            target_step: 13,
            available_now: false,
        });
    }

    // 3. Raw Stream Salvage (STEP 13)
    if (has_corrupt_nals
        || (!container_res.is_iso_bmff && !container_res.is_mkv && !container_res.is_avi))
        && file_size > 1024 * 1024
    {
        recs.push(RepairRecommendation {
            strategy_id: "RAW_STREAM_SALVAGE".to_string(),
            title: "Raw Bitstream Salvage".to_string(),
            description: "The container headers are unrecognized or severely damaged, but the raw binary file contains substantial data. In STEP 13, packet carver algorithms can scan for raw H.264/H.265 NAL start codes or ADTS audio frames to salvage recoverable segments.".to_string(),
            confidence: "Tentative".to_string(),
            required_evidence: vec![
                "File size > 1MB with damaged/unknown container structure".to_string(),
            ],
            target_step: 13,
            available_now: false,
        });
    }

    // 4. If no defects detected
    let critical_or_errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical || f.severity == Severity::Error)
        .count();

    if critical_or_errors == 0 && recs.is_empty() {
        recs.push(RepairRecommendation {
            strategy_id: "NO_REPAIR_NEEDED".to_string(),
            title: "Media Appears Structurally Healthy".to_string(),
            description: "No container structural defects, missing index atoms, or severe bitstream errors were identified. The file should be playable directly.".to_string(),
            confidence: "High".to_string(),
            required_evidence: vec![
                "Valid container signature".to_string(),
                "All required top-level boxes present".to_string(),
                "Zero critical stream defects detected".to_string(),
            ],
            target_step: 12,
            available_now: true,
        });
    }

    recs
}
