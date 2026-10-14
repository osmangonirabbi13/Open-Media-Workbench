use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use opentake_core::media::authorization::SourceProvenance;
use opentake_core::media::error::MediaEngineError;
use opentake_core::media::import::MediaLibrarySession;
use opentake_core::media::waveform::WaveformEngine;
use sha2::{Digest, Sha256};

fn compute_file_hash(path: &Path) -> String {
    let bytes = fs::read(path).expect("File must be readable to compute hash");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// Generates synthetic test fixtures with FFmpeg
fn generate_synthetic_audio_media(
    output_path: &Path,
    duration_secs: f32,
    audio_spec: Option<&str>,
    sample_rate: u32,
    channels: u32,
) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-f").arg("lavfi").arg("-i").arg(format!(
        "testsrc=duration={duration_secs}:size=160x120:rate=15"
    ));

    if let Some(spec) = audio_spec {
        cmd.arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg(format!("{spec}:duration={duration_secs}"))
            .arg("-c:a")
            .arg("aac")
            .arg("-ar")
            .arg(sample_rate.to_string())
            .arg("-ac")
            .arg(channels.to_string());
    } else {
        cmd.arg("-an");
    }

    cmd.arg("-c:v")
        .arg("libx264")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("--")
        .arg(output_path);

    let status = cmd.status();
    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_waveform_mono_sine_extraction_and_bounds() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let media_path = temp_dir.path().join("sine_mono.mp4");
    if !generate_synthetic_audio_media(&media_path, 1.0, Some("sine=frequency=1000"), 48000, 1) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let initial_hash = compute_file_hash(&media_path);

    let session = MediaLibrarySession::default();
    let asset = session
        .import_authorized_file(&media_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Import must succeed");

    assert!(!asset.metadata.audio_streams.is_empty());

    let engine = WaveformEngine::new(cache_dir.clone(), None);

    // Extract waveform
    let waveform = engine
        .extract_waveform(&asset, 0, None)
        .await
        .expect("Extraction should succeed");

    assert_eq!(waveform.asset_id, asset.id);
    assert_eq!(waveform.sample_rate, 16_000);
    assert_eq!(waveform.channels, 1);
    assert_eq!(waveform.levels.len(), 3);

    // Sine wave should have ~100 bins for 1 second at LOD 0 (AAC priming samples may yield 101-105 bins)
    let lod0 = &waveform.levels[0];
    assert!(lod0.bins.len() >= 100 && lod0.bins.len() <= 105);

    // Full-scale sine wave in steady state (e.g. bin 20, past AAC priming samples) should have peak ~1.0, min ~ -1.0, RMS ~ 0.707
    let steady_bin = lod0.bins[20];
    eprintln!(
        "steady_bin: min={}, max={}, rms={}",
        steady_bin.min, steady_bin.max, steady_bin.rms
    );
    assert!(steady_bin.max > 0.05 && steady_bin.max <= 1.0);
    assert!(steady_bin.min < -0.05 && steady_bin.min >= -1.0);
    assert!(steady_bin.rms > 0.05 && steady_bin.rms <= 1.0);

    // Verify source file was NOT mutated
    assert_eq!(
        compute_file_hash(&media_path),
        initial_hash,
        "Source media must never be modified"
    );
}

#[tokio::test]
async fn test_waveform_stereo_audio_and_resampling() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let media_path = temp_dir.path().join("stereo_44k.mp4");
    if !generate_synthetic_audio_media(&media_path, 1.0, Some("sine=frequency=440"), 44100, 2) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_authorized_file(&media_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Import must succeed");

    let engine = WaveformEngine::new(cache_dir, None);

    let waveform = engine
        .extract_waveform(&asset, 0, None)
        .await
        .expect("Extraction should downmix stereo to mono and resample to 16kHz");

    assert_eq!(waveform.sample_rate, 16_000);
    assert_eq!(waveform.channels, 1);
    assert!(waveform.levels[0].bins.len() >= 100 && waveform.levels[0].bins.len() <= 105);
}

#[tokio::test]
async fn test_waveform_no_audio_video_handling() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let media_path = temp_dir.path().join("no_audio.mp4");
    if !generate_synthetic_audio_media(&media_path, 0.5, None, 44100, 0) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_authorized_file(&media_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Import must succeed");

    assert!(asset.metadata.audio_streams.is_empty());

    let engine = WaveformEngine::new(cache_dir, None);

    let res = engine.extract_waveform(&asset, 0, None).await;
    match res {
        Err(MediaEngineError::NoAudioTrack { id }) => {
            assert_eq!(id, asset.id);
        }
        other => panic!("Expected NoAudioTrack error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_waveform_cache_reuse_and_invalidation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let media_path = temp_dir.path().join("cached_audio.mp4");
    if !generate_synthetic_audio_media(&media_path, 0.8, Some("sine=frequency=800"), 48000, 1) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_authorized_file(&media_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Import must succeed");

    let engine = WaveformEngine::new(cache_dir.clone(), None);

    // First run extracts & creates cache
    let initial = engine
        .extract_waveform(&asset, 0, None)
        .await
        .expect("First run extracts");

    // Second run should load directly from cache
    let cached = engine
        .find_cached_waveform(&asset, 0)
        .expect("Cache query should succeed")
        .expect("Cache entry should exist");

    assert_eq!(cached.total_samples, initial.total_samples);
    assert_eq!(cached.levels.len(), initial.levels.len());
    assert_eq!(cached.levels[0].bins.len(), initial.levels[0].bins.len());

    // If source file is modified, cache key must change
    let old_key = engine.compute_cache_key(&asset, 0);
    // Append 1 byte to source file
    let mut bytes = fs::read(&media_path).unwrap();
    bytes.push(0xAA);
    fs::write(&media_path, &bytes).unwrap();

    // Re-import to update metadata
    let session2 = MediaLibrarySession::default();
    let asset2 = session2
        .import_authorized_file(&media_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Re-import must succeed");

    let new_key = engine.compute_cache_key(&asset2, 0);
    assert_ne!(
        old_key, new_key,
        "Cache key must invalidate when file size or modification time changes"
    );
}

#[tokio::test]
async fn test_waveform_cancellation_and_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let media_path = temp_dir.path().join("long_audio.mp4");
    // Generate 3 seconds
    if !generate_synthetic_audio_media(&media_path, 3.0, Some("sine=frequency=500"), 48000, 1) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_authorized_file(&media_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Import must succeed");

    let engine = WaveformEngine::new(cache_dir.clone(), None);

    // Cancel token immediately
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let res = engine
        .extract_waveform(&asset, 0, Some(&cancellation))
        .await;

    match res {
        Err(MediaEngineError::WaveformCancelled { id }) => {
            assert_eq!(id, asset.id);
        }
        other => panic!("Expected WaveformCancelled error, got {other:?}"),
    }

    // Verify no stray `.tmp.otwf` files remain
    let cache_key = engine.compute_cache_key(&asset, 0);
    let temp_file = cache_dir.join(format!("{cache_key}.tmp.otwf"));
    assert!(
        !temp_file.exists(),
        "Temporary waveform file must be cleaned up on cancellation"
    );
}
