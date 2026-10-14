use std::path::Path;
use tokio_util::sync::CancellationToken;

use crate::media::error::MediaEngineError;
use crate::media::metadata::MediaMetadata;
use crate::media::probe::probe_media_file;

use super::report::{DiagnosticFinding, FindingCategory, Severity};

/// Results of stream probing and inventory inspection.
#[derive(Debug, Clone)]
pub struct StreamAnalysisResult {
    pub metadata: Option<MediaMetadata>,
    pub findings: Vec<DiagnosticFinding>,
    pub probe_succeeded: bool,
    pub video_count: usize,
    pub audio_count: usize,
}

/// Runs FFprobe to inspect streams, handling probe failures gracefully without crashing.
pub async fn analyze_streams(
    path: &Path,
    file_size_bytes: u64,
    cancellation: Option<&CancellationToken>,
) -> StreamAnalysisResult {
    let mut findings = Vec::new();

    match probe_media_file(None, path, file_size_bytes, cancellation).await {
        Ok(metadata) => {
            let video_count = metadata.video_streams.len();
            let audio_count = metadata.audio_streams.len();

            // 1. Check for stream presence
            if video_count == 0 && audio_count == 0 {
                findings.push(DiagnosticFinding {
                    code: "STREAM_NO_TRACKS_DISCOVERED".to_string(),
                    severity: Severity::Critical,
                    category: FindingCategory::StreamIntegrity,
                    summary: "Container contains zero playable audio or video streams".to_string(),
                    evidence: "FFprobe probed container successfully, but detected zero audio and zero video streams.".to_string(),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: None,
                    confidence: "Certain".to_string(),
                    recommended_action: Some("File may be an empty container or unsupported stream format.".to_string()),
                });
            } else if video_count == 0 {
                findings.push(DiagnosticFinding {
                    code: "STREAM_AUDIO_ONLY".to_string(),
                    severity: Severity::Info,
                    category: FindingCategory::StreamIntegrity,
                    summary: "File contains audio streams only (no video)".to_string(),
                    evidence: format!("{audio_count} audio stream(s) found; 0 video streams."),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: None,
                    confidence: "Certain".to_string(),
                    recommended_action: None,
                });
            } else if audio_count == 0 {
                findings.push(DiagnosticFinding {
                    code: "STREAM_VIDEO_SILENT".to_string(),
                    severity: Severity::Info,
                    category: FindingCategory::StreamIntegrity,
                    summary: "File contains video streams only (silent recording, no audio track)"
                        .to_string(),
                    evidence: format!("{video_count} video stream(s) found; 0 audio streams."),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: None,
                    confidence: "Certain".to_string(),
                    recommended_action: None,
                });
            }

            // 2. Check video streams for dimension and framerate validity
            for (idx, vs) in metadata.video_streams.iter().enumerate() {
                let stream_id = format!("video:{}", vs.stream_index);

                if vs.width == 0 || vs.height == 0 {
                    findings.push(DiagnosticFinding {
                        code: "STREAM_VIDEO_ZERO_DIMENSIONS".to_string(),
                        severity: Severity::Critical,
                        category: FindingCategory::StreamIntegrity,
                        summary: format!(
                            "Video stream #{idx} has invalid zero dimensions ({}x{})",
                            vs.width, vs.height
                        ),
                        evidence: format!(
                            "Header reports width={} height={}.",
                            vs.width, vs.height
                        ),
                        affected_stream: Some(stream_id.clone()),
                        media_time_range_ms: None,
                        byte_range: None,
                        confidence: "Certain".to_string(),
                        recommended_action: Some(
                            "Corrupted Sequence Parameter Set (SPS).".to_string(),
                        ),
                    });
                }

                if vs.nominal_frame_rate.is_none() && vs.average_frame_rate.is_none() {
                    findings.push(DiagnosticFinding {
                        code: "STREAM_FRAMERATE_UNKNOWN".to_string(),
                        severity: Severity::Warning,
                        category: FindingCategory::StreamIntegrity,
                        summary: format!("Video stream #{idx} framerate could not be parsed"),
                        evidence: "Neither average nor nominal frame rate fractions could be derived from container headers.".to_string(),
                        affected_stream: Some(stream_id.clone()),
                        media_time_range_ms: None,
                        byte_range: None,
                        confidence: "High".to_string(),
                        recommended_action: Some("May require CFR transcode or explicit timebase specification.".to_string()),
                    });
                }

                if vs.is_vfr_suspect {
                    findings.push(DiagnosticFinding {
                        code: "STREAM_VFR_SUSPECTED".to_string(),
                        severity: Severity::Warning,
                        category: FindingCategory::TimestampContinuity,
                        summary: format!("Video stream #{idx} appears to use variable frame rate (VFR)"),
                        evidence: "Average frame rate differs significantly from nominal frame rate.".to_string(),
                        affected_stream: Some(stream_id),
                        media_time_range_ms: None,
                        byte_range: None,
                        confidence: "Medium".to_string(),
                        recommended_action: Some("Non-constant frame rates can cause audio/video drift during timeline editing.".to_string()),
                    });
                }
            }

            // 3. Check duration
            if metadata.container.duration_micros.is_none()
                || metadata.container.duration_micros == Some(0)
            {
                findings.push(DiagnosticFinding {
                    code: "CONTAINER_DURATION_UNKNOWN".to_string(),
                    severity: Severity::Warning,
                    category: FindingCategory::StreamIntegrity,
                    summary: "Container duration is missing or reported as zero".to_string(),
                    evidence: "Container header lacks valid duration attribute; timeline duration must be determined by decoding.".to_string(),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: None,
                    confidence: "High".to_string(),
                    recommended_action: Some("Live recordings or unfinalized files often lack header duration.".to_string()),
                });
            }

            StreamAnalysisResult {
                metadata: Some(metadata),
                findings,
                probe_succeeded: true,
                video_count,
                audio_count,
            }
        }
        Err(err) => {
            let error_msg = match &err {
                MediaEngineError::NonZeroExit { stderr, .. } => stderr.clone(),
                other => other.to_string(),
            };

            // Evaluate specific failure patterns
            let (code, summary, severity) = if error_msg.contains("moov atom not found") {
                (
                    "PROBE_MOOV_ATOM_NOT_FOUND".to_string(),
                    "FFprobe failed: 'moov atom not found' in MP4/MOV container".to_string(),
                    Severity::Critical,
                )
            } else if error_msg.contains("Invalid data found") {
                (
                    "PROBE_INVALID_DATA_STREAM".to_string(),
                    "FFprobe rejected file: invalid or unrecognized media data".to_string(),
                    Severity::Critical,
                )
            } else if error_msg.contains("EBML") {
                (
                    "PROBE_MKV_EBML_HEADER_INVALID".to_string(),
                    "FFprobe failed to parse Matroska EBML structure".to_string(),
                    Severity::Error,
                )
            } else {
                (
                    "PROBE_METADATA_EXTRACTION_FAILED".to_string(),
                    "FFprobe could not extract stream metadata from container".to_string(),
                    Severity::Error,
                )
            };

            findings.push(DiagnosticFinding {
                code,
                severity,
                category: FindingCategory::StreamIntegrity,
                summary,
                evidence: format!("FFprobe process reported error: {error_msg}"),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: None,
                confidence: "Certain".to_string(),
                recommended_action: Some(
                    "Container metadata is inaccessible via standard parser. Structural repair required.".to_string()
                ),
            });

            StreamAnalysisResult {
                metadata: None,
                findings,
                probe_succeeded: false,
                video_count: 0,
                audio_count: 0,
            }
        }
    }
}
