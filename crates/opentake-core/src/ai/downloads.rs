//! Streaming Network-to-Disk Download Engine with Continuous Hashing and Cancellation.

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tokio::sync::watch;

use super::catalog::is_trusted_download_url;
use super::errors::AiError;
use super::models::{AiDownloadState, AiModelDownloadProgress, AiModelManifest};
use super::verification::constant_time_eq_str;

/// Maximum buffer tolerance (1 MB) above expected size before failing.
const SIZE_OVERFLOW_TOLERANCE_BYTES: u64 = 1_048_576;

/// Executes a verified streaming download of an AI model artifact.
pub async fn download_model_artifact<F>(
    manifest: &AiModelManifest,
    staging_path: &Path,
    cancel_rx: watch::Receiver<bool>,
    progress_callback: F,
) -> Result<(String, u64), AiError>
where
    F: Fn(AiModelDownloadProgress) + Send + Sync + 'static,
{
    let url = manifest
        .artifact_url
        .as_deref()
        .ok_or_else(|| AiError::ModelNotInstallable(manifest.id.clone()))?;

    if !is_trusted_download_url(url) {
        return Err(AiError::UntrustedUrl(url.to_string()));
    }

    // Initialize HTTP client with reasonable timeouts
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| AiError::DownloadError(format!("Failed to build HTTP client: {e}")))?;

    // Initial progress notification
    progress_callback(AiModelDownloadProgress {
        model_id: manifest.id.clone(),
        bytes_downloaded: 0,
        total_bytes: manifest.expected_size_bytes,
        progress_pct: 0.0,
        state: AiDownloadState::Queued,
        error: None,
    });

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AiError::DownloadError(format!("Network request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(AiError::HttpError(format!(
            "HTTP {} ({})",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown")
        )));
    }

    let mut file = File::create(staging_path)
        .map_err(|e| AiError::StorageError(format!("Failed to create staging file: {e}")))?;

    let mut stream = response.bytes_stream();
    let mut bytes_downloaded: u64 = 0;
    let mut hasher = Sha256::new();
    let max_allowed = manifest
        .expected_size_bytes
        .saturating_add(SIZE_OVERFLOW_TOLERANCE_BYTES);

    progress_callback(AiModelDownloadProgress {
        model_id: manifest.id.clone(),
        bytes_downloaded: 0,
        total_bytes: manifest.expected_size_bytes,
        progress_pct: 0.0,
        state: AiDownloadState::Downloading,
        error: None,
    });

    while let Some(chunk_result) = stream.next().await {
        // Check for cancellation
        if *cancel_rx.borrow() {
            let _ = std::fs::remove_file(staging_path);
            progress_callback(AiModelDownloadProgress {
                model_id: manifest.id.clone(),
                bytes_downloaded,
                total_bytes: manifest.expected_size_bytes,
                progress_pct: (bytes_downloaded as f64 / manifest.expected_size_bytes as f64)
                    * 100.0,
                state: AiDownloadState::Cancelled,
                error: Some("Download cancelled by user".into()),
            });
            return Err(AiError::DownloadCancelled);
        }

        let chunk = match chunk_result {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_file(staging_path);
                return Err(AiError::DownloadError(format!(
                    "Error streaming chunk: {e}"
                )));
            }
        };

        bytes_downloaded = bytes_downloaded.saturating_add(chunk.len() as u64);
        if bytes_downloaded > max_allowed {
            let _ = std::fs::remove_file(staging_path);
            return Err(AiError::SizeMismatch {
                expected: manifest.expected_size_bytes,
                actual: bytes_downloaded,
            });
        }

        hasher.update(&chunk);
        if let Err(e) = file.write_all(&chunk) {
            let _ = std::fs::remove_file(staging_path);
            return Err(AiError::StorageError(format!(
                "Failed writing chunk to disk: {e}"
            )));
        }

        let progress_pct = if manifest.expected_size_bytes > 0 {
            ((bytes_downloaded as f64 / manifest.expected_size_bytes as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        progress_callback(AiModelDownloadProgress {
            model_id: manifest.id.clone(),
            bytes_downloaded,
            total_bytes: manifest.expected_size_bytes,
            progress_pct,
            state: AiDownloadState::Downloading,
            error: None,
        });
    }

    if let Err(e) = file.sync_all() {
        let _ = std::fs::remove_file(staging_path);
        return Err(AiError::StorageError(format!(
            "Failed to flush artifact to disk: {e}"
        )));
    }

    // Size check
    if bytes_downloaded != manifest.expected_size_bytes {
        let _ = std::fs::remove_file(staging_path);
        return Err(AiError::SizeMismatch {
            expected: manifest.expected_size_bytes,
            actual: bytes_downloaded,
        });
    }

    // Verification state
    progress_callback(AiModelDownloadProgress {
        model_id: manifest.id.clone(),
        bytes_downloaded,
        total_bytes: manifest.expected_size_bytes,
        progress_pct: 100.0,
        state: AiDownloadState::Verifying,
        error: None,
    });

    let computed_hash = format!("{:x}", hasher.finalize());
    if let Some(expected_hash) = &manifest.expected_sha256 {
        if !constant_time_eq_str(&computed_hash, expected_hash) {
            let _ = std::fs::remove_file(staging_path);
            let err = AiError::Sha256Mismatch {
                expected: expected_hash.clone(),
                actual: computed_hash,
            };
            progress_callback(AiModelDownloadProgress {
                model_id: manifest.id.clone(),
                bytes_downloaded,
                total_bytes: manifest.expected_size_bytes,
                progress_pct: 100.0,
                state: AiDownloadState::Failed,
                error: Some(err.to_string()),
            });
            return Err(err);
        }
    }

    progress_callback(AiModelDownloadProgress {
        model_id: manifest.id.clone(),
        bytes_downloaded,
        total_bytes: manifest.expected_size_bytes,
        progress_pct: 100.0,
        state: AiDownloadState::Complete,
        error: None,
    });

    Ok((computed_hash, bytes_downloaded))
}
