use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use opentake_core::media::authorization::SourceProvenance;
use opentake_core::media::error::MediaEngineError;
use opentake_core::media::import::MediaLibrarySession;
use opentake_core::media::proxy::{compute_proxy_cache_key, ProxyEngine};
use opentake_core::media::range::{
    determine_mime_type, parse_byte_range, read_range_slice, MAX_RANGE_CHUNK_BYTES,
};
use sha2::{Digest, Sha256};

fn compute_file_hash(path: &Path) -> String {
    let bytes = fs::read(path).expect("File must be readable to compute hash");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

fn generate_synthetic_media(
    output_path: &Path,
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

    cmd.arg("-c:v").arg(vcodec).arg("--").arg(output_path);

    let status = cmd.status();
    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_byte_range_streaming_accuracy_and_bounds() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_sample.mp4");

    if !generate_synthetic_media(&file_path, "libx264", Some("aac"), 0.5) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let file_size = fs::metadata(&file_path).unwrap().len();
    assert!(file_size > 1000);

    // 1. Valid range bytes=0-499
    let range1 = parse_byte_range(Some("bytes=0-499"), file_size)
        .unwrap()
        .unwrap();
    assert_eq!(range1.start, 0);
    assert_eq!(range1.end, 499);
    assert_eq!(range1.length(), 500);

    let slice1 = read_range_slice(&file_path, &range1, MAX_RANGE_CHUNK_BYTES).unwrap();
    assert_eq!(slice1.len(), 500);

    // Verify bytes match raw file read
    let all_bytes = fs::read(&file_path).unwrap();
    assert_eq!(&slice1[..], &all_bytes[0..500]);

    // 2. Open-ended range bytes=500-
    let range2 = parse_byte_range(Some("bytes=500-"), file_size)
        .unwrap()
        .unwrap();
    assert_eq!(range2.start, 500);
    assert_eq!(range2.end, file_size - 1);

    // 3. Suffix range bytes=-100 (last 100 bytes)
    let range3 = parse_byte_range(Some("bytes=-100"), file_size)
        .unwrap()
        .unwrap();
    assert_eq!(range3.start, file_size - 100);
    assert_eq!(range3.end, file_size - 1);
    let slice3 = read_range_slice(&file_path, &range3, MAX_RANGE_CHUNK_BYTES).unwrap();
    assert_eq!(slice3.len(), 100);
    assert_eq!(&slice3[..], &all_bytes[(file_size as usize - 100)..]);

    // 4. Out-of-bounds range -> RangeNotSatisfiable (HTTP 416)
    let err = parse_byte_range(
        Some(&format!("bytes={}-{}", file_size + 10, file_size + 50)),
        file_size,
    )
    .unwrap_err();
    assert!(matches!(err, MediaEngineError::RangeNotSatisfiable { .. }));

    // 5. Malformed headers -> InvalidRange
    assert!(parse_byte_range(Some("characters=0-10"), file_size).is_err());
    assert!(parse_byte_range(Some("bytes=500-200"), file_size).is_err());
    assert!(parse_byte_range(Some("bytes=0-100,200-300"), file_size).is_err());

    // 6. MIME type
    assert_eq!(
        determine_mime_type(&file_path, Some("mov,mp4,m4a,3gp,3g2,mj2")),
        "video/mp4"
    );
}

#[tokio::test]
async fn test_preview_proxy_generation_and_source_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("proxy_cache");
    let source_path = temp_dir.path().join("original_source.mp4");

    if !generate_synthetic_media(&source_path, "libx264", Some("aac"), 0.5) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let initial_hash = compute_file_hash(&source_path);
    let initial_mtime = fs::metadata(&source_path).unwrap().modified().unwrap();

    let session = MediaLibrarySession::default();
    let asset = session
        .import_authorized_file(&source_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Import must succeed");

    let proxy_engine = ProxyEngine::new(cache_dir.clone(), None, None);

    // 1. Initial state: no proxy in cache
    let cached_before = proxy_engine.find_cached_proxy(&asset).await;
    assert!(cached_before.is_none());

    // 2. Generate preview proxy
    let mapping = proxy_engine
        .generate_proxy(&asset, None)
        .await
        .expect("Proxy generation must succeed");

    assert!(mapping.proxy_path.exists());
    let proxy_size = fs::metadata(&mapping.proxy_path).unwrap().len();
    assert!(proxy_size > 0);

    // 3. Cache reuse
    let cached_after = proxy_engine
        .find_cached_proxy(&asset)
        .await
        .expect("Cached proxy must now be found");
    assert_eq!(cached_after.proxy_path, mapping.proxy_path);

    // 4. CRITICAL INVARIANT: Original source file must be 100% byte-for-byte unchanged!
    let final_hash = compute_file_hash(&source_path);
    let final_mtime = fs::metadata(&source_path).unwrap().modified().unwrap();
    assert_eq!(
        initial_hash, final_hash,
        "Original file hash must be identical"
    );
    assert_eq!(
        initial_mtime, final_mtime,
        "Original file mtime must be identical"
    );
}

#[tokio::test]
async fn test_proxy_cancellation_and_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("proxy_cache");
    let source_path = temp_dir.path().join("cancelling_source.mp4");

    if !generate_synthetic_media(&source_path, "libx264", None, 0.5) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset = session
        .import_authorized_file(&source_path, SourceProvenance::NativeFileDialog, None)
        .await
        .expect("Import must succeed");

    let proxy_engine = ProxyEngine::new(cache_dir.clone(), None, None);

    // Token already cancelled before run
    let token = CancellationToken::new();
    token.cancel();

    let err = proxy_engine
        .generate_proxy(&asset, Some(&token))
        .await
        .unwrap_err();

    assert!(matches!(err, MediaEngineError::ProxyCancelled { .. }));

    // Verify no temporary files remain in cache_dir
    if cache_dir.exists() {
        for entry in fs::read_dir(&cache_dir).unwrap() {
            let path = entry.unwrap().path();
            assert!(
                !path.to_string_lossy().ends_with(".tmp.mp4"),
                "Temporary .tmp.mp4 file was not cleaned up!"
            );
        }
    }
}

#[tokio::test]
async fn test_cache_key_invalidation_on_source_modification() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("cache_key_test.mp4");

    if !generate_synthetic_media(&file_path, "libx264", None, 0.5) {
        eprintln!("Skipping: ffmpeg unavailable");
        return;
    }

    let session = MediaLibrarySession::default();
    let asset1 = session
        .import_authorized_file(&file_path, SourceProvenance::NativeFileDialog, None)
        .await
        .unwrap();

    let key1 = compute_proxy_cache_key(&asset1);

    // Create asset with simulated different file size
    let mut asset2 = asset1.clone();
    asset2.source.file_size += 1024;
    let key2 = compute_proxy_cache_key(&asset2);

    assert_ne!(key1, key2, "Modifying file size must invalidate cache key");

    // Create asset with simulated different mtime
    let mut asset3 = asset1.clone();
    asset3.source.modified_epoch_ms = Some(99999999);
    let key3 = compute_proxy_cache_key(&asset3);

    assert_ne!(key1, key3, "Modifying mtime must invalidate cache key");
}
