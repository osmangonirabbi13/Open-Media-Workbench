use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::path::Path;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use opentake_core::repair::{
    identify_container, run_diagnostics, scan_iso_bmff_structure, AnalysisMode, AnalysisStatus,
};

/// Computes SHA256 of file bytes to verify read-only guarantee.
fn compute_file_sha256(path: &Path) -> String {
    let bytes = fs::read(path).expect("Failed to read file for hash");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

#[test]
fn test_empty_file_signature() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.mp4");
    File::create(&file_path).unwrap();

    let res = identify_container(&file_path, 0).unwrap();
    assert_eq!(res.container_info.format_type, "Empty");
    assert!(!res.container_info.signature_matched);
    assert!(res
        .findings
        .iter()
        .any(|f| f.code == "CONTAINER_FILE_EMPTY"));
}

#[test]
fn test_truncated_header_under_12_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("truncated.mp4");
    fs::write(&file_path, b"1234").unwrap();

    let res = identify_container(&file_path, 4).unwrap();
    assert_eq!(res.container_info.format_type, "Truncated");
    assert!(res
        .findings
        .iter()
        .any(|f| f.code == "CONTAINER_HEADER_TRUNCATED"));
}

#[test]
fn test_healthy_mp4_box_hierarchy() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("healthy.mp4");

    let mut data = Vec::new();
    // 1. ftyp box (20 bytes): size=20, type="ftyp", major_brand="isom", minor_ver=0x200, compat="isom"
    data.extend_from_slice(&20u32.to_be_bytes());
    data.extend_from_slice(b"ftypisom\0\0\x02\0isom");

    // 2. moov box (20 bytes): size=20, type="moov", dummy payload (12 bytes)
    data.extend_from_slice(&20u32.to_be_bytes());
    data.extend_from_slice(b"moovpayloadbytes");

    // 3. mdat box (24 bytes): size=24, type="mdat", video packets
    data.extend_from_slice(&24u32.to_be_bytes());
    data.extend_from_slice(b"mdat0123456789abcdef");

    fs::write(&file_path, &data).unwrap();

    let container_res = identify_container(&file_path, data.len() as u64).unwrap();
    assert!(container_res.is_iso_bmff);
    assert_eq!(container_res.container_info.format_type, "MP4 (ISO BMFF)");

    let bmff_res = scan_iso_bmff_structure(&file_path, data.len() as u64).unwrap();
    assert_eq!(bmff_res.boxes.len(), 3);
    assert!(bmff_res.has_ftyp);
    assert!(bmff_res.has_moov);
    assert!(bmff_res.has_mdat);
    assert!(!bmff_res.is_fragmented);
    // moov was before mdat, so no MOOV_AT_END_OF_FILE
    assert!(!bmff_res
        .findings
        .iter()
        .any(|f| f.code == "MP4_MOOV_AT_END_OF_FILE"));
}

#[test]
fn test_healthy_mov_quicktime_brand() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("quicktime.mov");

    let mut data = Vec::new();
    // ftyp box with qt brand
    data.extend_from_slice(&20u32.to_be_bytes());
    data.extend_from_slice(b"ftypqt  \0\0\x02\0qt  ");
    // moov box
    data.extend_from_slice(&8u32.to_be_bytes());
    data.extend_from_slice(b"moov");
    // mdat box
    data.extend_from_slice(&12u32.to_be_bytes());
    data.extend_from_slice(b"mdat1234");

    fs::write(&file_path, &data).unwrap();

    let container_res = identify_container(&file_path, data.len() as u64).unwrap();
    assert!(container_res.is_iso_bmff);
    assert_eq!(container_res.container_info.format_type, "QuickTime / MOV");
}

#[test]
fn test_fragmented_mp4_detection() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("fragmented.mp4");

    let mut data = Vec::new();
    // ftyp
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"ftypiso6\0\0\0\0");
    // moov
    data.extend_from_slice(&8u32.to_be_bytes());
    data.extend_from_slice(b"moov");
    // moof (movie fragment header)
    data.extend_from_slice(&8u32.to_be_bytes());
    data.extend_from_slice(b"moof");
    // mdat
    data.extend_from_slice(&12u32.to_be_bytes());
    data.extend_from_slice(b"mdatdata");

    fs::write(&file_path, &data).unwrap();

    let bmff_res = scan_iso_bmff_structure(&file_path, data.len() as u64).unwrap();
    assert!(bmff_res.has_moof);
    assert!(bmff_res.is_fragmented);
    assert!(bmff_res
        .findings
        .iter()
        .any(|f| f.code == "MP4_FRAGMENTED_CONTAINER"));
}

#[test]
fn test_missing_moov_atom_detection_and_step14_recommendation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("unfinalized_crash.mp4");

    let mut data = Vec::new();
    // ftyp
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"ftypmp42\0\0\0\0");
    // mdat payload directly without moov (camera crashed during recording)
    data.extend_from_slice(&32u32.to_be_bytes());
    data.extend_from_slice(b"mdatthisisunfinalizedvideodata!!");

    fs::write(&file_path, &data).unwrap();

    let bmff_res = scan_iso_bmff_structure(&file_path, data.len() as u64).unwrap();
    assert!(bmff_res.has_mdat);
    assert!(!bmff_res.has_moov);
    assert!(bmff_res
        .findings
        .iter()
        .any(|f| f.code == "MP4_MOOV_MISSING"));
}

#[test]
fn test_truncated_mdat_box_size_exceeding_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("truncated_payload.mp4");

    let mut data = Vec::new();
    // ftyp (16 bytes)
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"ftypmp42\0\0\0\0");
    // mdat claims 10,000 bytes, but file ends right after header
    data.extend_from_slice(&10000u32.to_be_bytes());
    data.extend_from_slice(b"mdat1234");

    fs::write(&file_path, &data).unwrap();

    let bmff_res = scan_iso_bmff_structure(&file_path, data.len() as u64).unwrap();
    assert!(bmff_res
        .findings
        .iter()
        .any(|f| f.code == "MP4_BOX_TRUNCATED_BEFORE_END"));
}

#[test]
fn test_extended_64bit_box_size_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large_box.mp4");

    let mut data = Vec::new();
    // ftyp (16 bytes)
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"ftypisom\0\0\0\0");
    // mdat with 32-bit size = 1 (indicates 64-bit extended size), type = "mdat", 64-bit size = 32
    data.extend_from_slice(&1u32.to_be_bytes());
    data.extend_from_slice(b"mdat");
    data.extend_from_slice(&32u64.to_be_bytes());
    data.extend_from_slice(b"sixteenbytesdata");

    fs::write(&file_path, &data).unwrap();

    let bmff_res = scan_iso_bmff_structure(&file_path, data.len() as u64).unwrap();
    assert_eq!(bmff_res.boxes.len(), 2);
    let mdat_box = &bmff_res.boxes[1];
    assert!(mdat_box.is_extended);
    assert_eq!(mdat_box.total_size, 32);
    assert_eq!(mdat_box.payload_size, 16);
}

#[test]
fn test_size_zero_box_extending_to_eof() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("zero_box.mp4");

    let mut data = Vec::new();
    // ftyp (16 bytes)
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"ftypisom\0\0\0\0");
    // mdat with 32-bit size = 0 (extends to end of file)
    data.extend_from_slice(&0u32.to_be_bytes());
    data.extend_from_slice(b"mdatremainderpayloadtoendoffile");

    fs::write(&file_path, &data).unwrap();

    let bmff_res = scan_iso_bmff_structure(&file_path, data.len() as u64).unwrap();
    assert_eq!(bmff_res.boxes.len(), 2);
    let mdat_box = &bmff_res.boxes[1];
    assert!(mdat_box.is_to_eof);
    assert_eq!(mdat_box.total_size, 35); // 4 bytes size + 4 bytes type + 27 bytes payload
}

#[test]
fn test_healthy_mkv_ebml_signature() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("sample.mkv");

    let mut data = Vec::new();
    // EBML magic: 0x1A 0x45 0xDF 0xA3
    data.extend_from_slice(&[0x1a, 0x45, 0xdf, 0xa3]);
    data.extend_from_slice(b"\x01\x00\x00\x1f\x42\x86\x81\x01\x42\xf7\x81\x01");

    fs::write(&file_path, &data).unwrap();

    let res = identify_container(&file_path, data.len() as u64).unwrap();
    assert!(res.is_mkv);
    assert_eq!(res.container_info.format_type, "Matroska / WebM (EBML)");
    assert!(res.container_info.signature_matched);
}

#[test]
fn test_healthy_avi_riff_signature() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("sample.avi");

    let mut data = Vec::new();
    // RIFF (4 bytes) + length (4 bytes) + AVI (4 bytes)
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&12u32.to_le_bytes()); // 12 bytes chunk payload (total file 20 bytes)
    data.extend_from_slice(b"AVI ");
    data.extend_from_slice(b"LIST\0\0\0\0");

    fs::write(&file_path, &data).unwrap();

    let res = identify_container(&file_path, data.len() as u64).unwrap();
    assert!(res.is_avi);
    assert_eq!(
        res.container_info.format_type,
        "Audio Video Interleave (AVI)"
    );
    assert!(res.container_info.signature_matched);
}

#[test]
fn test_truncated_avi_riff_size() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("truncated.avi");

    let mut data = Vec::new();
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&50000u32.to_le_bytes()); // Claims 50,000 bytes
    data.extend_from_slice(b"AVI ");

    fs::write(&file_path, &data).unwrap();

    let res = identify_container(&file_path, data.len() as u64).unwrap();
    assert!(res.is_avi);
    assert!(res.findings.iter().any(|f| f.code == "AVI_RIFF_TRUNCATED"));
}

#[test]
fn test_random_bytes_unsupported_signature() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("random.bin");

    let data = vec![
        0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
    ];
    fs::write(&file_path, &data).unwrap();

    let res = identify_container(&file_path, data.len() as u64).unwrap();
    assert!(!res.is_iso_bmff);
    assert!(!res.is_mkv);
    assert!(!res.is_avi);
    assert_eq!(res.container_info.format_type, "Unknown");
    assert!(res
        .findings
        .iter()
        .any(|f| f.code == "CONTAINER_UNKNOWN_SIGNATURE"));
}

#[tokio::test]
async fn test_diagnostics_read_only_guarantee() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("readonly_test.mp4");

    let mut data = Vec::new();
    data.extend_from_slice(&20u32.to_be_bytes());
    data.extend_from_slice(b"ftypisom\0\0\x02\0isom");
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"moovtestpayload!");
    data.extend_from_slice(&16u32.to_be_bytes());
    data.extend_from_slice(b"mdatvideoframe!!");

    fs::write(&file_path, &data).unwrap();

    let hash_before = compute_file_sha256(&file_path);

    // Run diagnostics
    let report = run_diagnostics(
        &file_path,
        "asset-123",
        "readonly_test.mp4",
        AnalysisMode::Quick,
        None,
    )
    .await;

    let hash_after = compute_file_sha256(&file_path);

    // INVARIANT: Original source file must be 100% byte-identical
    assert_eq!(hash_before, hash_after);
    assert_eq!(report.asset_id, "asset-123");
    assert_eq!(report.analysis_status, AnalysisStatus::Completed);
}

#[tokio::test]
async fn test_diagnostics_cancellation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("cancel_test.mp4");
    fs::write(&file_path, b"dummy data").unwrap();

    let token = CancellationToken::new();
    token.cancel(); // Pre-cancelled

    let report = run_diagnostics(
        &file_path,
        "asset-cancel",
        "cancel_test.mp4",
        AnalysisMode::Quick,
        Some(&token),
    )
    .await;

    assert_eq!(report.analysis_status, AnalysisStatus::Cancelled);
}

#[test]
fn test_proptest_fuzz_box_scanner_never_panics() {
    use proptest::prelude::*;

    let mut proptest_runner = proptest::test_runner::TestRunner::default();
    proptest_runner
        .run(
            &(proptest::collection::vec(any::<u8>(), 0..1024)),
            |bytes| {
                let temp_dir = TempDir::new().unwrap();
                let file_path = temp_dir.path().join("fuzz.mp4");
                fs::write(&file_path, &bytes).unwrap();

                // Must never panic or hang
                let _ = scan_iso_bmff_structure(&file_path, bytes.len() as u64);
                Ok(())
            },
        )
        .unwrap();
}
