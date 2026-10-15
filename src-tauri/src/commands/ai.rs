//! Tauri IPC commands for AI Model Management.

use opentake_core::ai::{
    AiHardwareInfo, AiModelDownloadProgress, AiModelManager, AiModelSummary, AiStorageUsage,
};
use std::path::PathBuf;
use std::sync::Arc;

/// Managed application state for the shared AI Model Manager.
pub struct AiState {
    pub manager: Arc<AiModelManager>,
}

impl Default for AiState {
    fn default() -> Self {
        let base_dir = std::env::var("OPENTAKE_AI_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(home).join(".local/share/open-media-workbench/ai-models")
                } else {
                    std::env::temp_dir().join("open-media-workbench/ai-models")
                }
            });
        let manager = AiModelManager::new(base_dir).expect("initialize AI model manager");
        Self {
            manager: Arc::new(manager),
        }
    }
}

impl AiState {
    pub fn new_with_path(path: PathBuf) -> Self {
        let manager =
            AiModelManager::new(path).expect("initialize AI model manager with custom path");
        Self {
            manager: Arc::new(manager),
        }
    }
}

#[tauri::command]
pub async fn list_ai_models(
    state: tauri::State<'_, AiState>,
) -> Result<Vec<AiModelSummary>, String> {
    state.manager.list_models().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_model_status(
    model_id: String,
    state: tauri::State<'_, AiState>,
) -> Result<AiModelSummary, String> {
    state
        .manager
        .get_model_status(&model_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_hardware_info(
    state: tauri::State<'_, AiState>,
) -> Result<AiHardwareInfo, String> {
    Ok(state.manager.get_hardware_info())
}

#[tauri::command]
pub async fn start_ai_model_download(
    model_id: String,
    state: tauri::State<'_, AiState>,
) -> Result<AiModelDownloadProgress, String> {
    state
        .manager
        .start_download(&model_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_model_download_status(
    model_id: String,
    state: tauri::State<'_, AiState>,
) -> Result<Option<AiModelDownloadProgress>, String> {
    Ok(state.manager.get_download_progress(&model_id))
}

#[tauri::command]
pub async fn cancel_ai_model_download(
    model_id: String,
    state: tauri::State<'_, AiState>,
) -> Result<(), String> {
    state
        .manager
        .cancel_download(&model_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn verify_ai_model(
    model_id: String,
    state: tauri::State<'_, AiState>,
) -> Result<bool, String> {
    state
        .manager
        .verify_model(&model_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_ai_model(
    model_id: String,
    state: tauri::State<'_, AiState>,
) -> Result<(), String> {
    state
        .manager
        .remove_model(&model_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_storage_usage(
    state: tauri::State<'_, AiState>,
) -> Result<AiStorageUsage, String> {
    state.manager.get_storage_usage().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ai_state_initialization_and_listing() {
        let temp = tempdir().expect("tempdir");
        let state = AiState::new_with_path(temp.path().to_path_buf());

        let models = state.manager.list_models().expect("list models");
        assert!(!models.is_empty());

        let hw = state.manager.get_hardware_info();
        assert!(!hw.os.is_empty());

        let usage = state.manager.get_storage_usage().expect("usage");
        assert_eq!(usage.total_models_size_bytes, 0);
    }
}
