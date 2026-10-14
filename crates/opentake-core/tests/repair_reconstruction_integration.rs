use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use opentake_core::repair::{
    analyze_reference_compatibility, discover_reconstruction_capabilities,
    run_reconstruction_pipeline, ReconstructionRequest, ReconstructionStatus,
    ReferenceCompatibilityClassification,
};

fn compute_file_hash(path: &Path) -> String {
    let bytes = fs::read(path).expect("File must be readable to compute hash");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// Generates a synthetic valid MP4 clip with H.264 video using FFmpeg.
fn generate_synthetic_mp4(output_path: &Path, duration_secs: f32) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg(format!(
            "testsrc=duration={duration_secs}:size=320x240:rate=30"
        ))
        .arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("--")
        .arg(output_path);

    match cmd.status() {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

/// Creates a genuine unfinalized MP4 file by stripping the `moov` box from a valid MP4 file.
fn create_unfinalized_damaged_mp4(valid_mp4: &Path, damaged_output: &Path) -> bool {
    let Ok(data) = fs::read(valid_mp4) else {
        return false;
    };

    // Find the offset of the moov box
    let pos = data.windows(4).position(|w| w == b"moov");
    let Some(moov_pos) = pos else {
        return false;
    };

    // Cut right before the moov box (moov header starts 4 bytes before 'moov' tag)
    let cut_offset = moov_pos - 4;
    let damaged_data = &data[..cut_offset];

    fs::write(damaged_output, damaged_data).is_ok()
}

#[tokio::test]
async fn test_genuine_missing_moov_reconstruction() {
    let temp_dir = TempDir::new().unwrap();
    let ref_path = temp_dir.path().join("healthy_donor_reference.mp4");
    let damaged_path = temp_dir.path().join("unfinalized_damaged_recording.mp4");
    let staging_dir = temp_dir.path().join("staging_reconstruction");

    if !generate_synthetic_mp4(&ref_path, 1.0) {
        eprintln!("Skipping test: ffmpeg not available in test environment");
        return;
    }

    // Create real unfinalized recording missing moov
    assert!(
        create_unfinalized_damaged_mp4(&ref_path, &damaged_path),
        "Failed to create unfinalized damaged clip"
    );

    // Verify damaged clip is truly unfinalized (ffprobe fails with moov atom not found)
    let probe_cmd = Command::new("ffprobe").arg(&damaged_path).output();
    if let Ok(output) = probe_cmd {
        assert!(
            !output.status.success(),
            "Unfinalized file must not probe cleanly"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("moov atom not found"),
            "Error must confirm missing moov: {stderr}"
        );
    }

    let initial_damaged_hash = compute_file_hash(&damaged_path);
    let initial_ref_hash = compute_file_hash(&ref_path);

    let req = ReconstructionRequest {
        damaged_asset_id: "asset-damaged-1".to_string(),
        reference_asset_id: "asset-ref-1".to_string(),
        output_container_preference: Some("mp4".to_string()),
        selected_engine: Some("native".to_string()),
        diagnostic_report_id: None,
    };

    let report = run_reconstruction_pipeline(
        &damaged_path,
        "asset-damaged-1",
        "unfinalized_damaged_recording.mp4",
        &ref_path,
        "asset-ref-1",
        "healthy_donor_reference.mp4",
        &req,
        &staging_dir,
        None,
    )
    .await
    .expect("Reconstruction pipeline should succeed");

    // 1. Verify status and validation passed
    assert_eq!(
        report.processing_status,
        ReconstructionStatus::Completed,
        "Processing status must be Completed"
    );
    assert!(report.validation.passed, "Validation must pass");
    assert!(
        report.validation.is_playable,
        "Validation must confirm playable media"
    );
    assert!(
        report.output_file_path.is_some(),
        "Output candidate path must exist"
    );

    let out_path = report.output_file_path.unwrap();
    let out_file = Path::new(&out_path);
    assert!(
        out_file.exists(),
        "Reconstructed output file must physically exist"
    );
    assert!(
        fs::metadata(out_file).unwrap().len() > 0,
        "Output size must be > 0 bytes"
    );

    // 2. Decode verification: prove output decodes with zero errors using FFmpeg null-sink
    let decode_output = Command::new("ffmpeg")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(out_file)
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()
        .expect("Decode verification process should run");

    assert!(
        decode_output.status.success(),
        "FFmpeg decode verification failed: {}",
        String::from_utf8_lossy(&decode_output.stderr)
    );

    // 3. Immutability verification: neither damaged file nor reference file was altered
    let final_damaged_hash = compute_file_hash(&damaged_path);
    let final_ref_hash = compute_file_hash(&ref_path);
    assert_eq!(
        initial_damaged_hash, final_damaged_hash,
        "Zero-byte mutation violated: damaged file was modified!"
    );
    assert_eq!(
        initial_ref_hash, final_ref_hash,
        "Zero-byte mutation violated: reference file was modified!"
    );
}

#[tokio::test]
async fn test_reference_compatibility_identical_file_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let clip_path = temp_dir.path().join("same_clip.mp4");

    if !generate_synthetic_mp4(&clip_path, 0.5) {
        return;
    }

    let report =
        analyze_reference_compatibility(&clip_path, "asset-1", &clip_path, "asset-1", None)
            .await
            .expect("Compatibility analysis should run");

    assert_eq!(
        report.compatibility,
        ReferenceCompatibilityClassification::Incompatible
    );
    assert!(report.summary.contains("same file"));
}

#[tokio::test]
async fn test_missing_mdat_reconstruction_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let ref_path = temp_dir.path().join("ref_donor.mp4");
    let damaged_no_mdat = temp_dir.path().join("no_mdat.mp4");
    let staging_dir = temp_dir.path().join("staging_no_mdat");

    if !generate_synthetic_mp4(&ref_path, 0.5) {
        return;
    }

    // Write empty MP4 with only ftyp box (no mdat at all)
    let mut data = Vec::new();
    data.extend_from_slice(&20u32.to_be_bytes());
    data.extend_from_slice(b"ftypisom\0\0\x02\0isom");
    fs::write(&damaged_no_mdat, data).unwrap();

    let req = ReconstructionRequest {
        damaged_asset_id: "asset-no-mdat".to_string(),
        reference_asset_id: "asset-ref".to_string(),
        output_container_preference: Some("mp4".to_string()),
        selected_engine: Some("native".to_string()),
        diagnostic_report_id: None,
    };

    let report = run_reconstruction_pipeline(
        &damaged_no_mdat,
        "asset-no-mdat",
        "no_mdat.mp4",
        &ref_path,
        "asset-ref",
        "ref_donor.mp4",
        &req,
        &staging_dir,
        None,
    )
    .await
    .expect("Pipeline should handle empty mdat gracefully");

    assert_eq!(report.processing_status, ReconstructionStatus::Failed);
    assert!(!report.validation.passed);
}

#[tokio::test]
async fn test_cancellation_during_reconstruction() {
    let temp_dir = TempDir::new().unwrap();
    let ref_path = temp_dir.path().join("ref_cancel.mp4");
    let damaged_path = temp_dir.path().join("damaged_cancel.mp4");
    let staging_dir = temp_dir.path().join("staging_cancel");

    if !generate_synthetic_mp4(&ref_path, 0.5) {
        return;
    }
    assert!(create_unfinalized_damaged_mp4(&ref_path, &damaged_path));

    let token = CancellationToken::new();
    token.cancel(); // Cancel immediately

    let req = ReconstructionRequest {
        damaged_asset_id: "asset-d-cancel".to_string(),
        reference_asset_id: "asset-r-cancel".to_string(),
        output_container_preference: Some("mp4".to_string()),
        selected_engine: Some("native".to_string()),
        diagnostic_report_id: None,
    };

    let result = run_reconstruction_pipeline(
        &damaged_path,
        "asset-d-cancel",
        "damaged_cancel.mp4",
        &ref_path,
        "asset-r-cancel",
        "ref_cancel.mp4",
        &req,
        &staging_dir,
        Some(&token),
    )
    .await;

    assert!(result.is_err(), "Pre-cancelled pipeline must return error");
}

#[tokio::test]
async fn test_reconstruction_capabilities_discovery() {
    let caps = discover_reconstruction_capabilities();
    assert!(!caps.is_empty(), "Capabilities list must not be empty");

    let native = caps.iter().find(|c| c.engine_id == "native");
    assert!(native.is_some(), "Native engine must always be present");
    assert!(
        native.unwrap().is_available,
        "Native engine must be available"
    );
}
