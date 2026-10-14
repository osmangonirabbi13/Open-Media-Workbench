use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use super::report::{DiagnosticFinding, FindingCategory, Severity};
use crate::media::discovery::{discover_tool, DiscoveryConfig, ToolType};
use crate::media::process::{run_managed_process, ProcessSpec};

/// Maximum duration of media to decode during deep scan (seconds).
pub const DEEP_DECODE_SAMPLE_DURATION_SECS: u64 = 30;

/// Process execution timeout for deep decode check.
pub const DEEP_DECODE_TIMEOUT: Duration = Duration::from_secs(45);

/// Maximum stderr bytes to capture from ffmpeg decode to prevent memory issues.
pub const MAX_DECODE_STDERR_BYTES: usize = 256 * 1024;

/// Result of timestamp and decode inspection.
#[derive(Debug, Clone)]
pub struct DecodeInspectionResult {
    pub findings: Vec<DiagnosticFinding>,
    pub decode_sampled: bool,
    pub sample_duration_secs: Option<u64>,
}

/// Runs bounded decode and timestamp continuity analysis using FFmpeg.
///
/// This does NOT touch or modify the original media file. It decodes a bounded window
/// (up to `DEEP_DECODE_SAMPLE_DURATION_SECS`) into null sink (`-f null -`) to identify
/// bitstream corruptions, macroblock errors, corrupted NAL units, and timestamp anomalies.
pub async fn run_deep_decode_analysis(
    media_path: &Path,
    cancellation: Option<&CancellationToken>,
) -> DecodeInspectionResult {
    let mut findings = Vec::new();

    // 1. Discover ffmpeg executable
    let ffmpeg = match discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default()) {
        Ok(discovered) => discovered.canonical_path,
        Err(_) => {
            findings.push(DiagnosticFinding {
                code: "TOOL_FFMPEG_NOT_FOUND".to_string(),
                severity: Severity::Warning,
                category: FindingCategory::DecodeSample,
                summary: "FFmpeg binary unavailable for deep decode inspection".to_string(),
                evidence: "ffmpeg executable could not be discovered in system paths".to_string(),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: None,
                confidence: "High".to_string(),
                recommended_action: Some(
                    "Ensure ffmpeg is installed to enable deep bitstream verification".to_string(),
                ),
            });
            return DecodeInspectionResult {
                findings,
                decode_sampled: false,
                sample_duration_secs: None,
            };
        }
    };

    // 2. Prepare managed ffmpeg command:
    // ffmpeg -v warning -xerror -t 30 -i <file> -f null -
    // We use -v warning to capture both corruptions and severe warnings without drowning in noise.
    let args = vec![
        "-nostdin".to_string(),
        "-hide_banner".to_string(),
        "-v".to_string(),
        "warning".to_string(),
        "-t".to_string(),
        DEEP_DECODE_SAMPLE_DURATION_SECS.to_string(),
        "-i".to_string(),
        media_path.to_string_lossy().to_string(),
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ];

    let mut spec = ProcessSpec::new(ffmpeg, args, DEEP_DECODE_TIMEOUT);
    spec.max_stderr_bytes = MAX_DECODE_STDERR_BYTES;

    let output = match run_managed_process(spec, cancellation).await {
        Ok(out) => out,
        Err(err) => {
            findings.push(DiagnosticFinding {
                code: "DECODE_PROCESS_EXECUTION_FAILED".to_string(),
                severity: Severity::Warning,
                category: FindingCategory::DecodeSample,
                summary: "Decode validation process failed to complete".to_string(),
                evidence: format!("Process runner error: {err}"),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: None,
                confidence: "High".to_string(),
                recommended_action: Some(
                    "Check file read permissions and system resources".to_string(),
                ),
            });
            return DecodeInspectionResult {
                findings,
                decode_sampled: false,
                sample_duration_secs: None,
            };
        }
    };

    // Parse stderr output lines for specific decoder errors
    let stderr_text = output.stderr;
    let mut non_monotonic_dts_count = 0usize;
    let mut nal_corrupt_count = 0usize;
    let mut macroblock_error_count = 0usize;
    let mut missing_reference_count = 0usize;
    let mut generic_error_lines = Vec::new();

    for line in stderr_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();

        if lower.contains("non-monotonic")
            || lower.contains("dts") && lower.contains("out of order")
        {
            non_monotonic_dts_count += 1;
        } else if lower.contains("invalid nal")
            || lower.contains("nal_unit_type")
            || lower.contains("missing pps")
            || lower.contains("missing sps")
        {
            nal_corrupt_count += 1;
        } else if lower.contains("concealing")
            || lower.contains("error while decoding mb")
            || lower.contains("macroblock")
        {
            macroblock_error_count += 1;
        } else if lower.contains("missing reference picture") || lower.contains("co located poc") {
            missing_reference_count += 1;
        } else if (lower.contains("error")
            || lower.contains("corrupt")
            || lower.contains("invalid data"))
            && generic_error_lines.len() < 5
        {
            generic_error_lines.push(trimmed.to_string());
        }
    }

    if non_monotonic_dts_count > 0 {
        findings.push(DiagnosticFinding {
            code: "TIMESTAMP_NON_MONOTONIC_DTS".to_string(),
            severity: Severity::Error,
            category: FindingCategory::TimestampContinuity,
            summary: format!("Detected {non_monotonic_dts_count} non-monotonic DTS timestamp irregularities"),
            evidence: format!("Decoder reported {non_monotonic_dts_count} timestamp backward jumps or disorder instances in first {DEEP_DECODE_SAMPLE_DURATION_SECS}s"),
            affected_stream: None,
            media_time_range_ms: Some((0, DEEP_DECODE_SAMPLE_DURATION_SECS * 1000)),
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some("Stream remux or timestamp re-indexing will be required to prevent playback stutter".to_string()),
        });
    }

    if nal_corrupt_count > 0 {
        findings.push(DiagnosticFinding {
            code: "DECODE_BITSTREAM_NAL_CORRUPT".to_string(),
            severity: Severity::Critical,
            category: FindingCategory::DecodeSample,
            summary: format!("Detected {nal_corrupt_count} corrupted bitstream NAL units or missing SPS/PPS headers"),
            evidence: format!("Decoder reported {nal_corrupt_count} NAL parsing failures in sample decode"),
            affected_stream: None,
            media_time_range_ms: Some((0, DEEP_DECODE_SAMPLE_DURATION_SECS * 1000)),
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some("Video stream payload contains truncated or bit-flipped slices; stream salvage or frame drop recovery required".to_string()),
        });
    }

    if macroblock_error_count > 0 {
        findings.push(DiagnosticFinding {
            code: "DECODE_MACROBLOCK_CONCEALMENT".to_string(),
            severity: Severity::Error,
            category: FindingCategory::DecodeSample,
            summary: format!("Decoder triggered error concealment for {macroblock_error_count} damaged macroblocks"),
            evidence: format!("Decoder reported concealment on {macroblock_error_count} damaged blocks"),
            affected_stream: None,
            media_time_range_ms: Some((0, DEEP_DECODE_SAMPLE_DURATION_SECS * 1000)),
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some("Visual artifacts present in stream; frames may suffer tearing or block corruption".to_string()),
        });
    }

    if missing_reference_count > 0 {
        findings.push(DiagnosticFinding {
            code: "DECODE_MISSING_REFERENCE_FRAMES".to_string(),
            severity: Severity::Warning,
            category: FindingCategory::DecodeSample,
            summary: format!("Encountered {missing_reference_count} missing reference frames (broken GOP structure)"),
            evidence: format!("Decoder reported missing reference picture {missing_reference_count} times"),
            affected_stream: None,
            media_time_range_ms: Some((0, DEEP_DECODE_SAMPLE_DURATION_SECS * 1000)),
            byte_range: None,
            confidence: "Moderate".to_string(),
            recommended_action: Some("Broken intra-frame dependencies; GOP reconstruction may be necessary".to_string()),
        });
    }

    for err_line in generic_error_lines {
        findings.push(DiagnosticFinding {
            code: "DECODE_SAMPLE_ERROR".to_string(),
            severity: Severity::Error,
            category: FindingCategory::DecodeSample,
            summary: "Bitstream decode error encountered during deep scan".to_string(),
            evidence: err_line,
            affected_stream: None,
            media_time_range_ms: Some((0, DEEP_DECODE_SAMPLE_DURATION_SECS * 1000)),
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some(
                "Review decode logs for specific codec corruption details".to_string(),
            ),
        });
    }

    // If exit code is non-zero and no specific findings parsed yet
    if output.exit_code.unwrap_or(0) != 0 && findings.is_empty() {
        findings.push(DiagnosticFinding {
            code: "DECODE_PROCESS_EXIT_ERROR".to_string(),
            severity: Severity::Error,
            category: FindingCategory::DecodeSample,
            summary: format!(
                "Decoder exited with non-zero status code: {:?}",
                output.exit_code
            ),
            evidence: if stderr_text.trim().is_empty() {
                "Decoder exited with error but standard error stream was empty".to_string()
            } else {
                stderr_text.lines().take(3).collect::<Vec<_>>().join("; ")
            },
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some(
                "Check whether codec is supported or file was prematurely truncated".to_string(),
            ),
        });
    }

    // If completely clean
    if findings.is_empty() && output.exit_code == Some(0) {
        findings.push(DiagnosticFinding {
            code: "DECODE_SAMPLE_CLEAN".to_string(),
            severity: Severity::Info,
            category: FindingCategory::DecodeSample,
            summary: format!("Sample decode of first {DEEP_DECODE_SAMPLE_DURATION_SECS}s completed with zero decoder errors"),
            evidence: "FFmpeg returned exit code 0 with clean error log during test decode".to_string(),
            affected_stream: None,
            media_time_range_ms: Some((0, DEEP_DECODE_SAMPLE_DURATION_SECS * 1000)),
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: None,
        });
    }

    DecodeInspectionResult {
        findings,
        decode_sampled: true,
        sample_duration_secs: Some(DEEP_DECODE_SAMPLE_DURATION_SECS),
    }
}
