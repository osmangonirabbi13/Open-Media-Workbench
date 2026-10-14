use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use opentake_core::media::error::MediaEngineError;
use opentake_core::media::identity::MediaAsset;
use opentake_core::media::import::MediaLibrarySession;

/// Application state holding the session-level Media Library manager.
#[derive(Clone)]
pub struct MediaLibraryState {
    pub session: Arc<MediaLibrarySession>,
}

impl Default for MediaLibraryState {
    fn default() -> Self {
        Self {
            session: Arc::new(MediaLibrarySession::default()),
        }
    }
}

/// Description of a file that failed import inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    pub file_name: String,
    pub error_code: String,
    pub error_message: String,
}

/// Result of a multi-file import batch operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatchResult {
    pub succeeded: Vec<MediaAsset>,
    pub duplicates: Vec<String>,
    pub failed: Vec<ImportFailure>,
}

/// Opens the native OS file picker and imports all selected media files.
#[tauri::command]
pub async fn pick_and_import_files(
    state: State<'_, MediaLibraryState>,
) -> Result<ImportBatchResult, String> {
    // 1. Open native OS file picker
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Import Media Files")
        .add_filter(
            "Supported Media (*.mp4, *.mov, *.mkv, *.webm, *.m4v)",
            &["mp4", "mov", "mkv", "webm", "m4v"],
        )
        .add_filter("All Files (*.*)", &["*"]);

    let handles = dialog.pick_files().await;

    let Some(files) = handles else {
        // User cancelled dialog
        return Ok(ImportBatchResult {
            succeeded: Vec::new(),
            duplicates: Vec::new(),
            failed: Vec::new(),
        });
    };

    let paths: Vec<PathBuf> = files.into_iter().map(|h| h.path().to_path_buf()).collect();
    process_import_paths(&paths, &state.session).await
}

/// Ingests candidate file paths passed from native window drag-and-drop.
#[tauri::command]
pub async fn import_files_by_path(
    paths: Vec<String>,
    state: State<'_, MediaLibraryState>,
) -> Result<ImportBatchResult, String> {
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    process_import_paths(&path_bufs, &state.session).await
}

/// Returns all media assets currently in the active session.
#[tauri::command]
pub async fn get_media_library_assets(
    state: State<'_, MediaLibraryState>,
) -> Result<Vec<MediaAsset>, String> {
    Ok(state.session.get_assets().await)
}

/// Removes a media asset from the current library session.
/// GUARANTEE: Does NOT delete, rename, or touch the source file on disk.
#[tauri::command]
pub async fn remove_media_library_asset(
    id: String,
    state: State<'_, MediaLibraryState>,
) -> Result<bool, String> {
    Ok(state.session.remove_asset(&id).await)
}

/// Clears all media assets from the active library session.
#[tauri::command]
pub async fn clear_media_library(state: State<'_, MediaLibraryState>) -> Result<(), String> {
    state.session.clear_session().await;
    Ok(())
}

/// Shared helper function to validate and ingest multiple paths sequentially.
async fn process_import_paths(
    paths: &[PathBuf],
    session: &MediaLibrarySession,
) -> Result<ImportBatchResult, String> {
    let mut succeeded = Vec::new();
    let mut duplicates = Vec::new();
    let mut failed = Vec::new();

    for path in paths {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        match session.import_file(path, None).await {
            Ok(asset) => succeeded.push(asset),
            Err(MediaEngineError::DuplicateFile { .. }) => duplicates.push(file_name),
            Err(e) => failed.push(ImportFailure {
                file_name,
                error_code: e.code().to_string(),
                error_message: e.user_message(),
            }),
        }
    }

    Ok(ImportBatchResult {
        succeeded,
        duplicates,
        failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_media_library_state_lifecycle() {
        let state = MediaLibraryState::default();
        let assets = state.session.get_assets().await;
        assert!(assets.is_empty());
    }

    #[tokio::test]
    async fn test_process_import_paths_missing_file() {
        let session = MediaLibrarySession::default();
        let fake_path = PathBuf::from("/nonexistent/video_test_file.mp4");
        let result = process_import_paths(&[fake_path], &session).await.unwrap();

        assert_eq!(result.succeeded.len(), 0);
        assert_eq!(result.duplicates.len(), 0);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].error_code, "FILE_NOT_FOUND");
    }

    #[tokio::test]
    async fn test_session_clear() {
        let session = MediaLibrarySession::default();
        session.clear_session().await;
        assert_eq!(session.get_assets().await.len(), 0);
    }
}
