use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};
use tokio_util::sync::CancellationToken;

use super::compatibility::MediaCompatibility;
use super::error::MediaEngineError;
use super::identity::{MediaAsset, SourceIdentity};
use super::probe::probe_media_file;

/// Maximum allowable single file size for safe import inspection (500 GiB).
pub const MAX_IMPORT_FILE_SIZE_BYTES: u64 = 500 * 1024 * 1024 * 1024;

/// Default concurrency limit for probe subprocesses to ensure desktop responsiveness.
pub const MAX_CONCURRENT_PROBE_JOBS: usize = 2;

/// Detailed status of an in-flight or completed media import job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state", content = "payload")]
pub enum ImportJobStatus {
    Queued,
    Validating,
    Probing,
    Completed(MediaAsset),
    Duplicate(MediaAsset),
    Failed(String),
    Cancelled,
}

/// Validated filesystem properties of a candidate media file.
#[derive(Debug, Clone)]
pub struct ValidatedFile {
    pub canonical_path: PathBuf,
    pub canonical_path_str: String,
    pub file_size: u64,
    pub modified_epoch_ms: Option<u64>,
}

/// Validates a candidate path for media ingestion.
///
/// # Security & Safety Checks
/// 1. Verifies the path exists and is a regular file.
/// 2. Canonicalizes the path to resolve symlinks and prevent traversal tricks.
/// 3. Confirms read permissions.
/// 4. Rejects 0-byte (empty) files and files exceeding safety size limits.
pub fn validate_media_file_path(path: &Path) -> Result<ValidatedFile, MediaEngineError> {
    let path_str = path.to_string_lossy().to_string();

    // 1. Check path exists
    if !path.exists() {
        return Err(MediaEngineError::FileNotFound { path: path_str });
    }

    // 2. Canonicalize path
    let canonical = match fs::canonicalize(path) {
        Ok(c) => c,
        Err(e) => {
            return Err(MediaEngineError::InvalidPath {
                path: path_str,
                reason: format!("Failed to canonicalize path: {e}"),
            });
        }
    };

    let canonical_str = canonical.to_string_lossy().to_string();

    // 3. Inspect metadata
    let metadata = match fs::metadata(&canonical) {
        Ok(m) => m,
        Err(_e) => {
            return Err(MediaEngineError::PermissionDenied {
                path: canonical_str,
            });
        }
    };

    // 4. Ensure it is a regular file
    if !metadata.is_file() {
        return Err(MediaEngineError::NotARegularFile {
            path: canonical_str,
        });
    }

    // 5. Check size
    let file_size = metadata.len();
    if file_size == 0 {
        return Err(MediaEngineError::EmptyFile {
            path: canonical_str,
        });
    }

    if file_size > MAX_IMPORT_FILE_SIZE_BYTES {
        return Err(MediaEngineError::FileTooLarge {
            path: canonical_str,
            size_bytes: file_size,
            limit_bytes: MAX_IMPORT_FILE_SIZE_BYTES,
        });
    }

    // 6. Confirm readable access
    match fs::File::open(&canonical) {
        Ok(_) => {}
        Err(e) => {
            return Err(MediaEngineError::PermissionDenied {
                path: format!("{canonical_str}: {e}"),
            });
        }
    }

    // 7. Extract mtime
    let modified_epoch_ms = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);

    Ok(ValidatedFile {
        canonical_path: canonical,
        canonical_path_str: canonical_str,
        file_size,
        modified_epoch_ms,
    })
}

/// Headless session manager holding imported media assets in memory.
/// No media files are ever modified or deleted on disk by this service.
#[derive(Debug, Clone)]
pub struct MediaLibrarySession {
    assets: Arc<RwLock<Vec<MediaAsset>>>,
    probe_semaphore: Arc<Semaphore>,
    ffprobe_path: Option<PathBuf>,
}

impl Default for MediaLibrarySession {
    fn default() -> Self {
        Self::new(None)
    }
}

impl MediaLibrarySession {
    pub fn new(ffprobe_path: Option<PathBuf>) -> Self {
        Self {
            assets: Arc::new(RwLock::new(Vec::new())),
            probe_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_PROBE_JOBS)),
            ffprobe_path,
        }
    }

    /// Returns a snapshot of all active media assets in this session.
    pub async fn get_assets(&self) -> Vec<MediaAsset> {
        let read = self.assets.read().await;
        read.clone()
    }

    /// Finds an asset by its unique session ID.
    pub async fn get_asset_by_id(&self, id: &str) -> Option<MediaAsset> {
        let read = self.assets.read().await;
        read.iter().find(|a| a.id == id).cloned()
    }

    /// Checks if a file source is already imported in this session.
    pub async fn find_duplicate(&self, canonical_path: &str, file_size: u64) -> Option<MediaAsset> {
        let read = self.assets.read().await;
        read.iter()
            .find(|a| a.matches_source(canonical_path, file_size))
            .cloned()
    }

    /// Removes an asset from the media library session.
    /// GUARANTEE: Does NOT delete, rename, or touch the source file on disk.
    pub async fn remove_asset(&self, id: &str) -> bool {
        let mut write = self.assets.write().await;
        let original_len = write.len();
        write.retain(|a| a.id != id);
        write.len() < original_len
    }

    /// Clears all assets from the session without touching files on disk.
    pub async fn clear_session(&self) {
        let mut write = self.assets.write().await;
        write.clear();
    }

    /// Ingests and inspects a single validated file path.
    pub async fn import_file(
        &self,
        candidate_path: &Path,
        cancellation: Option<&CancellationToken>,
    ) -> Result<MediaAsset, MediaEngineError> {
        // 1. Check cancellation
        if let Some(token) = cancellation {
            if token.is_cancelled() {
                return Err(MediaEngineError::Cancelled {
                    command: "import_file".to_string(),
                });
            }
        }

        // 2. Validate file on filesystem
        let validated = validate_media_file_path(candidate_path)?;

        // 3. Duplicate check
        if let Some(_existing) = self
            .find_duplicate(&validated.canonical_path_str, validated.file_size)
            .await
        {
            return Err(MediaEngineError::DuplicateFile {
                path: validated.canonical_path_str,
            });
        }

        // 4. Acquire probe concurrency permit (bounds simultaneous FFprobe processes)
        let _permit = self
            .probe_semaphore
            .acquire()
            .await
            .map_err(|_| MediaEngineError::Internal("Probe semaphore closed".to_string()))?;

        // Check cancellation again after waiting on permit
        if let Some(token) = cancellation {
            if token.is_cancelled() {
                return Err(MediaEngineError::Cancelled {
                    command: "import_file".to_string(),
                });
            }
        }

        // 5. Probe media with FFprobe
        let metadata = probe_media_file(
            self.ffprobe_path.as_ref(),
            &validated.canonical_path,
            validated.file_size,
            cancellation,
        )
        .await?;

        // 6. Evaluate compatibility
        let compatibility = MediaCompatibility::evaluate(&metadata);

        // 7. Construct source identity and asset
        let source = SourceIdentity::new(
            validated.canonical_path_str,
            validated.file_size,
            validated.modified_epoch_ms,
        );

        let asset = MediaAsset::new(source, metadata, compatibility);

        // 8. Register in session
        {
            let mut write = self.assets.write().await;
            // Final duplicate check before inserting
            if !write
                .iter()
                .any(|a| a.matches_source(&asset.source.canonical_path, asset.source.file_size))
            {
                write.push(asset.clone());
            }
        }

        Ok(asset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::metadata::MediaMetadata;
    use std::io::Write;

    #[test]
    fn test_validate_nonexistent_file() {
        let res = validate_media_file_path(Path::new("/path/definitely/does/not/exist.mp4"));
        assert!(matches!(res, Err(MediaEngineError::FileNotFound { .. })));
    }

    #[test]
    fn test_validate_empty_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let empty_path = temp_dir.path().join("empty.mp4");
        fs::File::create(&empty_path).unwrap();

        let res = validate_media_file_path(&empty_path);
        assert!(matches!(res, Err(MediaEngineError::EmptyFile { .. })));
    }

    #[test]
    fn test_validate_directory_rejected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let res = validate_media_file_path(temp_dir.path());
        assert!(matches!(res, Err(MediaEngineError::NotARegularFile { .. })));
    }

    #[test]
    fn test_validate_valid_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let valid_path = temp_dir.path().join("test.mp4");
        {
            let mut f = fs::File::create(&valid_path).unwrap();
            f.write_all(b"sample data content").unwrap();
        }

        let res = validate_media_file_path(&valid_path).unwrap();
        assert_eq!(res.file_size, 19);
        assert!(res.canonical_path_str.ends_with("test.mp4"));
    }

    #[tokio::test]
    async fn test_session_remove_does_not_delete_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("persisted.mp4");
        {
            let mut f = fs::File::create(&file_path).unwrap();
            f.write_all(b"do not delete me!").unwrap();
        }

        let session = MediaLibrarySession::default();
        let source = SourceIdentity::new(file_path.to_string_lossy().to_string(), 17, None);
        let meta = MediaMetadata {
            container: super::super::metadata::ContainerMetadata {
                format_name: "mp4".to_string(),
                format_long_name: None,
                duration_micros: Some(1000),
                start_time_micros: None,
                bitrate_bps: None,
                size_bytes: 17,
                stream_count: 0,
            },
            video_streams: vec![],
            audio_streams: vec![],
            other_streams: vec![],
        };
        let compat = MediaCompatibility::evaluate(&meta);
        let asset = MediaAsset::new(source, meta, compat);
        let asset_id = asset.id.clone();

        {
            let mut write = session.assets.write().await;
            write.push(asset);
        }

        assert_eq!(session.get_assets().await.len(), 1);

        // Remove from session
        let removed = session.remove_asset(&asset_id).await;
        assert!(removed);
        assert_eq!(session.get_assets().await.len(), 0);

        // Verify file is still untouched on disk!
        assert!(file_path.exists());
        let content = fs::read(&file_path).unwrap();
        assert_eq!(content, b"do not delete me!");
    }
}
