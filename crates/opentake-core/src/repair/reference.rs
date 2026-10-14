use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use tokio_util::sync::CancellationToken;

use crate::media::error::MediaEngineError;
use crate::media::probe::probe_media_file;

use super::container::identify_container;
use super::iso_bmff::scan_iso_bmff_structure;
use super::reconstruction_report::{
    ReferenceCompatibilityClassification, ReferenceCompatibilityReport, ReferenceTrackInfo,
    RECONSTRUCTION_REPORT_SCHEMA_VERSION,
};

/// Analyzes donor reference recording compatibility with a damaged recording.
///
/// Under no circumstances does this function modify either the damaged file or the reference file.
pub async fn analyze_reference_compatibility(
    damaged_path: &Path,
    damaged_asset_id: &str,
    reference_path: &Path,
    reference_asset_id: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<ReferenceCompatibilityReport, MediaEngineError> {
    let mut warnings = Vec::new();
    let mut unknown_fields = Vec::new();

    // 1. Check identity collision (cannot be identical file)
    let canonical_damaged =
        damaged_path
            .canonicalize()
            .map_err(|e| MediaEngineError::InvalidPath {
                path: damaged_path.display().to_string(),
                reason: format!("Failed to resolve damaged path: {e}"),
            })?;
    let canonical_reference =
        reference_path
            .canonicalize()
            .map_err(|e| MediaEngineError::InvalidPath {
                path: reference_path.display().to_string(),
                reason: format!("Failed to resolve reference path: {e}"),
            })?;

    if canonical_damaged == canonical_reference {
        return Ok(ReferenceCompatibilityReport {
            schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
            reference_asset_id: reference_asset_id.to_string(),
            damaged_asset_id: damaged_asset_id.to_string(),
            reference_container: "identical".to_string(),
            damaged_container: "identical".to_string(),
            tracks: Vec::new(),
            compatibility: ReferenceCompatibilityClassification::Incompatible,
            summary: "The damaged media file and the reference media file are the exact same file."
                .to_string(),
            warnings: vec![
                "Reconstruction requires a distinct, healthy reference recording from the same device/profile.".to_string(),
            ],
            unknown_fields,
        });
    }

    // 2. Damaged file inspection
    let damaged_meta =
        std::fs::metadata(&canonical_damaged).map_err(|e| MediaEngineError::InvalidPath {
            path: canonical_damaged.display().to_string(),
            reason: format!("Failed to read damaged metadata: {e}"),
        })?;
    let damaged_size = damaged_meta.len();

    let damaged_container_res =
        identify_container(&canonical_damaged, damaged_size).map_err(|e| {
            MediaEngineError::CorruptMedia {
                path: canonical_damaged.display().to_string(),
                details: e.to_string(),
            }
        })?;
    let damaged_container_name = damaged_container_res.container_info.format_type.clone();

    if !damaged_container_res.is_iso_bmff {
        return Ok(ReferenceCompatibilityReport {
            schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
            reference_asset_id: reference_asset_id.to_string(),
            damaged_asset_id: damaged_asset_id.to_string(),
            reference_container: "unknown".to_string(),
            damaged_container: damaged_container_name,
            tracks: Vec::new(),
            compatibility: ReferenceCompatibilityClassification::Incompatible,
            summary: "Damaged file is not an ISO BMFF container (MP4/MOV). Reference-based moov reconstruction is only applicable to ISO BMFF containers.".to_string(),
            warnings: vec!["Container signature does not match ISO BMFF / QuickTime.".to_string()],
            unknown_fields,
        });
    }

    let damaged_iso = scan_iso_bmff_structure(&canonical_damaged, damaged_size).map_err(|e| {
        MediaEngineError::CorruptMedia {
            path: canonical_damaged.display().to_string(),
            details: format!("Failed to scan damaged ISO BMFF: {e}"),
        }
    })?;

    if !damaged_iso.has_mdat {
        return Ok(ReferenceCompatibilityReport {
            schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
            reference_asset_id: reference_asset_id.to_string(),
            damaged_asset_id: damaged_asset_id.to_string(),
            reference_container: "unknown".to_string(),
            damaged_container: damaged_container_name,
            tracks: Vec::new(),
            compatibility: ReferenceCompatibilityClassification::Incompatible,
            summary: "Damaged file has no media data box (mdat). There is no video/audio payload to recover.".to_string(),
            warnings: vec!["Empty or zero-byte media payload detected.".to_string()],
            unknown_fields,
        });
    }

    // 3. Reference file inspection
    let ref_meta =
        std::fs::metadata(&canonical_reference).map_err(|e| MediaEngineError::InvalidPath {
            path: canonical_reference.display().to_string(),
            reason: format!("Failed to read reference metadata: {e}"),
        })?;
    let ref_size = ref_meta.len();

    let ref_container_res = identify_container(&canonical_reference, ref_size).map_err(|e| {
        MediaEngineError::CorruptMedia {
            path: canonical_reference.display().to_string(),
            details: e.to_string(),
        }
    })?;
    let ref_container_name = ref_container_res.container_info.format_type.clone();

    if !ref_container_res.is_iso_bmff {
        return Ok(ReferenceCompatibilityReport {
            schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
            reference_asset_id: reference_asset_id.to_string(),
            damaged_asset_id: damaged_asset_id.to_string(),
            reference_container: ref_container_name,
            damaged_container: damaged_container_name,
            tracks: Vec::new(),
            compatibility: ReferenceCompatibilityClassification::Incompatible,
            summary: "Reference file is not an ISO BMFF container (MP4/MOV).".to_string(),
            warnings: vec!["Reference must be a valid MP4 or MOV file.".to_string()],
            unknown_fields,
        });
    }

    let ref_iso = scan_iso_bmff_structure(&canonical_reference, ref_size).map_err(|e| {
        MediaEngineError::CorruptMedia {
            path: canonical_reference.display().to_string(),
            details: format!("Failed to scan reference ISO BMFF: {e}"),
        }
    })?;

    if !ref_iso.has_moov {
        return Ok(ReferenceCompatibilityReport {
            schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
            reference_asset_id: reference_asset_id.to_string(),
            damaged_asset_id: damaged_asset_id.to_string(),
            reference_container: ref_container_name,
            damaged_container: damaged_container_name,
            tracks: Vec::new(),
            compatibility: ReferenceCompatibilityClassification::Incompatible,
            summary:
                "The reference file is itself damaged or unfinalized; its moov atom is missing."
                    .to_string(),
            warnings: vec!["A healthy donor with a complete moov index is required.".to_string()],
            unknown_fields,
        });
    }

    // 4. Probe reference file via FFprobe
    let ref_probe = probe_media_file(None, &canonical_reference, ref_size, cancellation).await?;

    let mut tracks = Vec::new();
    for v in &ref_probe.video_streams {
        tracks.push(ReferenceTrackInfo {
            track_id: v.stream_index as u32,
            media_type: "video".to_string(),
            codec_name: v.codec_name.clone(),
            codec_profile: v.profile.clone(),
            resolution: Some(format!("{}x{}", v.width, v.height)),
            frame_rate: v.nominal_frame_rate.as_ref().map(|r| r.to_f64_lossy()),
            timebase: v.timebase.as_ref().map(|r| r.to_string()),
            sample_rate: None,
            channels: None,
            extradata_size: 0,
        });
    }

    for a in &ref_probe.audio_streams {
        tracks.push(ReferenceTrackInfo {
            track_id: a.stream_index as u32,
            media_type: "audio".to_string(),
            codec_name: a.codec_name.clone(),
            codec_profile: None,
            resolution: None,
            frame_rate: None,
            timebase: a.timebase.as_ref().map(|r| r.to_string()),
            sample_rate: Some(a.sample_rate),
            channels: Some(a.channels),
            extradata_size: 0,
        });
    }

    if tracks.is_empty() {
        return Ok(ReferenceCompatibilityReport {
            schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
            reference_asset_id: reference_asset_id.to_string(),
            damaged_asset_id: damaged_asset_id.to_string(),
            reference_container: ref_container_name,
            damaged_container: damaged_container_name,
            tracks,
            compatibility: ReferenceCompatibilityClassification::Incompatible,
            summary: "Reference media contains no decodable video or audio streams.".to_string(),
            warnings: vec![
                "Donor reference must contain at least one valid video or audio stream."
                    .to_string(),
            ],
            unknown_fields,
        });
    }

    // 5. Inspect damaged payload for plausible codec markers
    let has_h264_ref = tracks.iter().any(|t| t.codec_name == "h264");
    let _has_aac_ref = tracks.iter().any(|t| t.codec_name == "aac");

    let payload_check = inspect_damaged_payload_markers(&canonical_damaged, &damaged_iso);

    let (compatibility, summary) = if damaged_iso.has_moov {
        warnings.push("Damaged file already possesses a moov atom. Lossless container remux (STEP 13) may be sufficient.".to_string());
        (
            ReferenceCompatibilityClassification::PlausiblyCompatible,
            "Damaged file already contains a moov box. Reference reconstruction can be attempted if the moov is corrupted.".to_string(),
        )
    } else if payload_check.has_nal_units && has_h264_ref {
        (
            ReferenceCompatibilityClassification::Compatible,
            "Compatible donor: reference contains H.264 video matching NAL unit signatures detected in damaged media payload.".to_string(),
        )
    } else if payload_check.payload_bytes > 1024 {
        unknown_fields.push("Exact frame dimensions and audio sample layout cannot be verified until full reconstruction.".to_string());
        warnings.push("Missing moov in damaged file prevents exact bitstream comparison. Reconstruction will proceed on best-effort basis.".to_string());
        (
            ReferenceCompatibilityClassification::PlausiblyCompatible,
            "Plausible donor: reference provides valid ISO BMFF track parameters for damaged payload.".to_string(),
        )
    } else {
        (
            ReferenceCompatibilityClassification::InsufficientEvidence,
            "Damaged payload is too small (<1KB) to establish reliable codec signatures."
                .to_string(),
        )
    };

    Ok(ReferenceCompatibilityReport {
        schema_version: RECONSTRUCTION_REPORT_SCHEMA_VERSION,
        reference_asset_id: reference_asset_id.to_string(),
        damaged_asset_id: damaged_asset_id.to_string(),
        reference_container: ref_container_name,
        damaged_container: damaged_container_name,
        tracks,
        compatibility,
        summary,
        warnings,
        unknown_fields,
    })
}

struct PayloadCheckResult {
    has_nal_units: bool,
    payload_bytes: u64,
}

fn inspect_damaged_payload_markers(
    path: &Path,
    iso_scan: &super::iso_bmff::IsoBmffScanResult,
) -> PayloadCheckResult {
    let mut result = PayloadCheckResult {
        has_nal_units: false,
        payload_bytes: 0,
    };

    let mdat_box = iso_scan.boxes.iter().find(|b| &b.box_type == b"mdat");
    let (offset, size) = match mdat_box {
        Some(b) => (b.offset + b.header_size, b.payload_size),
        None => return result,
    };

    result.payload_bytes = size;
    if size < 32 {
        return result;
    }

    let Ok(mut file) = File::open(path) else {
        return result;
    };

    if file.seek(SeekFrom::Start(offset)).is_err() {
        return result;
    }

    let read_len = (size as usize).min(64 * 1024);
    let mut buf = vec![0u8; read_len];
    if file.read_exact(&mut buf).is_err() {
        return result;
    }

    // Look for Annex B start codes (0x000001 or 0x00000001) or length prefixes
    for i in 0..buf.len().saturating_sub(4) {
        if buf[i] == 0
            && buf[i + 1] == 0
            && (buf[i + 2] == 1 || (buf[i + 2] == 0 && buf[i + 3] == 1))
        {
            let nal_byte = if buf[i + 2] == 1 {
                buf[i + 3]
            } else {
                buf[i + 4]
            };
            let nal_type = nal_byte & 0x1F;
            if (1..=12).contains(&nal_type) {
                result.has_nal_units = true;
                break;
            }
        }
    }

    result
}
