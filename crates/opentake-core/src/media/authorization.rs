use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::error::MediaEngineError;

/// Provenance verifying where the local media path originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceProvenance {
    /// User explicitly selected the file through a native OS file dialog.
    NativeFileDialog,
    /// User explicitly dropped the file onto the application window via OS desktop drag-and-drop.
    NativeWindowDrop,
    /// Staged recovery candidate generated safely in app-owned staging.
    StagedRecoveryCandidate,
}

/// An authorization record granting OpenTake permission to inspect and stream a specific file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationRecord {
    pub canonical_path: PathBuf,
    pub provenance: SourceProvenance,
    pub authorized_at_epoch_ms: u64,
    pub expected_file_size: u64,
    pub modified_epoch_ms: Option<u64>,
}

/// Thread-safe registry tracking strictly authorized media file paths.
///
/// # Security Invariant
/// A frontend-provided path must NEVER automatically authorize arbitrary filesystem access.
/// Only paths originating from authentic native operating system user interactions
/// (`rfd` file picker or native window drag-drop events) may be authorized.
#[derive(Debug, Clone, Default)]
pub struct MediaAuthorizationRegistry {
    records: Arc<RwLock<HashMap<PathBuf, AuthorizationRecord>>>,
}

impl MediaAuthorizationRegistry {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Authorizes a native file path with verified provenance.
    pub async fn authorize(
        &self,
        canonical_path: PathBuf,
        provenance: SourceProvenance,
        expected_file_size: u64,
        modified_epoch_ms: Option<u64>,
    ) -> AuthorizationRecord {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let record = AuthorizationRecord {
            canonical_path: canonical_path.clone(),
            provenance,
            authorized_at_epoch_ms: now_ms,
            expected_file_size,
            modified_epoch_ms,
        };

        let mut write = self.records.write().await;
        write.insert(canonical_path, record.clone());
        record
    }

    /// Verifies if a path is currently authorized and matches its initial size identity.
    pub async fn verify_authorization(
        &self,
        canonical_path: &Path,
        current_file_size: u64,
    ) -> Result<AuthorizationRecord, MediaEngineError> {
        let read = self.records.read().await;
        let record =
            read.get(canonical_path)
                .ok_or_else(|| MediaEngineError::UnauthorizedPath {
                    path: canonical_path.to_string_lossy().to_string(),
                })?;

        // Guard against file replacement / tampering attacks
        if record.expected_file_size != current_file_size {
            return Err(MediaEngineError::CorruptMedia {
                path: canonical_path.to_string_lossy().to_string(),
                details: format!(
                    "File size changed from authorized {} bytes to {} bytes",
                    record.expected_file_size, current_file_size
                ),
            });
        }

        Ok(record.clone())
    }

    /// Checks if a path is present in the authorization registry.
    pub async fn is_authorized(&self, canonical_path: &Path) -> bool {
        let read = self.records.read().await;
        read.contains_key(canonical_path)
    }

    /// Revokes authorization for a path (e.g. when removed from library).
    pub async fn revoke(&self, canonical_path: &Path) -> bool {
        let mut write = self.records.write().await;
        write.remove(canonical_path).is_some()
    }

    /// Clears all authorization records.
    pub async fn clear(&self) {
        let mut write = self.records.write().await;
        write.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authorization_lifecycle() {
        let registry = MediaAuthorizationRegistry::new();
        let path = PathBuf::from("/media/video.mp4");

        assert!(!registry.is_authorized(&path).await);

        let record = registry
            .authorize(path.clone(), SourceProvenance::NativeFileDialog, 1024, None)
            .await;
        assert_eq!(record.expected_file_size, 1024);
        assert!(registry.is_authorized(&path).await);

        // Verification succeeds with matching size
        let verified = registry.verify_authorization(&path, 1024).await;
        assert!(verified.is_ok());

        // Verification fails if file size changed (tamper attempt)
        let tampered = registry.verify_authorization(&path, 2048).await;
        assert!(matches!(
            tampered,
            Err(MediaEngineError::CorruptMedia { .. })
        ));

        // Revocation works
        assert!(registry.revoke(&path).await);
        assert!(!registry.is_authorized(&path).await);
        let rejected = registry.verify_authorization(&path, 1024).await;
        assert!(matches!(
            rejected,
            Err(MediaEngineError::UnauthorizedPath { .. })
        ));
    }
}
