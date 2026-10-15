//! Tauri IPC commands for Offline Text-to-Speech (TTS).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use opentake_core::ai::storage::AiStorageManager;
use opentake_core::ai::tts::engine::{SystemPiperCliEngine, TtsEngine};
use opentake_core::ai::tts::manager::TtsManager;
use opentake_core::ai::tts::models::{
    AiVoiceManifest, TtsRequest, TtsResult, TtsRuntimeCapabilities, TtsStatus,
};
use opentake_core::localization::subtitles::model::SubtitleTrack;

/// Managed application state for TTS synthesis.
pub struct TtsState {
    pub manager: Arc<TtsManager>,
    pub custom_binary_path: Arc<RwLock<Option<PathBuf>>>,
}

impl TtsState {
    pub fn new(storage: AiStorageManager) -> Self {
        let storage_arc = Arc::new(storage);
        let engine = Arc::new(SystemPiperCliEngine::new(None));
        let manager = Arc::new(TtsManager::new(storage_arc, engine));
        Self {
            manager,
            custom_binary_path: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_engine(storage: Arc<AiStorageManager>, engine: Arc<dyn TtsEngine>) -> Self {
        let manager = Arc::new(TtsManager::new(storage, engine));
        Self {
            manager,
            custom_binary_path: Arc::new(RwLock::new(None)),
        }
    }
}

/// Discovers local TTS runtime capabilities and binary availability.
#[tauri::command]
pub async fn get_tts_runtime_capabilities(
    state: State<'_, TtsState>,
) -> Result<TtsRuntimeCapabilities, String> {
    state
        .manager
        .engine()
        .discover_capabilities()
        .map_err(|e| e.to_string())
}

/// Allows user to select a custom Piper executable using a native file dialog.
#[tauri::command]
pub async fn select_tts_runtime(
    state: State<'_, TtsState>,
) -> Result<TtsRuntimeCapabilities, String> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Select Piper Executable")
        .add_filter("Executable", &["*", "exe", "bin", "piper"]);

    let handle = dialog
        .pick_file()
        .await
        .ok_or_else(|| "Selection cancelled.".to_string())?;

    let path = handle.path().to_path_buf();
    if !path.is_file() {
        return Err(format!("Selected path is not a file: {}", path.display()));
    }

    let engine = Arc::new(SystemPiperCliEngine::new(Some(path.clone())));
    let mut custom = state.custom_binary_path.write().await;
    *custom = Some(path);

    engine.discover_capabilities().map_err(|e| e.to_string())
}

/// Lists all supported voices and their local readiness.
#[tauri::command]
pub async fn list_available_tts_voices(
    state: State<'_, TtsState>,
) -> Result<Vec<AiVoiceManifest>, String> {
    Ok(state.manager.list_voices())
}

/// Retrieves the status and capabilities for a specific voice.
#[tauri::command]
pub async fn get_tts_voice_status(
    voice_id: String,
    state: State<'_, TtsState>,
) -> Result<AiVoiceManifest, String> {
    let voices = state.manager.list_voices();
    voices
        .into_iter()
        .find(|v| v.id == voice_id)
        .ok_or_else(|| format!("Voice '{voice_id}' not found in catalog"))
}

/// Starts an asynchronous offline speech synthesis job.
#[tauri::command]
pub async fn start_offline_tts(
    request: TtsRequest,
    track: Option<SubtitleTrack>,
    state: State<'_, TtsState>,
) -> Result<String, String> {
    state
        .manager
        .start_synthesis(request, track)
        .map_err(|e| e.to_string())
}

/// Queries the status and stage of an active or completed synthesis job.
#[tauri::command]
pub async fn get_offline_tts_status(
    job_id: String,
    state: State<'_, TtsState>,
) -> Result<TtsStatus, String> {
    state
        .manager
        .get_status(&job_id)
        .ok_or_else(|| format!("Synthesis job '{job_id}' not found"))
}

/// Cancels an in-progress offline speech synthesis job.
#[tauri::command]
pub async fn cancel_offline_tts(
    job_id: String,
    state: State<'_, TtsState>,
) -> Result<bool, String> {
    state.manager.cancel_job(&job_id).map_err(|e| e.to_string())
}

/// Retrieves the final result and segments for a completed synthesis job.
#[tauri::command]
pub async fn get_offline_tts_result(
    job_id: String,
    state: State<'_, TtsState>,
) -> Result<TtsResult, String> {
    state.manager.get_result(&job_id).map_err(|e| e.to_string())
}

/// Exports a single segment or entire synthesized speech to a WAV destination.
#[tauri::command]
pub async fn export_tts_audio(
    job_id: String,
    segment_id: Option<String>,
    destination_path: String,
    state: State<'_, TtsState>,
) -> Result<String, String> {
    let dest = Path::new(&destination_path);
    let exported = state
        .manager
        .export_audio(&job_id, segment_id.as_deref(), dest)
        .map_err(|e| e.to_string())?;

    Ok(exported.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentake_core::ai::tts::engine::MockTtsEngine;
    use tempfile::tempdir;

    #[test]
    fn test_tts_state_initialization() {
        let temp = tempdir().unwrap();
        let storage = Arc::new(AiStorageManager::new(temp.path()).unwrap());
        let mock_engine = Arc::new(MockTtsEngine::default());
        let state = TtsState::with_engine(storage, mock_engine);

        let voices = state.manager.list_voices();
        assert!(!voices.is_empty());
        assert_eq!(voices[0].id, "piper-en-lessac-medium");
    }
}
