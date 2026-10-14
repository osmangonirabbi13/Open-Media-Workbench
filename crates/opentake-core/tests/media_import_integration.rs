use std::fs;
use std::process::Command;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use opentake_core::media::compatibility::CompatibilityStatus;
use opentake_core::media::error::MediaEngineError;
use opentake_core::media::import::MediaLibrarySession;

/// Helper function to generate minimal synthetic test media using the host ffmpeg.
/// Returns false if ffmpeg is unavailable on the host.
fn generate_synthetic_media(
    output_path: &std::path::Path,
    vcodec: &str,
    acodec: Option<&str>,
    duration_secs: f32,
) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-f").arg("lavfi").arg("-i").arg(format!(
        "testsrc=duration={duration_secs}:size=320x240:rate=30"
    ));

    if let Some(acodec_name) = acodec {
        cmd.arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg(format!("sine=frequency=1000:duration={duration_secs}"))
            .arg("-c:a")
            .arg(acodec_name);
    } else {
        cmd.arg("-an");
    }

    cmd.arg("-c:v")
        .arg(vcodec)
        .arg("--") // Protection
        .arg(output_path);

    let status = cmd.status();
    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_import_synthetic_h264_aac_mp4() {
    let temp_dir = TempDir::new().unwrap();
    let mp4_path = temp_dir.path().join("synthetic_valid.mp4");

    if !generate_synthetic_media(&mp4_path, "libx264", Some("aac"), 1.0) {
        eprintln!("Skipping test: ffmpeg failed or not available to generate synthetic media");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_file(&mp4_path, None)
        .await
        .expect("Import failed");

    assert_eq!(asset.file_name, "synthetic_valid.mp4");
    assert_eq!(asset.compatibility.overall, CompatibilityStatus::Supported);
    assert!(asset.compatibility.can_edit_future);
    assert_eq!(asset.metadata.video_streams.len(), 1);
    assert_eq!(asset.metadata.video_streams[0].codec_name, "h264");
    assert_eq!(asset.metadata.video_streams[0].width, 320);
    assert_eq!(asset.metadata.video_streams[0].height, 240);
    assert_eq!(asset.metadata.audio_streams.len(), 1);
    assert_eq!(asset.metadata.audio_streams[0].codec_name, "aac");

    // Verify session state
    let assets = session.get_assets().await;
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].id, asset.id);
}

#[tokio::test]
async fn test_import_hyphen_filename_option_injection_defense() {
    let temp_dir = TempDir::new().unwrap();
    // Filename starting with a hyphen: "-dash_name.mp4"
    let hyphen_path = temp_dir.path().join("-dash_name.mp4");

    if !generate_synthetic_media(&hyphen_path, "libx264", None, 0.5) {
        eprintln!("Skipping test: ffmpeg failed or not available");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_file(&hyphen_path, None)
        .await
        .expect("Import of hyphen-prefixed file failed");

    assert_eq!(asset.file_name, "-dash_name.mp4");
    assert_eq!(asset.metadata.video_streams.len(), 1);
}

#[tokio::test]
async fn test_import_unicode_and_spaces_in_filename() {
    let temp_dir = TempDir::new().unwrap();
    let unicode_path = temp_dir.path().join("সংবাদ ও ভিডিও test 🎬.mp4");

    if !generate_synthetic_media(&unicode_path, "libx264", None, 0.5) {
        eprintln!("Skipping test: ffmpeg failed or not available");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_file(&unicode_path, None)
        .await
        .expect("Import of unicode filename failed");

    assert_eq!(asset.file_name, "সংবাদ ও ভিডিও test 🎬.mp4");
    assert_eq!(asset.metadata.video_streams.len(), 1);
}

#[tokio::test]
async fn test_duplicate_file_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let mp4_path = temp_dir.path().join("original.mp4");

    if !generate_synthetic_media(&mp4_path, "libx264", None, 0.5) {
        eprintln!("Skipping test: ffmpeg failed or not available");
        return;
    }

    let session = MediaLibrarySession::default();
    let _asset1 = session.import_file(&mp4_path, None).await.unwrap();

    // Second import of the identical file
    let err = session.import_file(&mp4_path, None).await.unwrap_err();
    assert!(matches!(err, MediaEngineError::DuplicateFile { .. }));
    assert_eq!(err.code(), "DUPLICATE_FILE");
}

#[tokio::test]
async fn test_corrupted_file_rejection() {
    let temp_dir = TempDir::new().unwrap();
    let corrupt_path = temp_dir.path().join("corrupted.mp4");
    fs::write(&corrupt_path, b"NotAValidMp4HeaderCorruptedDataHere!!").unwrap();

    let session = MediaLibrarySession::default();
    let err = session.import_file(&corrupt_path, None).await.unwrap_err();
    assert!(matches!(
        err,
        MediaEngineError::CorruptMedia { .. } | MediaEngineError::OutputParseFailed { .. }
    ));
}

#[tokio::test]
async fn test_import_cancellation() {
    let temp_dir = TempDir::new().unwrap();
    let mp4_path = temp_dir.path().join("cancel_test.mp4");

    if !generate_synthetic_media(&mp4_path, "libx264", None, 0.5) {
        eprintln!("Skipping test: ffmpeg failed or not available");
        return;
    }

    let session = MediaLibrarySession::default();
    let cancel_token = CancellationToken::new();
    cancel_token.cancel(); // Pre-cancelled

    let err = session
        .import_file(&mp4_path, Some(&cancel_token))
        .await
        .unwrap_err();
    assert!(matches!(err, MediaEngineError::Cancelled { .. }));
    assert_eq!(session.get_assets().await.len(), 0);
}

#[tokio::test]
async fn test_source_preservation_on_removal() {
    let temp_dir = TempDir::new().unwrap();
    let mp4_path = temp_dir.path().join("keep_me.mp4");

    if !generate_synthetic_media(&mp4_path, "libx264", None, 0.5) {
        eprintln!("Skipping test: ffmpeg failed or not available");
        return;
    }

    let initial_content = fs::read(&mp4_path).unwrap();

    let session = MediaLibrarySession::default();
    let asset = session.import_file(&mp4_path, None).await.unwrap();
    assert_eq!(session.get_assets().await.len(), 1);

    // Remove from library
    let removed = session.remove_asset(&asset.id).await;
    assert!(removed);
    assert_eq!(session.get_assets().await.len(), 0);

    // CRITICAL: Ensure original file is 100% untouched on disk
    assert!(mp4_path.exists());
    let current_content = fs::read(&mp4_path).unwrap();
    assert_eq!(initial_content, current_content);
}
