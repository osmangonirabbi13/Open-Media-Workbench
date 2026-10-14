use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use opentake_core::repair::{
    finalize_recovered_file, run_recovery_pipeline, validate_recovery_output, RecoveryMethod,
    RecoveryRequest, RecoveryStatus, SalvageStreamType, ValidationLevel,
};

fn compute_file_hash(path: &Path) -> String {
    let bytes = fs::read(path).expect("File must be readable to compute hash");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

fn generate_synthetic_clip(
    output_path: &Path,
    container: &str,
    has_audio: bool,
    duration_secs: f32,
) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-f").arg("lavfi").arg("-i").arg(format!(
        "testsrc=duration={duration_secs}:size=320x240:rate=30"
    ));

    if has_audio {
        cmd.arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg(format!("sine=frequency=1000:duration={duration_secs}"))
            .arg("-c:a")
            .arg("aac");
    } else {
        cmd.arg("-an");
    }

    cmd.arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p");

    if container == "mp4" {
        cmd.arg("-movflags").arg("+faststart");
    }

    cmd.arg("--").arg(output_path);

    match cmd.status() {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_healthy_lossless_remux() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source_clip.mp4");
    let staging_dir = temp_dir.path().join("staging");

    if !generate_synthetic_clip(&source_path, "mp4", true, 0.5) {
        eprintln!("Skipping test: ffmpeg not available in test environment");
        return;
    }

    let initial_hash = compute_file_hash(&source_path);

    let req = RecoveryRequest {
        asset_id: "asset-healthy-1".to_string(),
        method: RecoveryMethod::LosslessRemux,
        output_container_preference: Some("mp4".to_string()),
        salvage_stream_type: None,
        custom_destination: None,
        diagnostic_report_id: None,
    };

    let report = run_recovery_pipeline(
        &source_path,
        "asset-healthy-1",
        "source_clip.mp4",
        &req,
        &staging_dir,
        None,
    )
    .await
    .expect("Recovery pipeline should succeed");

    assert_eq!(report.processing_status, RecoveryStatus::Completed);
    assert!(report.validation.passed);
    assert!(report.validation.is_playable);
    assert!(!report.validation.stream_loss_detected);
    assert_eq!(report.output_streams.len(), 2); // 1 video + 1 audio
    assert!(report.output_file_path.is_some());

    // Source immutability check
    assert_eq!(compute_file_hash(&source_path), initial_hash);
}

#[tokio::test]
async fn test_video_only_stream_salvage() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("audio_corrupt_source.mp4");
    let staging_dir = temp_dir.path().join("staging");

    if !generate_synthetic_clip(&source_path, "mp4", true, 0.5) {
        eprintln!("Skipping test: ffmpeg not available");
        return;
    }

    let req = RecoveryRequest {
        asset_id: "asset-salvage-v".to_string(),
        method: RecoveryMethod::StreamSalvage,
        output_container_preference: Some("mp4".to_string()),
        salvage_stream_type: Some(SalvageStreamType::VideoOnly),
        custom_destination: None,
        diagnostic_report_id: None,
    };

    let report = run_recovery_pipeline(
        &source_path,
        "asset-salvage-v",
        "audio_corrupt_source.mp4",
        &req,
        &staging_dir,
        None,
    )
    .await
    .expect("Video salvage should succeed");

    assert!(report.validation.passed);
    assert!(report.validation.is_playable);
    // Verified 1 video stream, 0 audio streams
    assert_eq!(report.output_streams.len(), 1);
    assert_eq!(report.output_streams[0].stream_type, "video");
}

#[tokio::test]
async fn test_audio_only_stream_salvage() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("video_corrupt_source.mp4");
    let staging_dir = temp_dir.path().join("staging");

    if !generate_synthetic_clip(&source_path, "mp4", true, 0.5) {
        eprintln!("Skipping test: ffmpeg not available");
        return;
    }

    let req = RecoveryRequest {
        asset_id: "asset-salvage-a".to_string(),
        method: RecoveryMethod::StreamSalvage,
        output_container_preference: Some("m4a".to_string()),
        salvage_stream_type: Some(SalvageStreamType::AudioOnly),
        custom_destination: None,
        diagnostic_report_id: None,
    };

    let report = run_recovery_pipeline(
        &source_path,
        "asset-salvage-a",
        "video_corrupt_source.mp4",
        &req,
        &staging_dir,
        None,
    )
    .await
    .expect("Audio salvage should succeed");

    assert!(report.validation.passed);
    assert_eq!(report.output_streams.len(), 1);
    assert_eq!(report.output_streams[0].stream_type, "audio");
}

#[tokio::test]
async fn test_timestamp_rebuild() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("pts_issue_source.mp4");
    let staging_dir = temp_dir.path().join("staging");

    if !generate_synthetic_clip(&source_path, "mp4", true, 0.5) {
        eprintln!("Skipping test: ffmpeg not available");
        return;
    }

    let req = RecoveryRequest {
        asset_id: "asset-ts-1".to_string(),
        method: RecoveryMethod::TimestampRebuild,
        output_container_preference: Some("mp4".to_string()),
        salvage_stream_type: None,
        custom_destination: None,
        diagnostic_report_id: None,
    };

    let report = run_recovery_pipeline(
        &source_path,
        "asset-ts-1",
        "pts_issue_source.mp4",
        &req,
        &staging_dir,
        None,
    )
    .await
    .expect("Timestamp rebuild should succeed");

    assert!(report.validation.passed);
    assert!(report.validation.is_playable);
}

#[tokio::test]
async fn test_missing_moov_atom_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("unfinalized_no_moov.mp4");
    let staging_dir = temp_dir.path().join("staging");

    // Synthesize unfinalized ISO BMFF with ftyp + mdat only (no moov)
    let mut data = Vec::new();
    // ftyp (20 bytes)
    data.extend_from_slice(&20u32.to_be_bytes());
    data.extend_from_slice(b"ftypisom\0\0\x02\0isom");
    // mdat (1000 bytes)
    data.extend_from_slice(&1000u32.to_be_bytes());
    data.extend_from_slice(b"mdat");
    data.extend(vec![0xAA; 992]);

    fs::write(&source_path, &data).unwrap();

    let req = RecoveryRequest {
        asset_id: "asset-unfinalized".to_string(),
        method: RecoveryMethod::LosslessRemux,
        output_container_preference: Some("mp4".to_string()),
        salvage_stream_type: None,
        custom_destination: None,
        diagnostic_report_id: None,
    };

    let report = run_recovery_pipeline(
        &source_path,
        "asset-unfinalized",
        "unfinalized_no_moov.mp4",
        &req,
        &staging_dir,
        None,
    )
    .await
    .expect("Pipeline should handle missing moov gracefully without erroring");

    assert_eq!(report.processing_status, RecoveryStatus::Failed);
    assert!(!report.validation.passed);
    assert!(report.output_file_path.is_none());
    assert!(report
        .limitations
        .iter()
        .any(|l| l.contains("STEP 14") && l.contains("moov")));
}

#[tokio::test]
async fn test_finalize_destination_already_exists_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let staged_file = temp_dir.path().join("staged.mp4");
    let dest_file = temp_dir.path().join("existing.mp4");
    let source_file = temp_dir.path().join("source.mp4");

    fs::write(&staged_file, b"recovered data").unwrap();
    fs::write(&dest_file, b"existing data").unwrap();
    fs::write(&source_file, b"source data").unwrap();

    let res = finalize_recovered_file(&staged_file, &dest_file, &source_file);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("already exists"));
}

#[tokio::test]
async fn test_finalize_source_destination_collision_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let staged_file = temp_dir.path().join("staged.mp4");
    let source_file = temp_dir.path().join("source.mp4");

    fs::write(&staged_file, b"recovered data").unwrap();
    fs::write(&source_file, b"source data").unwrap();

    // Trying to save directly over source
    let res = finalize_recovered_file(&staged_file, &source_file, &source_file);
    assert!(res.is_err());
}

#[tokio::test]
async fn test_finalize_symlink_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let staged_file = temp_dir.path().join("staged.mp4");
    let target_file = temp_dir.path().join("target.mp4");
    let symlink_dest = temp_dir.path().join("dest_symlink.mp4");
    let source_file = temp_dir.path().join("source.mp4");

    fs::write(&staged_file, b"recovered data").unwrap();
    fs::write(&source_file, b"source data").unwrap();

    // Create a symlink pointing to target_file
    symlink(&target_file, &symlink_dest).unwrap();

    let res = finalize_recovered_file(&staged_file, &symlink_dest, &source_file);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("symlink"));
}

#[tokio::test]
async fn test_finalize_success() {
    let temp_dir = TempDir::new().unwrap();
    let staged_file = temp_dir.path().join("staged.mp4");
    let dest_file = temp_dir.path().join("safe_output.mp4");
    let source_file = temp_dir.path().join("source.mp4");

    fs::write(&staged_file, b"recovered media payload").unwrap();
    fs::write(&source_file, b"original media").unwrap();

    let res = finalize_recovered_file(&staged_file, &dest_file, &source_file);
    assert!(res.is_ok());
    assert!(dest_file.exists());
    assert_eq!(fs::read(&dest_file).unwrap(), b"recovered media payload");
}

#[tokio::test]
async fn test_empty_output_validation() {
    let temp_dir = TempDir::new().unwrap();
    let empty_file = temp_dir.path().join("empty_output.mp4");
    File::create(&empty_file).unwrap();

    let outcome =
        validate_recovery_output(&empty_file, 1, 1, ValidationLevel::SampledDecode, None).await;

    assert_eq!(outcome.status, RecoveryStatus::Failed);
    assert!(!outcome.validation_result.passed);
    assert!(!outcome.validation_result.is_playable);
}

#[tokio::test]
async fn test_cancellation_during_pipeline() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("source.mp4");
    let staging_dir = temp_dir.path().join("staging");

    if !generate_synthetic_clip(&source_path, "mp4", true, 0.5) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let token = CancellationToken::new();
    token.cancel(); // Pre-cancelled

    let req = RecoveryRequest {
        asset_id: "asset-cancel".to_string(),
        method: RecoveryMethod::LosslessRemux,
        output_container_preference: Some("mp4".to_string()),
        salvage_stream_type: None,
        custom_destination: None,
        diagnostic_report_id: None,
    };

    let res = run_recovery_pipeline(
        &source_path,
        "asset-cancel",
        "source.mp4",
        &req,
        &staging_dir,
        Some(&token),
    )
    .await;

    assert!(res.is_err());
}
