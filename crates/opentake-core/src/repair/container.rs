use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::report::{
    ByteRange, DetectedContainerInfo, DiagnosticFinding, FindingCategory, Severity,
};

/// Result of container signature identification.
#[derive(Debug, Clone)]
pub struct ContainerIdentificationResult {
    pub container_info: DetectedContainerInfo,
    pub findings: Vec<DiagnosticFinding>,
    pub is_iso_bmff: bool,
    pub is_mkv: bool,
    pub is_avi: bool,
}

/// Identifies container format using magic byte inspection and extension comparison.
pub fn identify_container(
    path: &Path,
    file_size: u64,
) -> Result<ContainerIdentificationResult, std::io::Error> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut findings = Vec::new();

    // 1. Check for empty file
    if file_size == 0 {
        findings.push(DiagnosticFinding {
            code: "CONTAINER_FILE_EMPTY".to_string(),
            severity: Severity::Critical,
            category: FindingCategory::ContainerSignature,
            summary: "Media file is zero bytes in length".to_string(),
            evidence: "Total file size is 0 bytes; no media data exists.".to_string(),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: Some(ByteRange {
                start: 0,
                length: 0,
            }),
            confidence: "Certain".to_string(),
            recommended_action: Some(
                "Verify if source file download or transfer completed.".to_string(),
            ),
        });

        return Ok(ContainerIdentificationResult {
            container_info: DetectedContainerInfo {
                format_type: "Empty".to_string(),
                extension,
                signature_matched: false,
                signature_details: "Zero-byte file".to_string(),
                mismatch_warning: Some("File is completely empty".to_string()),
            },
            findings,
            is_iso_bmff: false,
            is_mkv: false,
            is_avi: false,
        });
    }

    // 2. Check for truncated header (< 12 bytes)
    if file_size < 12 {
        findings.push(DiagnosticFinding {
            code: "CONTAINER_HEADER_TRUNCATED".to_string(),
            severity: Severity::Critical,
            category: FindingCategory::ContainerSignature,
            summary: "File header is severely truncated".to_string(),
            evidence: format!("File size is only {file_size} bytes (minimum 12 bytes required for container header)."),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: Some(ByteRange { start: 0, length: file_size }),
            confidence: "Certain".to_string(),
            recommended_action: Some("File is too small to contain valid headers; re-acquire source file.".to_string()),
        });

        return Ok(ContainerIdentificationResult {
            container_info: DetectedContainerInfo {
                format_type: "Truncated".to_string(),
                extension,
                signature_matched: false,
                signature_details: format!("Truncated header ({file_size} bytes)"),
                mismatch_warning: Some("File is truncated below minimal header size".to_string()),
            },
            findings,
            is_iso_bmff: false,
            is_mkv: false,
            is_avi: false,
        });
    }

    // 3. Read first 64 bytes for magic inspection
    let mut file = File::open(path)?;
    let mut magic = [0u8; 64];
    let bytes_read = file.read(&mut magic)?;
    let header = &magic[..bytes_read];

    let mut is_iso_bmff = false;
    let mut is_mkv = false;
    let mut is_avi = false;

    let mut detected_format = "Unknown".to_string();
    let mut signature_details = "No known container magic bytes recognized".to_string();

    // Check ISO BMFF (MP4 / QuickTime MOV)
    // Box structure: 4-byte size + 4-byte type
    if header.len() >= 8 {
        let box_type = &header[4..8];
        if box_type == b"ftyp" {
            is_iso_bmff = true;
            let major_brand = if header.len() >= 12 {
                String::from_utf8_lossy(&header[8..12]).to_string()
            } else {
                "Unknown".to_string()
            };

            if major_brand == "qt  " {
                detected_format = "QuickTime / MOV".to_string();
                signature_details =
                    format!("ISO BMFF ftyp box with QuickTime brand ({major_brand})");
            } else {
                detected_format = "MP4 (ISO BMFF)".to_string();
                signature_details = format!("ISO BMFF ftyp box with brand '{major_brand}'");
            }
        } else if box_type == b"moov"
            || box_type == b"mdat"
            || box_type == b"wide"
            || box_type == b"skip"
            || box_type == b"free"
        {
            // Unfinalized or legacy QuickTime file starting with moov/mdat
            is_iso_bmff = true;
            detected_format = "MP4/MOV (Headerless/Unfinalized)".to_string();
            let box_str = String::from_utf8_lossy(box_type);
            signature_details = format!(
                "ISO BMFF container starting directly with '{box_str}' atom at offset 0 (no ftyp)"
            );

            findings.push(DiagnosticFinding {
                code: "MP4_FTYP_MISSING".to_string(),
                severity: Severity::Warning,
                category: FindingCategory::ContainerSignature,
                summary: "ISO BMFF file lacks leading 'ftyp' file type box".to_string(),
                evidence: format!("Container starts directly with '{box_str}' box at byte 0."),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: Some(ByteRange {
                    start: 0,
                    length: 8,
                }),
                confidence: "High".to_string(),
                recommended_action: Some(
                    "Standard players may reject this file without container remuxing.".to_string(),
                ),
            });
        }
    }

    // Check Matroska / WebM (EBML header magic: 0x1A 0x45 0xDF 0xA3)
    if !is_iso_bmff && header.len() >= 4 && header[0..4] == [0x1a, 0x45, 0xdf, 0xa3] {
        is_mkv = true;
        detected_format = "Matroska / WebM (EBML)".to_string();
        signature_details = "Standard EBML Header Signature (0x1A45DFA3)".to_string();
    }

    // Check AVI (RIFF ... AVI )
    if !is_iso_bmff && !is_mkv && header.len() >= 12 && &header[0..4] == b"RIFF" {
        let riff_type = &header[8..12];
        if riff_type == b"AVI " || riff_type == b"AVIX" {
            is_avi = true;
            let riff_str = String::from_utf8_lossy(riff_type);
            detected_format = "Audio Video Interleave (AVI)".to_string();
            signature_details = format!("RIFF container with '{riff_str}' form type");

            // Check if RIFF chunk size exceeds file size
            let declared_riff_size =
                u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;
            if declared_riff_size + 8 > file_size {
                findings.push(DiagnosticFinding {
                    code: "AVI_RIFF_TRUNCATED".to_string(),
                    severity: Severity::Error,
                    category: FindingCategory::BoxStructure,
                    summary: "AVI RIFF chunk declared size exceeds physical file length".to_string(),
                    evidence: format!(
                        "RIFF header claims {declared_riff_size} bytes (total {} bytes expected), but file has only {file_size} bytes.",
                        declared_riff_size + 8
                    ),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: Some(ByteRange { start: 0, length: 8 }),
                    confidence: "High".to_string(),
                    recommended_action: Some("File was truncated before write completed; index rebuilding required.".to_string()),
                });
            }
        }
    }

    // 4. Compare file extension with detected signature
    let mut mismatch_warning = None;
    let expected_ext_for_format = match detected_format.as_str() {
        s if s.contains("MP4") => vec!["mp4", "m4v"],
        s if s.contains("MOV") => vec!["mov", "qt"],
        s if s.contains("Matroska") => vec!["mkv", "webm"],
        s if s.contains("AVI") => vec!["avi"],
        _ => vec![],
    };

    if !expected_ext_for_format.is_empty() && !expected_ext_for_format.contains(&extension.as_str())
    {
        let warn_msg = format!(
            "Filename extension '.{extension}' does not match detected container format ({detected_format})."
        );
        findings.push(DiagnosticFinding {
            code: "CONTAINER_EXTENSION_MISMATCH".to_string(),
            severity: Severity::Warning,
            category: FindingCategory::ContainerSignature,
            summary: "Filename extension mismatch with file bitstream signature".to_string(),
            evidence: warn_msg.clone(),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: Some(ByteRange {
                start: 0,
                length: 12,
            }),
            confidence: "High".to_string(),
            recommended_action: Some(format!(
                "Rename file extension to '.{}' to avoid parser confusion.",
                expected_ext_for_format[0]
            )),
        });
        mismatch_warning = Some(warn_msg);
    }

    // 5. If unknown signature, report finding
    if !is_iso_bmff && !is_mkv && !is_avi {
        findings.push(DiagnosticFinding {
            code: "CONTAINER_UNKNOWN_SIGNATURE".to_string(),
            severity: Severity::Error,
            category: FindingCategory::ContainerSignature,
            summary: "Unrecognized container signature bytes".to_string(),
            evidence: format!(
                "First 8 bytes ({:02X?}) do not match MP4/MOV, MKV, or AVI specifications.",
                &header[..header.len().min(8)]
            ),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: Some(ByteRange {
                start: 0,
                length: header.len().min(16) as u64,
            }),
            confidence: "High".to_string(),
            recommended_action: Some(
                "Check whether file is an unsupported format or non-media data.".to_string(),
            ),
        });
    }

    Ok(ContainerIdentificationResult {
        container_info: DetectedContainerInfo {
            format_type: detected_format,
            extension,
            signature_matched: is_iso_bmff || is_mkv || is_avi,
            signature_details,
            mismatch_warning,
        },
        findings,
        is_iso_bmff,
        is_mkv,
        is_avi,
    })
}
