use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::report::{ByteRange, DiagnosticFinding, FindingCategory, Severity};

/// A scanned top-level ISO BMFF box header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedBox {
    pub box_type: [u8; 4],
    pub box_type_str: String,
    pub offset: u64,
    pub header_size: u64,
    pub total_size: u64,
    pub payload_size: u64,
    pub is_extended: bool,
    pub is_to_eof: bool,
}

/// Results of the ISO BMFF structural scan.
#[derive(Debug, Clone)]
pub struct IsoBmffScanResult {
    pub boxes: Vec<ScannedBox>,
    pub findings: Vec<DiagnosticFinding>,
    pub scanned_bytes: u64,
    pub has_ftyp: bool,
    pub has_moov: bool,
    pub has_mdat: bool,
    pub has_moof: bool,
    pub is_fragmented: bool,
}

/// Maximum number of top-level boxes to parse before stopping (prevents infinite loop on corrupt streams).
pub const MAX_TOP_LEVEL_BOXES: usize = 5000;

/// Scans top-level ISO BMFF boxes in a read-only, memory-bounded manner.
pub fn scan_iso_bmff_structure(
    path: &Path,
    file_size: u64,
) -> Result<IsoBmffScanResult, std::io::Error> {
    let mut file = File::open(path)?;
    let mut boxes = Vec::new();
    let mut findings = Vec::new();

    let mut current_offset: u64 = 0;
    let mut scanned_bytes: u64 = 0;

    let mut has_ftyp = false;
    let mut has_moov = false;
    let mut has_mdat = false;
    let mut has_moof = false;

    let mut moov_count = 0;
    let mut first_mdat_offset: Option<u64> = None;
    let mut first_moov_offset: Option<u64> = None;

    while current_offset < file_size && boxes.len() < MAX_TOP_LEVEL_BOXES {
        // Minimum header is 8 bytes (4 bytes size, 4 bytes type)
        let remaining = file_size.saturating_sub(current_offset);
        if remaining < 8 {
            findings.push(DiagnosticFinding {
                code: "MP4_HEADER_TRUNCATED_AT_OFFSET".to_string(),
                severity: Severity::Error,
                category: FindingCategory::BoxStructure,
                summary: "Incomplete box header at end of file".to_string(),
                evidence: format!("At offset {current_offset}, only {remaining} bytes remain (minimum 8 required for box header)."),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: Some(ByteRange { start: current_offset, length: remaining }),
                confidence: "Certain".to_string(),
                recommended_action: Some("Truncated trailing bytes may be removed or repaired via container re-indexing.".to_string()),
            });
            break;
        }

        // Seek to current box offset
        file.seek(SeekFrom::Start(current_offset))?;

        let mut header_buf = [0u8; 8];
        file.read_exact(&mut header_buf)?;

        let size_32 =
            u32::from_be_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]) as u64;
        let box_type = [header_buf[4], header_buf[5], header_buf[6], header_buf[7]];
        let box_type_str = String::from_utf8_lossy(&box_type).to_string();

        let mut header_size = 8u64;
        let mut total_size = size_32;
        let mut is_extended = false;
        let mut is_to_eof = false;

        // Check for extended 64-bit size
        if size_32 == 1 {
            is_extended = true;
            header_size = 16;
            if remaining < 16 {
                findings.push(DiagnosticFinding {
                    code: "MP4_EXTENDED_SIZE_TRUNCATED".to_string(),
                    severity: Severity::Critical,
                    category: FindingCategory::BoxStructure,
                    summary: format!("Truncated 64-bit extended size header for box '{box_type_str}'"),
                    evidence: format!("Box at offset {current_offset} declared extended size (1), but only {remaining} bytes remain."),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: Some(ByteRange { start: current_offset, length: remaining }),
                    confidence: "Certain".to_string(),
                    recommended_action: Some("File was severed mid-header.".to_string()),
                });
                break;
            }

            let mut ext_buf = [0u8; 8];
            file.read_exact(&mut ext_buf)?;
            total_size = u64::from_be_bytes(ext_buf);

            // Valid extended size must be at least 16 bytes
            if total_size < 16 {
                findings.push(DiagnosticFinding {
                    code: "MP4_INVALID_EXTENDED_SIZE".to_string(),
                    severity: Severity::Error,
                    category: FindingCategory::BoxStructure,
                    summary: format!("Invalid extended size value ({total_size} bytes) for box '{box_type_str}'"),
                    evidence: format!("Extended size at offset {current_offset} must be >= 16 bytes, but reported {total_size}."),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: Some(ByteRange { start: current_offset, length: 16 }),
                    confidence: "Certain".to_string(),
                    recommended_action: Some("Corrupted box size descriptor.".to_string()),
                });
                break;
            }
        } else if size_32 == 0 {
            // size == 0 means box extends to end of file
            is_to_eof = true;
            total_size = remaining;
        } else if size_32 < 8 {
            // Invalid box size (< 8 bytes)
            findings.push(DiagnosticFinding {
                code: "MP4_INVALID_BOX_SIZE".to_string(),
                severity: Severity::Error,
                category: FindingCategory::BoxStructure,
                summary: format!("Invalid box size ({size_32} bytes) for box '{box_type_str}'"),
                evidence: format!(
                    "Box size at offset {current_offset} cannot be less than 8 bytes."
                ),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: Some(ByteRange {
                    start: current_offset,
                    length: 8,
                }),
                confidence: "Certain".to_string(),
                recommended_action: Some(
                    "Structural corruption detected in ISO BMFF atom stream.".to_string(),
                ),
            });
            break;
        }

        // Verify total size does not exceed file bounds (unless it's an unfinalized size extending beyond EOF)
        if total_size > remaining {
            findings.push(DiagnosticFinding {
                code: "MP4_BOX_TRUNCATED_BEFORE_END".to_string(),
                severity: Severity::Critical,
                category: FindingCategory::BoxStructure,
                summary: format!("Box '{box_type_str}' truncated: claims {total_size} bytes, but file has {remaining} bytes left"),
                evidence: format!("At offset {current_offset}, box claims {total_size} bytes, exceeding remaining {remaining} bytes by {} bytes.", total_size - remaining),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: Some(ByteRange { start: current_offset, length: remaining }),
                confidence: "Certain".to_string(),
                recommended_action: Some("Recording cut off before container writing finished. Truncated atom payload requires salvage.".to_string()),
            });
        }

        let payload_size = total_size.saturating_sub(header_size);

        // Track key boxes
        match &box_type {
            b"ftyp" => {
                has_ftyp = true;
            }
            b"moov" => {
                has_moov = true;
                moov_count += 1;
                if first_moov_offset.is_none() {
                    first_moov_offset = Some(current_offset);
                }
            }
            b"mdat" => {
                has_mdat = true;
                if first_mdat_offset.is_none() {
                    first_mdat_offset = Some(current_offset);
                }
            }
            b"moof" => {
                has_moof = true;
            }
            _ => {}
        }

        boxes.push(ScannedBox {
            box_type,
            box_type_str: box_type_str.clone(),
            offset: current_offset,
            header_size,
            total_size,
            payload_size,
            is_extended,
            is_to_eof,
        });

        scanned_bytes = scanned_bytes.saturating_add(header_size);

        // Advance to next box offset using checked addition
        let advance_by = if total_size > 0 {
            total_size
        } else {
            remaining
        };
        match current_offset.checked_add(advance_by) {
            Some(next_offset) => {
                // If advance_by is 0 or doesn't move forward, break to avoid infinite loop
                if next_offset <= current_offset {
                    break;
                }
                current_offset = next_offset;
            }
            None => {
                findings.push(DiagnosticFinding {
                    code: "MP4_OFFSET_OVERFLOW".to_string(),
                    severity: Severity::Critical,
                    category: FindingCategory::BoxStructure,
                    summary: "64-bit integer overflow computing next box offset".to_string(),
                    evidence: format!("Offset {current_offset} + {advance_by} overflowed u64."),
                    affected_stream: None,
                    media_time_range_ms: None,
                    byte_range: Some(ByteRange {
                        start: current_offset,
                        length: 8,
                    }),
                    confidence: "Certain".to_string(),
                    recommended_action: Some("Corrupted box length header.".to_string()),
                });
                break;
            }
        }
    }

    let is_fragmented = has_moof;

    // Structural findings evaluation:
    // 1. Missing MOOV atom
    if !has_moov && has_mdat {
        findings.push(DiagnosticFinding {
            code: "MP4_MOOV_MISSING".to_string(),
            severity: Severity::Critical,
            category: FindingCategory::BoxStructure,
            summary: "Missing 'moov' atom header in MP4/MOV container".to_string(),
            evidence: format!(
                "Scanned {} top-level boxes: 'mdat' media payload was found (at offset {:?}), but no 'moov' atom was detected anywhere in the file.",
                boxes.len(),
                first_mdat_offset
            ),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some(
                "Typical symptom of unfinalized recording (crash, power loss, battery pull). Requires reference-based MOOV atom reconstruction (Scheduled for STEP 13).".to_string()
            ),
        });
    }

    // 2. Missing MDAT payload
    if has_moov && !has_mdat && !has_moof {
        findings.push(DiagnosticFinding {
            code: "MP4_MDAT_MISSING".to_string(),
            severity: Severity::Critical,
            category: FindingCategory::BoxStructure,
            summary: "Missing 'mdat' media payload box".to_string(),
            evidence: "File contains 'moov' metadata, but zero audio/video sample data ('mdat') was found.".to_string(),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: None,
            confidence: "Certain".to_string(),
            recommended_action: Some("Container metadata exists without media packets. Video payload cannot be recovered.".to_string()),
        });
    }

    // 3. Atom ordering assessment (moov after mdat)
    if let (Some(mdat_off), Some(moov_off)) = (first_mdat_offset, first_moov_offset) {
        if moov_off > mdat_off {
            findings.push(DiagnosticFinding {
                code: "MP4_MOOV_AT_END_OF_FILE".to_string(),
                severity: Severity::Info,
                category: FindingCategory::BoxStructure,
                summary: "Movie metadata ('moov') is located after media payload ('mdat')".to_string(),
                evidence: format!("moov atom is at offset {moov_off}, following mdat payload at offset {mdat_off}."),
                affected_stream: None,
                media_time_range_ms: None,
                byte_range: Some(ByteRange { start: moov_off, length: 8 }),
                confidence: "Certain".to_string(),
                recommended_action: Some("Standard for camera recordings; faststart optimization moves moov to file start for instant web streaming.".to_string()),
            });
        }
    }

    // 4. Duplicate moov or mdat atoms
    if moov_count > 1 {
        findings.push(DiagnosticFinding {
            code: "MP4_MULTIPLE_MOOV_ATOMS".to_string(),
            severity: Severity::Warning,
            category: FindingCategory::BoxStructure,
            summary: format!("Container contains multiple 'moov' atoms ({moov_count} found)"),
            evidence: format!(
                "{moov_count} distinct moov atom headers exist in the top-level container."
            ),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some(
                "May cause ambiguous track parsing; container remuxing recommended.".to_string(),
            ),
        });
    }

    // 5. Fragmented MP4 notification
    if is_fragmented {
        findings.push(DiagnosticFinding {
            code: "MP4_FRAGMENTED_CONTAINER".to_string(),
            severity: Severity::Info,
            category: FindingCategory::BoxStructure,
            summary: "Fragmented MP4 container detected (contains 'moof' movie fragments)"
                .to_string(),
            evidence:
                "Container utilizes progressive fragments ('moof' boxes) for live stream recording."
                    .to_string(),
            affected_stream: None,
            media_time_range_ms: None,
            byte_range: None,
            confidence: "High".to_string(),
            recommended_action: Some("Handled through fragmented segment demuxing.".to_string()),
        });
    }

    Ok(IsoBmffScanResult {
        boxes,
        findings,
        scanned_bytes,
        has_ftyp,
        has_moov,
        has_mdat,
        has_moof,
        is_fragmented,
    })
}
