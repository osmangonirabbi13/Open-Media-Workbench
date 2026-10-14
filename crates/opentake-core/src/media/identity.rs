use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::compatibility::MediaCompatibility;
use super::metadata::MediaMetadata;

/// Provenance and identity metadata for a validated media source on the filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentity {
    /// Absolute canonical path on the host filesystem.
    pub canonical_path: String,
    /// Exact file size in bytes at time of import inspection.
    pub file_size: u64,
    /// Last modified timestamp in milliseconds since Unix epoch, if obtainable.
    pub modified_epoch_ms: Option<u64>,
    /// Lightweight non-cryptographic cache key / fingerprint based on path, size, and mtime.
    pub identity_hint: String,
}

impl SourceIdentity {
    pub fn new(canonical_path: String, file_size: u64, modified_epoch_ms: Option<u64>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(canonical_path.as_bytes());
        hasher.update(file_size.to_le_bytes());
        if let Some(mtime) = modified_epoch_ms {
            hasher.update(mtime.to_le_bytes());
        }
        let identity_hint = format!("{:x}", hasher.finalize());

        Self {
            canonical_path,
            file_size,
            modified_epoch_ms,
            identity_hint,
        }
    }
}

/// An imported media asset managed in the active OpenTake session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    /// Unique immutable session asset identifier.
    pub id: String,
    /// User-facing file display name (e.g. "interview_01.mp4").
    pub file_name: String,
    /// Validated source identity and filesystem provenance.
    pub source: SourceIdentity,
    /// Structured normalized container and stream metadata.
    pub metadata: MediaMetadata,
    /// Format compatibility and preview/edit readiness assessment.
    pub compatibility: MediaCompatibility,
    /// Timestamp when this asset was successfully imported (epoch ms).
    pub imported_at_epoch_ms: u64,
}

impl MediaAsset {
    pub fn new(
        source: SourceIdentity,
        metadata: MediaMetadata,
        compatibility: MediaCompatibility,
    ) -> Self {
        let file_name = Path::new(&source.canonical_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("media_file")
            .to_string();

        let imported_at_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            id: Uuid::new_v4().to_string(),
            file_name,
            source,
            metadata,
            compatibility,
            imported_at_epoch_ms,
        }
    }

    /// Checks if this asset represents the identical file source.
    pub fn matches_source(&self, canonical_path: &str, file_size: u64) -> bool {
        self.source.canonical_path == canonical_path && self.source.file_size == file_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::compatibility::CompatibilityStatus;
    use crate::media::metadata::ContainerMetadata;

    #[test]
    fn test_source_identity_fingerprint() {
        let s1 = SourceIdentity::new("/path/to/video.mp4".to_string(), 1024, Some(1000));
        let s2 = SourceIdentity::new("/path/to/video.mp4".to_string(), 1024, Some(1000));
        let s3 = SourceIdentity::new("/path/to/video.mp4".to_string(), 2048, Some(1000));

        assert_eq!(s1.identity_hint, s2.identity_hint);
        assert_ne!(s1.identity_hint, s3.identity_hint);
    }

    #[test]
    fn test_media_asset_creation() {
        let source = SourceIdentity::new("/media/project/camera_a.mov".to_string(), 5000, None);
        let meta = MediaMetadata {
            container: ContainerMetadata {
                format_name: "mov".to_string(),
                format_long_name: None,
                duration_micros: Some(5_000_000),
                start_time_micros: None,
                bitrate_bps: None,
                size_bytes: 5000,
                stream_count: 0,
            },
            video_streams: vec![],
            audio_streams: vec![],
            other_streams: vec![],
        };
        let compat = MediaCompatibility {
            overall: CompatibilityStatus::Supported,
            container_status: CompatibilityStatus::Supported,
            video_status: CompatibilityStatus::Supported,
            audio_status: CompatibilityStatus::Supported,
            can_inspect: true,
            can_edit_future: true,
            warnings: vec![],
        };

        let asset = MediaAsset::new(source, meta, compat);
        assert_eq!(asset.file_name, "camera_a.mov");
        assert!(!asset.id.is_empty());
        assert!(asset.matches_source("/media/project/camera_a.mov", 5000));
        assert!(!asset.matches_source("/media/project/camera_a.mov", 6000));
    }
}
