use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::media::error::MediaEngineError;
use crate::media::probe::probe_media_file;

use super::container::identify_container;
use super::iso_bmff::scan_iso_bmff_structure;
use super::recovery_report::{
    RecoveryMethod, RecoveryReport, RecoveryRequest, RecoveryStatus, RecoveryValidationResult,
    SalvageStreamType, StreamReportInfo, ValidationLevel, RECOVERY_REPORT_SCHEMA_VERSION,
};
use super::remux::execute_lossless_remux;
use super::salvage::execute_stream_salvage;
use super::validation::{validate_recovery_output, OutputValidationOutcome};

/// Executes the end-to-end media recovery pipeline in an application-managed staging area.
///
/// Under no circumstances does this function modify, truncate, or overwrite the source media file.
pub async fn run_recovery_pipeline(
    source_path: &Path,
    source_asset_id: &str,
    source_file_name: &str,
    request: &RecoveryRequest,
    staging_dir: &Path,
    cancellation: Option<&CancellationToken>,
) -> Result<RecoveryReport, MediaEngineError> {
    let started_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let job_id = Uuid::new_v4().to_string();
    let mut warnings = Vec::new();
    let mut observed_ffmpeg_errors = Vec::new();
    let mut limitations = Vec::new();

    // 1. Source existence and canonical check
    let canonical_source =
        source_path
            .canonicalize()
            .map_err(|e| MediaEngineError::InvalidPath {
                path: source_path.display().to_string(),
                reason: format!("Cannot resolve source path: {e}"),
            })?;

    let source_metadata =
        fs::metadata(&canonical_source).map_err(|e| MediaEngineError::InvalidPath {
            path: canonical_source.display().to_string(),
            reason: format!("Cannot read source metadata: {e}"),
        })?;
    let source_size = source_metadata.len();

    // 2. Identify source container
    let container_res = identify_container(&canonical_source, source_size);
    let (original_container, container_ext, is_iso_bmff) = match container_res {
        Ok(ref id) => (
            id.container_info.format_type.clone(),
            id.container_info.extension.clone(),
            id.is_iso_bmff,
        ),
        Err(_) => ("unknown".to_string(), "".to_string(), false),
    };

    // 3. Pre-flight check: Missing moov atom in MP4/MOV
    if is_iso_bmff {
        if let Ok(iso_res) = scan_iso_bmff_structure(&canonical_source, source_size) {
            if !iso_res.has_moov && iso_res.has_mdat {
                limitations.push(
                    "Missing moov atom detected. Standard container remux and stream salvage cannot index packets without sample tables. Reference-based reconstruction (STEP 14) is required."
                        .to_string(),
                );
                return Ok(RecoveryReport {
                    schema_version: RECOVERY_REPORT_SCHEMA_VERSION,
                    job_id,
                    source_asset_id: source_asset_id.to_string(),
                    source_file_name: source_file_name.to_string(),
                    recovery_method: request.method,
                    original_container,
                    intended_output_container: request
                        .output_container_preference
                        .clone()
                        .unwrap_or_else(|| "mp4".to_string()),
                    actual_output_container: "none".to_string(),
                    input_streams: Vec::new(),
                    output_streams: Vec::new(),
                    processing_status: RecoveryStatus::Failed,
                    validation: RecoveryValidationResult {
                        passed: false,
                        is_playable: false,
                        validation_level: ValidationLevel::MetadataOnly,
                        streams_present: Vec::new(),
                        streams_expected: Vec::new(),
                        stream_loss_detected: true,
                        output_duration_seconds: None,
                        details: "Unfinalized recording missing moov atom cannot be remuxed without STEP 14 reconstruction donor.".to_string(),
                        warnings: vec!["Cannot demux stream packets without track index atom (moov).".to_string()],
                    },
                    warnings,
                    observed_ffmpeg_errors: vec!["moov atom not found".to_string()],
                    output_file_size_bytes: None,
                    output_file_path: None,
                    source_duration_seconds: None,
                    output_duration_seconds: None,
                    duration_difference_seconds: None,
                    started_at_epoch_ms,
                    completed_at_epoch_ms: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    limitations,
                });
            }
        }
    }

    // 4. Probe source file to extract input streams and duration
    let mut input_streams = Vec::new();
    let mut source_duration_seconds = None;
    let mut expected_video_count = 0;
    let mut expected_audio_count = 0;

    if let Ok(meta) = probe_media_file(None, &canonical_source, source_size, cancellation).await {
        source_duration_seconds = meta.duration_micros().map(|d| d as f64 / 1_000_000.0);
        for v in &meta.video_streams {
            expected_video_count += 1;
            input_streams.push(StreamReportInfo {
                index: v.stream_index,
                stream_type: "video".to_string(),
                codec_name: v.codec_name.clone(),
                duration_seconds: source_duration_seconds,
                resolution: Some(format!("{}x{}", v.width, v.height)),
                sample_rate: None,
                channels: None,
            });
        }
        for a in &meta.audio_streams {
            expected_audio_count += 1;
            input_streams.push(StreamReportInfo {
                index: a.stream_index,
                stream_type: "audio".to_string(),
                codec_name: a.codec_name.clone(),
                duration_seconds: source_duration_seconds,
                resolution: None,
                sample_rate: Some(a.sample_rate),
                channels: Some(a.channels),
            });
        }
    }

    // Adjust expectations based on requested method and salvage options
    match request.method {
        RecoveryMethod::StreamSalvage => match request.salvage_stream_type {
            Some(SalvageStreamType::VideoOnly) => {
                expected_audio_count = 0;
                expected_video_count = expected_video_count.min(1);
            }
            Some(SalvageStreamType::AudioOnly) => {
                expected_video_count = 0;
                expected_audio_count = expected_audio_count.min(1);
            }
            _ => {}
        },
        RecoveryMethod::LosslessRemux | RecoveryMethod::TimestampRebuild => {}
    }

    // 5. Determine target container extension
    let default_ext = if !container_ext.is_empty() {
        container_ext.as_str()
    } else {
        "mp4"
    };
    let target_ext = request
        .output_container_preference
        .as_deref()
        .unwrap_or(default_ext)
        .trim_start_matches('.')
        .to_lowercase();

    // 6. Setup staging directory and output path
    fs::create_dir_all(staging_dir).map_err(|e| {
        MediaEngineError::Internal(format!("Failed to create recovery staging directory: {e}"))
    })?;

    let output_file_name = format!("recovered_{}.{}", job_id, target_ext);
    let staged_output_path = staging_dir.join(&output_file_name);

    // Staging invariant: Never overwrite source
    if staged_output_path == canonical_source {
        return Err(MediaEngineError::InvalidPath {
            path: staged_output_path.display().to_string(),
            reason: "Staged output destination matches source file path. Operation aborted to prevent clobbering source.".to_string(),
        });
    }

    // Check cancellation before launching process
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            return Err(MediaEngineError::Cancelled {
                command: "media recovery pipeline".to_string(),
            });
        }
    }

    // 7. Execute recovery engine
    let exec_result = match request.method {
        RecoveryMethod::LosslessRemux => {
            execute_lossless_remux(
                &canonical_source,
                &staged_output_path,
                false,
                &target_ext,
                cancellation,
            )
            .await
        }
        RecoveryMethod::TimestampRebuild => {
            limitations.push(
                "Timestamp rebuild applies genpts and normalize offset. Stream copy is preserved without re-encoding."
                    .to_string(),
            );
            execute_lossless_remux(
                &canonical_source,
                &staged_output_path,
                true,
                &target_ext,
                cancellation,
            )
            .await
        }
        RecoveryMethod::StreamSalvage => {
            let stream_type = request
                .salvage_stream_type
                .unwrap_or(SalvageStreamType::AllReadable);
            limitations.push(format!(
                "Salvage mode isolating stream type: {stream_type:?}. Damaged or omitted streams were discarded."
            ));
            execute_stream_salvage(
                &canonical_source,
                &staged_output_path,
                stream_type,
                &target_ext,
                cancellation,
            )
            .await
        }
    };

    // Check cancellation
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            let _ = fs::remove_file(&staged_output_path);
            return Err(MediaEngineError::Cancelled {
                command: "media recovery process".to_string(),
            });
        }
    }

    match exec_result {
        Ok(proc_res) => {
            warnings.extend(proc_res.warnings);
            observed_ffmpeg_errors.extend(proc_res.errors);
        }
        Err(e) => {
            let _ = fs::remove_file(&staged_output_path);
            return Err(e);
        }
    }

    // 8. Output Validation
    let validation_outcome: OutputValidationOutcome = validate_recovery_output(
        &staged_output_path,
        expected_video_count,
        expected_audio_count,
        ValidationLevel::SampledDecode,
        cancellation,
    )
    .await;

    // Check cancellation after validation
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            let _ = fs::remove_file(&staged_output_path);
            return Err(MediaEngineError::Cancelled {
                command: "media recovery validation".to_string(),
            });
        }
    }

    warnings.extend(validation_outcome.validation_result.warnings.clone());
    observed_ffmpeg_errors.extend(validation_outcome.observed_errors);

    let final_status = validation_outcome.status;
    let output_duration_seconds = validation_outcome.output_duration_seconds;
    let duration_difference_seconds = match (source_duration_seconds, output_duration_seconds) {
        (Some(src), Some(out)) => Some((out - src).abs()),
        _ => None,
    };

    let completed_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let output_file_path_str = if final_status == RecoveryStatus::Failed {
        let _ = fs::remove_file(&staged_output_path);
        None
    } else {
        Some(staged_output_path.to_string_lossy().to_string())
    };

    Ok(RecoveryReport {
        schema_version: RECOVERY_REPORT_SCHEMA_VERSION,
        job_id,
        source_asset_id: source_asset_id.to_string(),
        source_file_name: source_file_name.to_string(),
        recovery_method: request.method,
        original_container,
        intended_output_container: target_ext.clone(),
        actual_output_container: validation_outcome.format_name,
        input_streams,
        output_streams: validation_outcome.output_streams,
        processing_status: final_status,
        validation: validation_outcome.validation_result,
        warnings,
        observed_ffmpeg_errors,
        output_file_size_bytes: validation_outcome.output_size_bytes,
        output_file_path: output_file_path_str,
        source_duration_seconds,
        output_duration_seconds,
        duration_difference_seconds,
        started_at_epoch_ms,
        completed_at_epoch_ms,
        limitations,
    })
}

/// Safely finalizes a staged recovery output into a user-specified destination path.
///
/// Implements strict security guarantees:
/// 1. Verifies destination does not already exist (no silent clobbering).
/// 2. Verifies destination is not a symlink.
/// 3. Verifies destination canonical path does not collide with source.
/// 4. Validates that final file size exactly matches staged size.
pub fn finalize_recovered_file(
    staged_path: &Path,
    destination_path: &Path,
    source_canonical_path: &Path,
) -> Result<PathBuf, String> {
    if !staged_path.exists() {
        return Err(
            "Staged recovery file does not exist or has already been finalized".to_string(),
        );
    }

    // 1. Strict No-Clobber: Destination must not already exist
    if destination_path.exists() {
        return Err(format!(
            "Destination file already exists: {}. Please choose a different name or path.",
            destination_path.display()
        ));
    }

    // 2. Symlink protection: Check if destination or parent is a symlink
    if destination_path.is_symlink() {
        return Err(
            "Destination path is a symlink. Writing to symlinks is prohibited for safety."
                .to_string(),
        );
    }

    // 3. Parent directory check
    let parent = destination_path
        .parent()
        .ok_or_else(|| "Destination path has no parent directory".to_string())?;

    if !parent.exists() {
        return Err(format!(
            "Parent directory does not exist: {}",
            parent.display()
        ));
    }

    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Cannot canonicalize parent directory: {e}"))?;

    let file_name = destination_path
        .file_name()
        .ok_or_else(|| "Destination has no filename".to_string())?;

    let canonical_dest = canonical_parent.join(file_name);

    // 4. Source collision protection: Destination must not be the source file
    if canonical_dest == *source_canonical_path {
        return Err("Destination path collides with source file path. Overwriting the source is prohibited.".to_string());
    }

    // 5. Safe copy and verification
    let staged_meta =
        fs::metadata(staged_path).map_err(|e| format!("Failed to read staged metadata: {e}"))?;
    let staged_len = staged_meta.len();

    fs::copy(staged_path, &canonical_dest)
        .map_err(|e| format!("Failed to copy recovered file to destination: {e}"))?;

    let final_meta = fs::metadata(&canonical_dest)
        .map_err(|e| format!("Failed to verify finalized destination: {e}"))?;

    if final_meta.len() != staged_len {
        let _ = fs::remove_file(&canonical_dest);
        return Err("Finalized file size mismatch. Destination file was cleaned up.".to_string());
    }

    Ok(canonical_dest)
}
