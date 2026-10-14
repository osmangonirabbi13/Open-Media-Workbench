use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::media::discovery::{discover_tool, DiscoveryConfig, ToolType};
use crate::media::error::MediaEngineError;
use crate::media::process::{run_managed_process, ProcessSpec};

use super::iso_bmff::scan_iso_bmff_structure;
use super::reconstruction_report::{
    ReconstructionReport, ReconstructionRequest, ReconstructionStatus,
    ReferenceCompatibilityClassification, ReferenceCompatibilityReport,
    RECONSTRUCTION_REPORT_SCHEMA_VERSION,
};
use super::recovery_report::{RecoveryValidationResult, ValidationLevel};
use super::reference::analyze_reference_compatibility;
use super::validation::validate_recovery_output;

/// Engine execution evidence recording exact recovery steps performed.
#[derive(Debug, Clone)]
pub struct EngineExecutionEvidence {
    pub engine_name: String,
    pub engine_version: String,
    pub execution_mode: String,
    pub extracted_nal_units: usize,
    pub injected_sps_bytes: usize,
    pub injected_pps_bytes: usize,
    pub raw_payload_bytes: u64,
    pub recovered_stream_count: usize,
}

/// Available reconstruction engine capabilities.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructionEngineCapability {
    pub engine_id: String,
    pub name: String,
    pub version: String,
    pub is_available: bool,
    pub description: String,
}

/// Discovers available reconstruction engines on the current platform.
pub fn discover_reconstruction_capabilities() -> Vec<ReconstructionEngineCapability> {
    let mut capabilities = vec![ReconstructionEngineCapability {
        engine_id: "native".to_string(),
        name: "Built-in Safe Rust Reconstructor".to_string(),
        version: "1.0.0-native".to_string(),
        is_available: true,
        description:
            "Native ISO BMFF sample table synthesis & H.264/AVC parameter set donor reconstruction."
                .to_string(),
    }];

    let untrunc_available = std::process::Command::new("which")
        .arg("untrunc")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    capabilities.push(ReconstructionEngineCapability {
        engine_id: "untrunc".to_string(),
        name: "Untrunc CLI Adapter".to_string(),
        version: if untrunc_available {
            "detected".to_string()
        } else {
            "not-installed".to_string()
        },
        is_available: untrunc_available,
        description:
            "External C++ untrunc binary (anthwlock/ponchio) for reference-based MP4 repair."
                .to_string(),
    });

    capabilities
}

/// Executes the end-to-end reference-based reconstruction pipeline in an application-managed staging area.
///
/// Under no circumstances does this function modify either the damaged file or the reference file.
#[allow(clippy::too_many_arguments)]
pub async fn run_reconstruction_pipeline(
    damaged_path: &Path,
    damaged_asset_id: &str,
    damaged_file_name: &str,
    reference_path: &Path,
    reference_asset_id: &str,
    reference_file_name: &str,
    request: &ReconstructionRequest,
    staging_dir: &Path,
    cancellation: Option<&CancellationToken>,
) -> Result<ReconstructionReport, MediaEngineError> {
    let started_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let operation_id = format!("reconst-{}", Uuid::new_v4());
    let mut warnings = Vec::new();
    let mut uncertainties = Vec::new();

    // 1. Check cancellation early
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            return Err(MediaEngineError::Cancelled {
                command: "reconstruction_pipeline".to_string(),
            });
        }
    }

    // 2. Analyze reference compatibility
    let compatibility_report = analyze_reference_compatibility(
        damaged_path,
        damaged_asset_id,
        reference_path,
        reference_asset_id,
        cancellation,
    )
    .await?;

    if compatibility_report.compatibility == ReferenceCompatibilityClassification::Incompatible {
        return Ok(ReconstructionReport {
            schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
            operation_id,
            engine_name: "Open Media Workbench Reconstruction Engine".to_string(),
            engine_version: "1.0.0".to_string(),
            damaged_asset_id: damaged_asset_id.to_string(),
            reference_asset_id: reference_asset_id.to_string(),
            damaged_file_name: damaged_file_name.to_string(),
            reference_file_name: reference_file_name.to_string(),
            reference_compatibility: compatibility_report.clone(),
            attempted_mode: "none".to_string(),
            reconstructed_streams: Vec::new(),
            recovered_duration_seconds: None,
            validation: RecoveryValidationResult {
                passed: false,
                is_playable: false,
                validation_level: ValidationLevel::MetadataOnly,
                streams_present: Vec::new(),
                streams_expected: Vec::new(),
                stream_loss_detected: true,
                output_duration_seconds: None,
                details: compatibility_report.summary.clone(),
                warnings: compatibility_report.warnings.clone(),
            },
            output_file_size_bytes: None,
            output_file_path: None,
            processing_status: ReconstructionStatus::Failed,
            failure_details: Some(compatibility_report.summary),
            warnings,
            uncertainties,
            started_at_epoch_ms,
            completed_at_epoch_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        });
    }

    // 3. Setup application staging directory
    fs::create_dir_all(staging_dir).map_err(|e| MediaEngineError::InvalidPath {
        path: staging_dir.display().to_string(),
        reason: format!("Failed to create staging directory: {e}"),
    })?;

    let output_container = request
        .output_container_preference
        .as_deref()
        .unwrap_or("mp4");
    let candidate_output_path =
        staging_dir.join(format!("reconstructed_candidate.{output_container}"));

    // 4. Select and execute engine
    let selected_engine = request.selected_engine.as_deref().unwrap_or("auto");

    let evidence = if selected_engine == "untrunc" {
        // Untrunc selected explicitly
        execute_untrunc_reconstruction(
            damaged_path,
            reference_path,
            &candidate_output_path,
            cancellation,
        )
        .await?
    } else {
        // Native engine (primary)
        execute_native_reconstruction(
            damaged_path,
            reference_path,
            &candidate_output_path,
            staging_dir,
            &compatibility_report,
            cancellation,
        )
        .await?
    };

    // Check cancellation
    if let Some(token) = cancellation {
        if token.is_cancelled() {
            let _ = fs::remove_file(&candidate_output_path);
            return Err(MediaEngineError::Cancelled {
                command: "reconstruction_pipeline".to_string(),
            });
        }
    }

    // 5. Output Validation
    let expected_video = compatibility_report
        .tracks
        .iter()
        .filter(|t| t.media_type == "video")
        .count()
        .max(1);
    let expected_audio = compatibility_report
        .tracks
        .iter()
        .filter(|t| t.media_type == "audio")
        .count();

    let validation_outcome = validate_recovery_output(
        &candidate_output_path,
        expected_video,
        expected_audio,
        ValidationLevel::SampledDecode,
        cancellation,
    )
    .await;

    let output_size_bytes = fs::metadata(&candidate_output_path).map(|m| m.len()).ok();

    let (processing_status, failure_details) = if validation_outcome.validation_result.passed {
        if validation_outcome.validation_result.stream_loss_detected {
            warnings.push(
                "Output is playable but some tracks from the reference layout were omitted."
                    .to_string(),
            );
            (ReconstructionStatus::Partial, None)
        } else {
            (ReconstructionStatus::Completed, None)
        }
    } else if validation_outcome.validation_result.is_playable {
        (
            ReconstructionStatus::Partial,
            Some(validation_outcome.validation_result.details.clone()),
        )
    } else {
        (
            ReconstructionStatus::Failed,
            Some(validation_outcome.validation_result.details.clone()),
        )
    };

    let recovered_duration_seconds = validation_outcome.output_duration_seconds;
    let reconstructed_streams = validation_outcome.output_streams;
    let validation_result = validation_outcome.validation_result;

    uncertainties.push(
        "Bitstream sample boundaries were reconstructed using donor parameter sets; frame timestamps may contain slight variance from original recording.".to_string(),
    );

    Ok(ReconstructionReport {
        schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
        operation_id,
        engine_name: evidence.engine_name,
        engine_version: evidence.engine_version,
        damaged_asset_id: damaged_asset_id.to_string(),
        reference_asset_id: reference_asset_id.to_string(),
        damaged_file_name: damaged_file_name.to_string(),
        reference_file_name: reference_file_name.to_string(),
        reference_compatibility: compatibility_report,
        attempted_mode: evidence.execution_mode,
        reconstructed_streams,
        recovered_duration_seconds,
        validation: validation_result,
        output_file_size_bytes: output_size_bytes,
        output_file_path: if processing_status != ReconstructionStatus::Failed {
            Some(candidate_output_path.to_string_lossy().to_string())
        } else {
            None
        },
        processing_status,
        failure_details,
        warnings,
        uncertainties,
        started_at_epoch_ms,
        completed_at_epoch_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    })
}

/// Executes native reference-based reconstruction using safe Rust atom parsing and stream copy multiplexing.
async fn execute_native_reconstruction(
    damaged_path: &Path,
    reference_path: &Path,
    output_path: &Path,
    staging_dir: &Path,
    compatibility_report: &ReferenceCompatibilityReport,
    cancellation: Option<&CancellationToken>,
) -> Result<EngineExecutionEvidence, MediaEngineError> {
    // 1. Extract SPS/PPS from reference file
    let (sps, pps) = extract_sps_pps_from_reference(reference_path)?;

    // 2. Scan damaged mdat payload and extract NALUs
    let damaged_size = fs::metadata(damaged_path)
        .map_err(|e| MediaEngineError::InvalidPath {
            path: damaged_path.display().to_string(),
            reason: e.to_string(),
        })?
        .len();

    let damaged_iso = scan_iso_bmff_structure(damaged_path, damaged_size).map_err(|e| {
        MediaEngineError::CorruptMedia {
            path: damaged_path.display().to_string(),
            details: e.to_string(),
        }
    })?;

    let mdat_box = damaged_iso
        .boxes
        .iter()
        .find(|b| &b.box_type == b"mdat")
        .ok_or_else(|| MediaEngineError::CorruptMedia {
            path: damaged_path.display().to_string(),
            details: "Damaged file lacks mdat box".to_string(),
        })?;

    let mdat_offset = mdat_box.offset + mdat_box.header_size;
    let mdat_payload_size = mdat_box.payload_size;

    if mdat_payload_size == 0 {
        return Err(MediaEngineError::CorruptMedia {
            path: damaged_path.display().to_string(),
            details: "Damaged file contains an empty mdat payload (0 bytes)".to_string(),
        });
    }

    let mut damaged_file = File::open(damaged_path).map_err(|e| MediaEngineError::InvalidPath {
        path: damaged_path.display().to_string(),
        reason: e.to_string(),
    })?;

    damaged_file
        .seek(SeekFrom::Start(mdat_offset))
        .map_err(|e| MediaEngineError::CorruptMedia {
            path: damaged_path.display().to_string(),
            details: format!("Failed to seek to mdat offset: {e}"),
        })?;

    // Bounded read buffer (up to 128 MiB for extraction)
    let read_limit = (mdat_payload_size as usize).min(128 * 1024 * 1024);
    let mut mdat_bytes = vec![0u8; read_limit];
    damaged_file
        .read_exact(&mut mdat_bytes)
        .map_err(|e| MediaEngineError::CorruptMedia {
            path: damaged_path.display().to_string(),
            details: format!("Failed to read mdat payload: {e}"),
        })?;

    // 3. Parse NAL units and assemble Annex B stream
    let elementary_stream_path = staging_dir.join(format!("temp_elem_{}.h264", Uuid::new_v4()));
    let mut stream_file = File::create(&elementary_stream_path).map_err(|e| {
        MediaEngineError::Internal(format!("Failed to create temporary stream file: {e}"))
    })?;

    // Prepend donor SPS and PPS with Annex B start codes
    stream_file
        .write_all(b"\x00\x00\x00\x01")
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    stream_file
        .write_all(&sps)
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    stream_file
        .write_all(b"\x00\x00\x00\x01")
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    stream_file
        .write_all(&pps)
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;

    let mut extracted_nal_units = 0usize;
    let mut offset = 0usize;

    // Check if mdat starts with length prefixes (standard MP4)
    let mut is_length_prefixed = false;
    if mdat_bytes.len() >= 8 {
        let first_len =
            u32::from_be_bytes([mdat_bytes[0], mdat_bytes[1], mdat_bytes[2], mdat_bytes[3]])
                as usize;
        if first_len > 0 && first_len < mdat_bytes.len() {
            let nal_type = mdat_bytes[4] & 0x1F;
            if (1..=12).contains(&nal_type) {
                is_length_prefixed = true;
            }
        }
    }

    if is_length_prefixed {
        while offset + 4 < mdat_bytes.len() {
            let nal_len = u32::from_be_bytes([
                mdat_bytes[offset],
                mdat_bytes[offset + 1],
                mdat_bytes[offset + 2],
                mdat_bytes[offset + 3],
            ]) as usize;

            if nal_len == 0 || offset + 4 + nal_len > mdat_bytes.len() {
                break;
            }

            let nal_data = &mdat_bytes[offset + 4..offset + 4 + nal_len];
            let nal_type = nal_data[0] & 0x1F;

            // Before IDR frames (type 5), re-inject SPS/PPS for decoder resync
            if nal_type == 5 && extracted_nal_units > 0 {
                let _ = stream_file.write_all(b"\x00\x00\x00\x01");
                let _ = stream_file.write_all(&sps);
                let _ = stream_file.write_all(b"\x00\x00\x00\x01");
                let _ = stream_file.write_all(&pps);
            }

            let _ = stream_file.write_all(b"\x00\x00\x00\x01");
            let _ = stream_file.write_all(nal_data);

            extracted_nal_units += 1;
            offset += 4 + nal_len;
        }
    } else {
        // Annex B scanning
        let mut i = 0;
        let mut start_indices = Vec::new();
        while i < mdat_bytes.len().saturating_sub(4) {
            if mdat_bytes[i] == 0
                && mdat_bytes[i + 1] == 0
                && (mdat_bytes[i + 2] == 1 || (mdat_bytes[i + 2] == 0 && mdat_bytes[i + 3] == 1))
            {
                let code_len = if mdat_bytes[i + 2] == 1 { 3 } else { 4 };
                start_indices.push((i, code_len));
                i += code_len;
            } else {
                i += 1;
            }
        }

        for (idx, &(pos, code_len)) in start_indices.iter().enumerate() {
            let nal_start = pos + code_len;
            let nal_end = if idx + 1 < start_indices.len() {
                start_indices[idx + 1].0
            } else {
                mdat_bytes.len()
            };

            if nal_start < nal_end {
                let nal_data = &mdat_bytes[nal_start..nal_end];
                let _ = stream_file.write_all(b"\x00\x00\x00\x01");
                let _ = stream_file.write_all(nal_data);
                extracted_nal_units += 1;
            }
        }
    }

    drop(stream_file);

    if extracted_nal_units == 0 {
        let _ = fs::remove_file(&elementary_stream_path);
        return Err(MediaEngineError::CorruptMedia {
            path: damaged_path.display().to_string(),
            details: "Failed to extract valid H.264 video NAL units from damaged media payload."
                .to_string(),
        });
    }

    // 4. Resolve frame rate from reference track
    let fps_arg = compatibility_report
        .tracks
        .iter()
        .find(|t| t.media_type == "video")
        .and_then(|t| t.frame_rate)
        .map(|r| format!("{:.2}", r))
        .unwrap_or_else(|| "30".to_string());

    // 5. Mux into compliant ISO BMFF container using stream copy
    let ffmpeg_tool = discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default())?;
    let args = vec![
        "-y".to_string(),
        "-fflags".to_string(),
        "+genpts".to_string(),
        "-r".to_string(),
        fps_arg,
        "-i".to_string(),
        elementary_stream_path.to_string_lossy().to_string(),
        "-c:v".to_string(),
        "copy".to_string(),
        "-avoid_negative_ts".to_string(),
        "make_zero".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        output_path.to_string_lossy().to_string(),
    ];

    let spec = ProcessSpec::new(ffmpeg_tool.canonical_path, args, Duration::from_secs(60));
    let proc_res = run_managed_process(spec, cancellation).await;

    // Clean up temporary elementary stream
    let _ = fs::remove_file(&elementary_stream_path);

    let output = proc_res?;
    if output.exit_code != Some(0) {
        let err_snippet = &output.stderr;
        return Err(MediaEngineError::Internal(format!(
            "Muxing reconstructed stream failed: {err_snippet}"
        )));
    }

    Ok(EngineExecutionEvidence {
        engine_name: "Open Media Workbench Native Reconstructor".to_string(),
        engine_version: "1.0.0-native".to_string(),
        execution_mode: "SPS/PPS Donor Injection & Stream Reconstruction".to_string(),
        extracted_nal_units,
        injected_sps_bytes: sps.len(),
        injected_pps_bytes: pps.len(),
        raw_payload_bytes: mdat_payload_size,
        recovered_stream_count: 1,
    })
}

/// Executes untrunc CLI subprocess if configured.
async fn execute_untrunc_reconstruction(
    damaged_path: &Path,
    reference_path: &Path,
    output_path: &Path,
    cancellation: Option<&CancellationToken>,
) -> Result<EngineExecutionEvidence, MediaEngineError> {
    let which_res = std::process::Command::new("which")
        .arg("untrunc")
        .output()
        .map_err(|e| MediaEngineError::BinaryNotFound {
            tool: "untrunc".to_string(),
            searched_paths: vec![format!("System PATH (error: {e})")],
        })?;

    if !which_res.status.success() {
        return Err(MediaEngineError::BinaryNotFound {
            tool: "untrunc".to_string(),
            searched_paths: vec!["System PATH (untrunc binary not found)".to_string()],
        });
    }

    let untrunc_bin = PathBuf::from(String::from_utf8_lossy(&which_res.stdout).trim());
    let args = vec![
        reference_path.to_string_lossy().to_string(),
        damaged_path.to_string_lossy().to_string(),
    ];

    let spec = ProcessSpec::new(untrunc_bin, args, Duration::from_secs(120));
    let output = run_managed_process(spec, cancellation).await?;

    if output.exit_code != Some(0) {
        let err_snippet = &output.stderr;
        return Err(MediaEngineError::Internal(format!(
            "Untrunc execution failed: {err_snippet}"
        )));
    }

    // untrunc writes to <damaged_path>_fixed.mp4
    let default_fixed = damaged_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(
            "{}_fixed.mp4",
            damaged_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        ));

    if default_fixed.exists() {
        fs::rename(&default_fixed, output_path).map_err(|e| {
            MediaEngineError::Internal(format!("Failed to move untrunc fixed file to staging: {e}"))
        })?;
    }

    Ok(EngineExecutionEvidence {
        engine_name: "Untrunc CLI Adapter".to_string(),
        engine_version: "upstream".to_string(),
        execution_mode: "External Process moov donor reconstructor".to_string(),
        extracted_nal_units: 0,
        injected_sps_bytes: 0,
        injected_pps_bytes: 0,
        raw_payload_bytes: 0,
        recovered_stream_count: 1,
    })
}

/// Helper function to extract SPS and PPS parameter sets from a reference MP4/MOV file.
fn extract_sps_pps_from_reference(
    reference_path: &Path,
) -> Result<(Vec<u8>, Vec<u8>), MediaEngineError> {
    let mut file = File::open(reference_path).map_err(|e| MediaEngineError::InvalidPath {
        path: reference_path.display().to_string(),
        reason: format!("Cannot open reference file: {e}"),
    })?;

    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| MediaEngineError::CorruptMedia {
            path: reference_path.display().to_string(),
            details: e.to_string(),
        })?;

    // Search for avcC atom tag
    let pos = find_subsequence(&data, b"avcC").ok_or_else(|| MediaEngineError::CorruptMedia {
        path: reference_path.display().to_string(),
        details: "Reference file does not contain an avcC atom (H.264 codec configuration box)."
            .to_string(),
    })?;

    if pos < 4 {
        return Err(MediaEngineError::CorruptMedia {
            path: reference_path.display().to_string(),
            details: "Corrupted avcC box offset in reference".to_string(),
        });
    }

    let size =
        u32::from_be_bytes([data[pos - 4], data[pos - 3], data[pos - 2], data[pos - 1]]) as usize;
    let end = (pos - 4 + size).min(data.len());
    let payload = &data[pos + 4..end];

    if payload.len() < 8 {
        return Err(MediaEngineError::CorruptMedia {
            path: reference_path.display().to_string(),
            details: "Truncated avcC atom payload in reference".to_string(),
        });
    }

    let num_sps = (payload[5] & 0x1F) as usize;
    if num_sps == 0 {
        return Err(MediaEngineError::CorruptMedia {
            path: reference_path.display().to_string(),
            details: "avcC atom contains 0 Sequence Parameter Sets".to_string(),
        });
    }

    let sps_len = u16::from_be_bytes([payload[6], payload[7]]) as usize;
    if payload.len() < 8 + sps_len + 3 {
        return Err(MediaEngineError::CorruptMedia {
            path: reference_path.display().to_string(),
            details: "Truncated SPS bytes in avcC atom".to_string(),
        });
    }

    let sps = payload[8..8 + sps_len].to_vec();
    let pps_offset = 8 + sps_len;
    let num_pps = payload[pps_offset] as usize;

    if num_pps == 0 {
        return Err(MediaEngineError::CorruptMedia {
            path: reference_path.display().to_string(),
            details: "avcC atom contains 0 Picture Parameter Sets".to_string(),
        });
    }

    let pps_len = u16::from_be_bytes([payload[pps_offset + 1], payload[pps_offset + 2]]) as usize;
    if payload.len() < pps_offset + 3 + pps_len {
        return Err(MediaEngineError::CorruptMedia {
            path: reference_path.display().to_string(),
            details: "Truncated PPS bytes in avcC atom".to_string(),
        });
    }

    let pps = payload[pps_offset + 3..pps_offset + 3 + pps_len].to_vec();

    Ok((sps, pps))
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
