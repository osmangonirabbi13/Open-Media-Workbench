use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::media::discovery::{discover_tool, DiscoveryConfig, ToolType};
use crate::media::probe::probe_media_file;
use crate::media::process::{run_managed_process, ProcessSpec};

use super::container::identify_container;
use super::recovery_report::{
    RecoveryStatus, RecoveryValidationResult, StreamReportInfo, ValidationLevel,
};

/// Timeout for bounded decode test during validation.
pub const VALIDATION_DECODE_TIMEOUT: Duration = Duration::from_secs(20);

/// Bounded sample duration to decode during validation (in seconds).
pub const VALIDATION_SAMPLE_SECS: u64 = 8;

/// Maximum stderr bytes to capture during validation decode test.
pub const MAX_VALIDATION_STDERR_BYTES: usize = 128 * 1024;

/// Result of complete output validation.
#[derive(Debug, Clone)]
pub struct OutputValidationOutcome {
    pub validation_result: RecoveryValidationResult,
    pub status: RecoveryStatus,
    pub output_streams: Vec<StreamReportInfo>,
    pub output_size_bytes: Option<u64>,
    pub output_duration_seconds: Option<f64>,
    pub format_name: String,
    pub observed_errors: Vec<String>,
}

/// Validates recovered media candidate to ensure non-empty size, recognized container,
/// preserved streams, and bounded playable decode.
pub async fn validate_recovery_output(
    output_path: &Path,
    expected_video_count: usize,
    expected_audio_count: usize,
    level: ValidationLevel,
    cancellation: Option<&CancellationToken>,
) -> OutputValidationOutcome {
    let mut warnings = Vec::new();
    let mut observed_errors = Vec::new();
    let mut streams_present = Vec::new();
    let mut streams_expected = Vec::new();

    if expected_video_count > 0 {
        streams_expected.push(format!("{expected_video_count} video stream(s)"));
    }
    if expected_audio_count > 0 {
        streams_expected.push(format!("{expected_audio_count} audio stream(s)"));
    }

    // 1. Check file existence and non-empty size
    let metadata = match fs::metadata(output_path) {
        Ok(m) => m,
        Err(e) => {
            observed_errors.push(format!("Output file could not be read: {e}"));
            return OutputValidationOutcome {
                validation_result: RecoveryValidationResult {
                    passed: false,
                    is_playable: false,
                    validation_level: level,
                    streams_present,
                    streams_expected,
                    stream_loss_detected: true,
                    output_duration_seconds: None,
                    details: "Output file does not exist or is inaccessible".to_string(),
                    warnings,
                },
                status: RecoveryStatus::Failed,
                output_streams: Vec::new(),
                output_size_bytes: None,
                output_duration_seconds: None,
                format_name: "unknown".to_string(),
                observed_errors,
            };
        }
    };

    let file_size = metadata.len();
    if file_size == 0 {
        observed_errors.push("Recovery operation generated a 0-byte file".to_string());
        return OutputValidationOutcome {
            validation_result: RecoveryValidationResult {
                passed: false,
                is_playable: false,
                validation_level: level,
                streams_present,
                streams_expected,
                stream_loss_detected: true,
                output_duration_seconds: None,
                details: "Recovery failed: output file is empty (0 bytes)".to_string(),
                warnings,
            },
            status: RecoveryStatus::Failed,
            output_streams: Vec::new(),
            output_size_bytes: Some(0),
            output_duration_seconds: None,
            format_name: "empty".to_string(),
            observed_errors,
        };
    }

    // 2. Container identification
    let container_res = identify_container(output_path, file_size);
    let (format_name, signature_matched) = match container_res {
        Ok(ref id) => (
            id.container_info.format_type.clone(),
            id.container_info.signature_matched,
        ),
        Err(_) => ("unknown".to_string(), false),
    };
    if !signature_matched {
        warnings.push("Output file container signature is unrecognized or incomplete".to_string());
    }

    // 3. FFprobe probe
    let probe_res = probe_media_file(None, output_path, file_size, cancellation).await;
    let mut output_streams = Vec::new();
    let mut output_duration_seconds = None;
    let mut actual_video_count = 0;
    let mut actual_audio_count = 0;

    match probe_res {
        Ok(media_meta) => {
            output_duration_seconds = media_meta.duration_micros().map(|d| d as f64 / 1_000_000.0);

            for v in &media_meta.video_streams {
                actual_video_count += 1;
                let resolution = format!("{}x{}", v.width, v.height);
                streams_present.push(format!(
                    "Video stream (codec: {}, {})",
                    v.codec_name, resolution
                ));
                output_streams.push(StreamReportInfo {
                    index: v.stream_index,
                    stream_type: "video".to_string(),
                    codec_name: v.codec_name.clone(),
                    duration_seconds: output_duration_seconds,
                    resolution: Some(resolution),
                    sample_rate: None,
                    channels: None,
                });
            }

            for a in &media_meta.audio_streams {
                actual_audio_count += 1;
                streams_present.push(format!(
                    "Audio stream (codec: {}, {} Hz, {} ch)",
                    a.codec_name, a.sample_rate, a.channels
                ));
                output_streams.push(StreamReportInfo {
                    index: a.stream_index,
                    stream_type: "audio".to_string(),
                    codec_name: a.codec_name.clone(),
                    duration_seconds: output_duration_seconds,
                    resolution: None,
                    sample_rate: Some(a.sample_rate),
                    channels: Some(a.channels),
                });
            }
        }
        Err(e) => {
            observed_errors.push(format!("FFprobe failed to inspect output file: {e}"));
        }
    }

    // 4. Check for stream loss
    let mut stream_loss_detected = false;
    if expected_video_count > 0 && actual_video_count == 0 {
        stream_loss_detected = true;
        warnings.push("Expected video stream was not found in recovered output".to_string());
    }
    if expected_audio_count > 0 && actual_audio_count == 0 {
        stream_loss_detected = true;
        warnings.push("Expected audio stream was not found in recovered output".to_string());
    }

    let has_any_stream = actual_video_count > 0 || actual_audio_count > 0;
    if !has_any_stream {
        observed_errors.push("Output contains zero playable streams".to_string());
        return OutputValidationOutcome {
            validation_result: RecoveryValidationResult {
                passed: false,
                is_playable: false,
                validation_level: level,
                streams_present,
                streams_expected,
                stream_loss_detected: true,
                output_duration_seconds,
                details: "Output file contains no recognized audio or video streams".to_string(),
                warnings,
            },
            status: RecoveryStatus::Failed,
            output_streams,
            output_size_bytes: Some(file_size),
            output_duration_seconds,
            format_name: format_name.clone(),
            observed_errors,
        };
    }

    // 5. Decode verification (Sampled or Full)
    let mut decode_passed = true;
    if level != ValidationLevel::MetadataOnly {
        if let Ok(ffmpeg) = discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default()) {
            let mut args = vec![
                "-v".to_string(),
                "warning".to_string(),
                "-i".to_string(),
                output_path.to_string_lossy().to_string(),
            ];

            if level == ValidationLevel::SampledDecode {
                args.push("-t".to_string());
                args.push(VALIDATION_SAMPLE_SECS.to_string());
            }

            args.push("-f".to_string());
            args.push("null".to_string());
            args.push("-".to_string());

            let mut spec = ProcessSpec::new(ffmpeg.canonical_path, args, VALIDATION_DECODE_TIMEOUT);
            spec.max_stderr_bytes = MAX_VALIDATION_STDERR_BYTES;

            match run_managed_process(spec, cancellation).await {
                Ok(proc_out) => {
                    let stderr = proc_out.stderr;
                    if let Some(code) = proc_out.exit_code {
                        if code != 0 {
                            decode_passed = false;
                            observed_errors
                                .push(format!("Decode verification failed with exit code {code}"));
                        }
                    }
                    if !stderr.is_empty() {
                        let lower_err = stderr.to_lowercase();
                        if lower_err.contains("fatal") || lower_err.contains("error while decoding")
                        {
                            decode_passed = false;
                            observed_errors.push(
                                "Fatal decode errors observed during playback validation"
                                    .to_string(),
                            );
                        } else {
                            warnings.push(
                                "Minor non-fatal decode warnings observed in output stream"
                                    .to_string(),
                            );
                        }
                    }
                }
                Err(e) => {
                    decode_passed = false;
                    observed_errors.push(format!("Decode verification process error: {e}"));
                }
            }
        } else {
            warnings.push(
                "FFmpeg binary not available for decode verification; validated via FFprobe only"
                    .to_string(),
            );
        }
    }

    // 6. Classification
    let final_status = if !decode_passed {
        RecoveryStatus::Failed
    } else if stream_loss_detected || !warnings.is_empty() {
        RecoveryStatus::Partial
    } else {
        RecoveryStatus::Completed
    };

    let is_playable = decode_passed && has_any_stream;
    let passed = is_playable;

    let details = match final_status {
        RecoveryStatus::Completed => {
            format!(
                "Recovery completed successfully. Verified {} streams ({} bytes, ~{:.2}s duration).",
                output_streams.len(),
                file_size,
                output_duration_seconds.unwrap_or(0.0)
            )
        }
        RecoveryStatus::Partial => {
            format!(
                "Partial recovery: output is playable with {} streams, but with warnings or selective stream omission.",
                output_streams.len()
            )
        }
        RecoveryStatus::Failed => {
            "Validation failed: generated output could not be decoded reliably.".to_string()
        }
        _ => "Validation completed.".to_string(),
    };

    OutputValidationOutcome {
        validation_result: RecoveryValidationResult {
            passed,
            is_playable,
            validation_level: level,
            streams_present,
            streams_expected,
            stream_loss_detected,
            output_duration_seconds,
            details,
            warnings,
        },
        status: final_status,
        output_streams,
        output_size_bytes: Some(file_size),
        output_duration_seconds,
        format_name,
        observed_errors,
    }
}
