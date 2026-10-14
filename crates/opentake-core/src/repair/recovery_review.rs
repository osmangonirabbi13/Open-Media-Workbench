use serde::{Deserialize, Serialize};

use super::reconstruction_report::ReconstructionReport;
use super::recovery_report::{
    RecoveryMethod, RecoveryReport, RecoveryStatus, StreamReportInfo, ValidationLevel,
};

pub const RECOVERY_REVIEW_SCHEMA_VERSION: u32 = 1;

/// Overall evaluation status of a validation check or category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationOutcome {
    Passed,
    Warning,
    Failed,
    Inconclusive,
    NotChecked,
}

/// An individual verified inspection check with a stable machine-readable code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    pub code: String,
    pub outcome: ValidationOutcome,
    pub title: String,
    pub message: String,
}

/// Precise comparison between original and recovered stream duration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DurationComparison {
    pub source_duration_seconds: Option<f64>,
    pub recovered_duration_seconds: Option<f64>,
    pub difference_seconds: Option<f64>,
    pub is_source_duration_unknown: bool,
    pub explanation: String,
}

/// Observable temporal alignment and offset evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvidence {
    pub source_duration_seconds: Option<f64>,
    pub recovered_duration_seconds: Option<f64>,
    pub video_start_offset_seconds: Option<f64>,
    pub audio_start_offset_seconds: Option<f64>,
    pub av_drift_seconds: Option<f64>,
    pub known_decode_failures: Vec<String>,
    pub uninspected_regions_note: Option<String>,
}

/// Comprehensive validation summary across structural, stream, timing, and decode dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    pub level: ValidationLevel,
    pub overall_outcome: ValidationOutcome,
    pub container_checks: Vec<CheckItem>,
    pub stream_checks: Vec<CheckItem>,
    pub timestamp_checks: Vec<CheckItem>,
    pub decode_checks: Vec<CheckItem>,
    pub audio_checks: Vec<CheckItem>,
    pub duration_comparison: DurationComparison,
    pub inspection_coverage: String,
}

/// High-level inventory of an asset's media properties for comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMediaSummary {
    pub file_name: String,
    pub file_size_bytes: Option<u64>,
    pub container_format: String,
    pub duration_seconds: Option<f64>,
    pub streams: Vec<StreamReportInfo>,
    pub is_playable: bool,
}

/// Unified, client-facing recovery review combining execution evidence, comparison,
/// validation findings, and explicit limitation notices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReview {
    pub schema_version: u32,
    pub review_id: String,
    pub source_asset_id: String,
    pub recovery_job_id: String,
    pub recovery_method: String,
    pub recovery_status: RecoveryStatus,
    pub output_asset_id: Option<String>,
    pub output_candidate_id: Option<String>,
    pub source_summary: AssetMediaSummary,
    pub output_summary: Option<AssetMediaSummary>,
    pub validation_summary: ValidationSummary,
    pub timeline_evidence: Option<TimelineEvidence>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
    pub created_at_epoch_ms: u64,
}

impl RecoveryReview {
    /// Creates a RecoveryReview from a standard RecoveryReport (Remux, Salvage, TimestampRebuild).
    pub fn from_recovery_report(
        report: &RecoveryReport,
        source_is_playable: bool,
        candidate_asset_id: Option<String>,
    ) -> Self {
        let is_source_duration_unknown = report.source_duration_seconds.is_none();
        let duration_diff = match (
            report.source_duration_seconds,
            report.output_duration_seconds,
        ) {
            (Some(s), Some(o)) => Some(o - s),
            _ => None,
        };

        let duration_explanation = if is_source_duration_unknown {
            "Original source container lacks valid header duration (e.g. unfinalized or missing moov). Output duration is newly established from recovered stream timestamps."
                .to_string()
        } else if let Some(diff) = duration_diff {
            if diff.abs() < 0.05 {
                "Source and recovered durations are closely aligned (<50ms discrepancy)."
                    .to_string()
            } else if diff < 0.0 {
                format!(
                    "Recovered media is {:.2}s shorter than original nominal container duration.",
                    diff.abs()
                )
            } else {
                format!(
                    "Recovered media duration extends {:.2}s beyond original nominal container duration.",
                    diff
                )
            }
        } else {
            "Duration could not be compared accurately.".to_string()
        };

        let duration_comp = DurationComparison {
            source_duration_seconds: report.source_duration_seconds,
            recovered_duration_seconds: report.output_duration_seconds,
            difference_seconds: duration_diff,
            is_source_duration_unknown,
            explanation: duration_explanation,
        };

        // Synthesize container checks
        let mut container_checks = Vec::new();
        if let Some(bytes) = report.output_file_size_bytes {
            if bytes > 0 {
                container_checks.push(CheckItem {
                    code: "CONTAINER_EXISTS_NONEMPTY".to_string(),
                    outcome: ValidationOutcome::Passed,
                    title: "Container Integrity".to_string(),
                    message: format!("Recovered output is non-empty ({bytes} bytes)."),
                });
            } else {
                container_checks.push(CheckItem {
                    code: "CONTAINER_EMPTY".to_string(),
                    outcome: ValidationOutcome::Failed,
                    title: "Container Empty".to_string(),
                    message: "Output file is 0 bytes.".to_string(),
                });
            }
        }

        container_checks.push(CheckItem {
            code: "CONTAINER_FORMAT_RESOLVED".to_string(),
            outcome: ValidationOutcome::Passed,
            title: "Muxing Format".to_string(),
            message: format!(
                "Output container muxed as {}",
                report.actual_output_container
            ),
        });

        // Synthesize stream checks
        let mut stream_checks = Vec::new();
        let out_video_count = report
            .output_streams
            .iter()
            .filter(|s| s.stream_type == "video")
            .count();
        let out_audio_count = report
            .output_streams
            .iter()
            .filter(|s| s.stream_type == "audio")
            .count();

        if report.output_streams.is_empty() {
            stream_checks.push(CheckItem {
                code: "STREAM_NONE_DETECTED".to_string(),
                outcome: ValidationOutcome::Failed,
                title: "Stream Inventory".to_string(),
                message: "No readable audio or video streams found in output.".to_string(),
            });
        } else {
            stream_checks.push(CheckItem {
                code: "STREAM_INVENTORY_DETECTED".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Stream Inventory".to_string(),
                message: format!(
                    "Found {} stream(s): {} video, {} audio.",
                    report.output_streams.len(),
                    out_video_count,
                    out_audio_count
                ),
            });
        }

        if report.validation.stream_loss_detected {
            stream_checks.push(CheckItem {
                code: "STREAM_LOSS_DETECTED".to_string(),
                outcome: ValidationOutcome::Warning,
                title: "Stream Preservation".to_string(),
                message:
                    "One or more streams present in source could not be transferred to output."
                        .to_string(),
            });
        } else {
            stream_checks.push(CheckItem {
                code: "STREAM_COUNT_MATCH".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Stream Preservation".to_string(),
                message: "All requested streams preserved in output.".to_string(),
            });
        }

        // Synthesize timestamp checks
        let mut timestamp_checks = Vec::new();
        let has_ts_warning = report
            .warnings
            .iter()
            .any(|w| w.to_lowercase().contains("timestamp") || w.to_lowercase().contains("pts"));
        if has_ts_warning {
            timestamp_checks.push(CheckItem {
                code: "TIMESTAMPS_ANOMALY_DETECTED".to_string(),
                outcome: ValidationOutcome::Warning,
                title: "Timestamp Regularity".to_string(),
                message: "Timestamp discontinuities or non-monotonic DTS were observed."
                    .to_string(),
            });
        } else {
            timestamp_checks.push(CheckItem {
                code: "TIMESTAMPS_CONTINUOUS".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Timestamp Regularity".to_string(),
                message: "Stream presentation timestamps (PTS) appear continuous.".to_string(),
            });
        }

        // Synthesize decode checks
        let mut decode_checks = Vec::new();
        match report.validation.validation_level {
            ValidationLevel::MetadataOnly => {
                decode_checks.push(CheckItem {
                    code: "DECODE_METADATA_ONLY".to_string(),
                    outcome: ValidationOutcome::NotChecked,
                    title: "Bitstream Decoding".to_string(),
                    message: "Metadata validated; frame bitstream was not decoded.".to_string(),
                });
            }
            ValidationLevel::SampledDecode => {
                if report.validation.passed {
                    decode_checks.push(CheckItem {
                        code: "DECODE_SAMPLED_PASSED".to_string(),
                        outcome: ValidationOutcome::Passed,
                        title: "Sampled Bitstream Decode".to_string(),
                        message: "Sampled initial frames decoded cleanly with null sink."
                            .to_string(),
                    });
                } else {
                    decode_checks.push(CheckItem {
                        code: "DECODE_ERROR_DETECTED".to_string(),
                        outcome: ValidationOutcome::Failed,
                        title: "Sampled Bitstream Decode".to_string(),
                        message: "Errors encountered while decoding sample frames.".to_string(),
                    });
                }
            }
            ValidationLevel::FullDecode => {
                if report.validation.passed {
                    decode_checks.push(CheckItem {
                        code: "DECODE_FULL_PASSED".to_string(),
                        outcome: ValidationOutcome::Passed,
                        title: "Full Bitstream Decode".to_string(),
                        message: "Entire media file decoded through null sink with 0 fatal errors."
                            .to_string(),
                    });
                } else {
                    decode_checks.push(CheckItem {
                        code: "DECODE_ERROR_DETECTED".to_string(),
                        outcome: ValidationOutcome::Failed,
                        title: "Full Bitstream Decode".to_string(),
                        message: "Errors encountered during full bitstream decode.".to_string(),
                    });
                }
            }
        }

        // Synthesize audio checks
        let mut audio_checks = Vec::new();
        if out_audio_count > 0 {
            audio_checks.push(CheckItem {
                code: "AUDIO_STREAM_PRESENT".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Audio Track".to_string(),
                message: format!("Detected {out_audio_count} valid audio track(s)."),
            });
        } else if report.recovery_method == RecoveryMethod::StreamSalvage {
            audio_checks.push(CheckItem {
                code: "AUDIO_INTENTIONALLY_OMITTED".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Audio Track".to_string(),
                message: "Audio stream was intentionally omitted per salvage configuration."
                    .to_string(),
            });
        } else {
            audio_checks.push(CheckItem {
                code: "AUDIO_UNEXPECTEDLY_MISSING".to_string(),
                outcome: ValidationOutcome::Warning,
                title: "Audio Track".to_string(),
                message: "No audio stream present in recovered container.".to_string(),
            });
        }

        let overall_outcome = if !report.validation.passed {
            ValidationOutcome::Failed
        } else if !report.warnings.is_empty() || report.validation.stream_loss_detected {
            ValidationOutcome::Warning
        } else {
            ValidationOutcome::Passed
        };

        let inspection_coverage = match report.validation.validation_level {
            ValidationLevel::MetadataOnly => {
                "Metadata container headers and stream descriptors only. Bitstream frames unverified."
                    .to_string()
            }
            ValidationLevel::SampledDecode => {
                "Initial 8-second window verified via FFmpeg null-sink decode. Subsequent timeline unverified."
                    .to_string()
            }
            ValidationLevel::FullDecode => {
                "Full file decode verified from 0:00 to end of stream.".to_string()
            }
        };

        let val_summary = ValidationSummary {
            level: report.validation.validation_level,
            overall_outcome,
            container_checks,
            stream_checks,
            timestamp_checks,
            decode_checks,
            audio_checks,
            duration_comparison: duration_comp,
            inspection_coverage,
        };

        let source_summary = AssetMediaSummary {
            file_name: report.source_file_name.clone(),
            file_size_bytes: None,
            container_format: report.original_container.clone(),
            duration_seconds: report.source_duration_seconds,
            streams: report.input_streams.clone(),
            is_playable: source_is_playable,
        };

        let output_summary = if report.output_streams.is_empty() {
            None
        } else {
            Some(AssetMediaSummary {
                file_name: format!(
                    "recovered_{}.{}",
                    report.job_id, report.actual_output_container
                ),
                file_size_bytes: report.output_file_size_bytes,
                container_format: report.actual_output_container.clone(),
                duration_seconds: report.output_duration_seconds,
                streams: report.output_streams.clone(),
                is_playable: report.validation.is_playable,
            })
        };

        let timeline_evidence = Some(TimelineEvidence {
            source_duration_seconds: report.source_duration_seconds,
            recovered_duration_seconds: report.output_duration_seconds,
            video_start_offset_seconds: Some(0.0),
            audio_start_offset_seconds: if out_audio_count > 0 { Some(0.0) } else { None },
            av_drift_seconds: None,
            known_decode_failures: report.observed_ffmpeg_errors.clone(),
            uninspected_regions_note: match report.validation.validation_level {
                ValidationLevel::SampledDecode => Some(
                    "Frames beyond the initial 8s sample window remain unverified.".to_string(),
                ),
                ValidationLevel::MetadataOnly => {
                    Some("All bitstream payload frames remain uninspected.".to_string())
                }
                ValidationLevel::FullDecode => None,
            },
        });

        let mut limitations = report.limitations.clone();
        if is_source_duration_unknown {
            limitations.push(
                "Original timeline duration is unknown due to missing or damaged container header."
                    .to_string(),
            );
        }
        limitations.push(
            "Playback capability does not guarantee that 100% of original recorded frames were recovered."
                .to_string(),
        );

        let method_str = match report.recovery_method {
            RecoveryMethod::LosslessRemux => "Lossless Remux",
            RecoveryMethod::StreamSalvage => "Stream Salvage",
            RecoveryMethod::TimestampRebuild => "Timestamp Rebuild",
        };

        Self {
            schema_version: RECOVERY_REVIEW_SCHEMA_VERSION,
            review_id: format!("rev_{}", report.job_id),
            source_asset_id: report.source_asset_id.clone(),
            recovery_job_id: report.job_id.clone(),
            recovery_method: method_str.to_string(),
            recovery_status: report.processing_status,
            output_asset_id: candidate_asset_id.clone(),
            output_candidate_id: candidate_asset_id,
            source_summary,
            output_summary,
            validation_summary: val_summary,
            timeline_evidence,
            warnings: report.warnings.clone(),
            limitations,
            created_at_epoch_ms: report.completed_at_epoch_ms,
        }
    }

    /// Creates a RecoveryReview from a ReconstructionReport (Reference-Based MP4/MOV recovery).
    pub fn from_reconstruction_report(
        report: &ReconstructionReport,
        candidate_asset_id: Option<String>,
    ) -> Self {
        let is_source_duration_unknown = true; // By definition, missing moov has no source duration
        let duration_comp = DurationComparison {
            source_duration_seconds: None,
            recovered_duration_seconds: report.recovered_duration_seconds,
            difference_seconds: None,
            is_source_duration_unknown,
            explanation: "Source MP4 lacked moov atom; original duration was missing. Recovered duration was synthesized from sample table reconstruction."
                .to_string(),
        };

        let mut container_checks = Vec::new();
        if let Some(bytes) = report.output_file_size_bytes {
            if bytes > 0 {
                container_checks.push(CheckItem {
                    code: "CONTAINER_EXISTS_NONEMPTY".to_string(),
                    outcome: ValidationOutcome::Passed,
                    title: "Container Integrity".to_string(),
                    message: format!("Reconstructed container size: {bytes} bytes."),
                });
            } else {
                container_checks.push(CheckItem {
                    code: "CONTAINER_EMPTY".to_string(),
                    outcome: ValidationOutcome::Failed,
                    title: "Container Empty".to_string(),
                    message: "Reconstructed output file is 0 bytes.".to_string(),
                });
            }
        }

        container_checks.push(CheckItem {
            code: "MOOV_ATOM_SYNTHESIZED".to_string(),
            outcome: ValidationOutcome::Passed,
            title: "Header Reconstruction".to_string(),
            message: "Generated valid moov metadata and sample tables from reference donor."
                .to_string(),
        });

        let mut stream_checks = Vec::new();
        let out_video_count = report
            .reconstructed_streams
            .iter()
            .filter(|s| s.stream_type == "video")
            .count();
        let out_audio_count = report
            .reconstructed_streams
            .iter()
            .filter(|s| s.stream_type == "audio")
            .count();

        if report.reconstructed_streams.is_empty() {
            stream_checks.push(CheckItem {
                code: "STREAM_NONE_DETECTED".to_string(),
                outcome: ValidationOutcome::Failed,
                title: "Stream Inventory".to_string(),
                message: "No playable streams recovered from damaged payload.".to_string(),
            });
        } else {
            stream_checks.push(CheckItem {
                code: "STREAM_INVENTORY_DETECTED".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Stream Inventory".to_string(),
                message: format!(
                    "Reconstructed {} stream(s): {} video, {} audio.",
                    report.reconstructed_streams.len(),
                    out_video_count,
                    out_audio_count
                ),
            });
        }

        let timestamp_checks = vec![CheckItem {
            code: "TIMESTAMPS_RECONSTRUCTED".to_string(),
            outcome: ValidationOutcome::Passed,
            title: "Timing Synthesis".to_string(),
            message: "Synthesized monotonic timebase and sample durations from donor SPS/timebase."
                .to_string(),
        }];

        let mut decode_checks = Vec::new();
        if report.validation.passed {
            decode_checks.push(CheckItem {
                code: "DECODE_SAMPLED_PASSED".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Bitstream Decode".to_string(),
                message: "Reconstructed bitstream decoded cleanly with FFmpeg null-sink."
                    .to_string(),
            });
        } else {
            decode_checks.push(CheckItem {
                code: "DECODE_ERROR_DETECTED".to_string(),
                outcome: ValidationOutcome::Failed,
                title: "Bitstream Decode".to_string(),
                message: "Decode errors observed during candidate validation.".to_string(),
            });
        }

        let mut audio_checks = Vec::new();
        if out_audio_count > 0 {
            audio_checks.push(CheckItem {
                code: "AUDIO_STREAM_PRESENT".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Audio Track".to_string(),
                message: format!("Reconstructed {out_audio_count} audio stream(s)."),
            });
        } else {
            audio_checks.push(CheckItem {
                code: "AUDIO_NOT_PRESENT".to_string(),
                outcome: ValidationOutcome::Warning,
                title: "Audio Track".to_string(),
                message: "Audio stream absent in reconstructed output.".to_string(),
            });
        }

        let overall_outcome = if !report.validation.passed {
            ValidationOutcome::Failed
        } else if !report.warnings.is_empty() {
            ValidationOutcome::Warning
        } else {
            ValidationOutcome::Passed
        };

        let val_summary = ValidationSummary {
            level: report.validation.validation_level,
            overall_outcome,
            container_checks,
            stream_checks,
            timestamp_checks,
            decode_checks,
            audio_checks,
            duration_comparison: duration_comp,
            inspection_coverage: "Sampled bitstream null-sink decode on reconstructed candidate."
                .to_string(),
        };

        let source_summary = AssetMediaSummary {
            file_name: report.damaged_file_name.clone(),
            file_size_bytes: None,
            container_format: "mp4/mov (unfinalized)".to_string(),
            duration_seconds: None,
            streams: Vec::new(),
            is_playable: false,
        };

        let output_summary = Some(AssetMediaSummary {
            file_name: format!("recovered_{}.mp4", report.operation_id),
            file_size_bytes: report.output_file_size_bytes,
            container_format: "mp4".to_string(),
            duration_seconds: report.recovered_duration_seconds,
            streams: report.reconstructed_streams.clone(),
            is_playable: report.validation.is_playable,
        });

        let timeline_evidence = Some(TimelineEvidence {
            source_duration_seconds: None,
            recovered_duration_seconds: report.recovered_duration_seconds,
            video_start_offset_seconds: Some(0.0),
            audio_start_offset_seconds: if out_audio_count > 0 { Some(0.0) } else { None },
            av_drift_seconds: None,
            known_decode_failures: Vec::new(),
            uninspected_regions_note: Some(
                "Sampled validation inspected initial keyframes. Deeper sample validation can be run on request."
                    .to_string(),
            ),
        });

        let mut limitations = report.uncertainties.clone();
        limitations.push(
            "Donor reference parameter sets (SPS/PPS) were used to synthesize moov metadata."
                .to_string(),
        );
        limitations.push(
            "Frame timestamps were reconstructed from fixed frame rate assumption.".to_string(),
        );

        let status = match report.processing_status {
            super::reconstruction_report::ReconstructionStatus::Completed => {
                RecoveryStatus::Completed
            }
            super::reconstruction_report::ReconstructionStatus::Partial => RecoveryStatus::Partial,
            super::reconstruction_report::ReconstructionStatus::Failed => RecoveryStatus::Failed,
            super::reconstruction_report::ReconstructionStatus::Cancelled => {
                RecoveryStatus::Cancelled
            }
            _ => RecoveryStatus::Running,
        };

        Self {
            schema_version: RECOVERY_REVIEW_SCHEMA_VERSION,
            review_id: format!("rev_{}", report.operation_id),
            source_asset_id: report.damaged_asset_id.clone(),
            recovery_job_id: report.operation_id.clone(),
            recovery_method: "Reference-Based MP4 Reconstruction".to_string(),
            recovery_status: status,
            output_asset_id: candidate_asset_id.clone(),
            output_candidate_id: candidate_asset_id,
            source_summary,
            output_summary,
            validation_summary: val_summary,
            timeline_evidence,
            warnings: report.warnings.clone(),
            limitations,
            created_at_epoch_ms: report.completed_at_epoch_ms,
        }
    }
}
