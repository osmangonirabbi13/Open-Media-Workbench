//! Tauri IPC commands for Offline Whisper Speech-to-Text Transcription.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use super::media_import::MediaLibraryState;
use opentake_core::ai::transcription::models::{
    TranscriptionRequest, TranscriptionResult, TranscriptionStatus, WhisperRuntimeCapabilities,
};
use opentake_core::ai::transcription::subtitle_conversion::{
    append_track_to_document, convert_transcription_result_to_track,
};
use opentake_core::ai::transcription::whisper::{
    discover_whisper_runtime, probe_candidate_binary, BoxFuture, RawInferenceOutput,
    SystemWhisperCliEngine, WhisperEngine,
};
use opentake_core::ai::transcription::TranscriptionManager;
use opentake_core::ai::AiStorageManager;
use opentake_core::localization::subtitles::model::SubtitleDocument;
use tokio_util::sync::CancellationToken;

/// Dynamic Whisper Engine delegating to user-selected or auto-discovered whisper-cli executables.
#[derive(Clone)]
pub struct DynamicWhisperEngine {
    custom_path: Arc<RwLock<Option<PathBuf>>>,
    discovered_path: Arc<RwLock<Option<PathBuf>>>,
    discovered_version: Arc<RwLock<Option<String>>>,
}

impl DynamicWhisperEngine {
    pub fn new(discovered_path: Option<PathBuf>, discovered_version: Option<String>) -> Self {
        Self {
            custom_path: Arc::new(RwLock::new(None)),
            discovered_path: Arc::new(RwLock::new(discovered_path)),
            discovered_version: Arc::new(RwLock::new(discovered_version)),
        }
    }

    pub async fn set_custom_path(&self, path: Option<PathBuf>) {
        let mut write = self.custom_path.write().await;
        *write = path;
    }

    pub async fn get_effective_path(&self) -> Option<PathBuf> {
        let custom = self.custom_path.read().await;
        if let Some(ref p) = *custom {
            if p.exists() {
                return Some(p.clone());
            }
        }
        let discovered = self.discovered_path.read().await;
        discovered.clone()
    }
}

impl WhisperEngine for DynamicWhisperEngine {
    fn is_available(&self) -> bool {
        if let Ok(custom) = self.custom_path.try_read() {
            if let Some(ref p) = *custom {
                if p.exists() {
                    return true;
                }
            }
        }
        if let Ok(disc) = self.discovered_path.try_read() {
            if let Some(ref p) = *disc {
                return p.exists();
            }
        }
        false
    }

    fn runtime_path(&self) -> Option<PathBuf> {
        if let Ok(custom) = self.custom_path.try_read() {
            if let Some(ref p) = *custom {
                if p.exists() {
                    return Some(p.clone());
                }
            }
        }
        if let Ok(disc) = self.discovered_path.try_read() {
            return disc.clone();
        }
        None
    }

    fn runtime_version(&self) -> Option<String> {
        if let Ok(ver) = self.discovered_version.try_read() {
            return ver.clone();
        }
        Some("whisper.cpp (dynamic)".to_string())
    }

    fn transcribe<'a>(
        &'a self,
        model_path: &'a std::path::Path,
        audio_path: &'a std::path::Path,
        language: &'a str,
        threads: Option<u32>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<RawInferenceOutput, opentake_core::ai::transcription::errors::TranscriptionError>,
    > {
        Box::pin(async move {
            let eff_path = self.get_effective_path().await.ok_or_else(|| {
                opentake_core::ai::transcription::errors::TranscriptionError::RuntimeUnavailable(
                    "No valid whisper.cpp executable found. Please install whisper-cli or select one in settings.".to_string(),
                )
            })?;

            let ver = self
                .discovered_version
                .read()
                .await
                .clone()
                .unwrap_or_else(|| "whisper.cpp".to_string());

            let inner = SystemWhisperCliEngine::new(eff_path, ver);
            inner
                .transcribe(model_path, audio_path, language, threads, cancel_token)
                .await
        })
    }
}

/// Managed application state for offline Whisper transcription.
#[derive(Clone)]
pub struct TranscriptionState {
    pub manager: Arc<TranscriptionManager>,
    pub dynamic_engine: Arc<DynamicWhisperEngine>,
}

impl TranscriptionState {
    pub fn new(storage: AiStorageManager) -> Self {
        let dynamic_engine = Arc::new(DynamicWhisperEngine::new(None, None));
        let manager = TranscriptionManager::new(
            Arc::new(storage),
            dynamic_engine.clone() as Arc<dyn WhisperEngine>,
        );
        Self {
            manager: Arc::new(manager),
            dynamic_engine,
        }
    }
}

/// Queries current Whisper runtime availability, version, and supported languages.
#[tauri::command]
pub async fn get_whisper_runtime_capabilities(
    state: State<'_, TranscriptionState>,
) -> Result<WhisperRuntimeCapabilities, String> {
    let custom = state.dynamic_engine.custom_path.read().await.clone();
    let caps = discover_whisper_runtime(custom.as_deref()).await;

    // Refresh dynamic engine state with discovered results
    if caps.is_available {
        if let Some(ref path_str) = caps.runtime_path {
            let mut disc_path = state.dynamic_engine.discovered_path.write().await;
            *disc_path = Some(PathBuf::from(path_str));
            let mut disc_ver = state.dynamic_engine.discovered_version.write().await;
            *disc_ver = caps.runtime_version.clone();
        }
    }

    Ok(caps)
}

/// Opens the native OS file picker to let the user authorize a custom whisper.cpp executable.
#[tauri::command]
pub async fn select_whisper_runtime(
    state: State<'_, TranscriptionState>,
) -> Result<WhisperRuntimeCapabilities, String> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Select whisper.cpp Executable")
        .add_filter("Executable", &["*", "exe", "bin", "cli"]);

    let handle = dialog
        .pick_file()
        .await
        .ok_or_else(|| "Selection cancelled.".to_string())?;
    let path = handle.path().to_path_buf();

    // Probe selected candidate binary
    match probe_candidate_binary(&path).await {
        Ok(ver) => {
            state
                .dynamic_engine
                .set_custom_path(Some(path.clone()))
                .await;
            let mut caps = discover_whisper_runtime(Some(&path)).await;
            caps.is_available = true;
            caps.runtime_path = Some(path.to_string_lossy().to_string());
            caps.runtime_version = Some(ver);
            caps.is_custom = true;
            Ok(caps)
        }
        Err(err) => Err(format!(
            "Selected binary '{}' is not a valid whisper.cpp executable: {err}",
            path.display()
        )),
    }
}

/// Initiates an asynchronous offline transcription job for an authorized media asset.
#[tauri::command]
pub async fn start_offline_transcription(
    request: TranscriptionRequest,
    state: State<'_, TranscriptionState>,
    media_state: State<'_, MediaLibraryState>,
) -> Result<String, String> {
    // 1. Verify asset is registered and authorized in media library
    let asset = media_state
        .session
        .get_asset_by_id(&request.asset_id)
        .await
        .ok_or_else(|| format!("Media asset '{}' not found in library.", request.asset_id))?;

    let asset_path = PathBuf::from(&asset.source.canonical_path);
    if !asset_path.exists() {
        return Err(format!(
            "Media file missing from disk: {}",
            asset.source.canonical_path
        ));
    }

    // 2. Audio duration from asset metadata
    let audio_duration_us = asset.metadata.container.duration_micros.map(|d| d as i64);

    // 3. Start background job
    state
        .manager
        .start_transcription(request, asset_path, audio_duration_us)
        .await
        .map_err(|e| e.to_string())
}

/// Checks the operational status and progress of a transcription job.
#[tauri::command]
pub async fn get_offline_transcription_status(
    job_id: String,
    state: State<'_, TranscriptionState>,
) -> Result<Option<TranscriptionStatus>, String> {
    Ok(state.manager.get_job_status(&job_id).await)
}

/// Cancels an in-flight transcription job.
#[tauri::command]
pub async fn cancel_offline_transcription(
    job_id: String,
    state: State<'_, TranscriptionState>,
) -> Result<(), String> {
    state
        .manager
        .cancel_job(&job_id)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieves the finalized result of a completed transcription job.
#[tauri::command]
pub async fn get_offline_transcription_result(
    job_id: String,
    state: State<'_, TranscriptionState>,
) -> Result<TranscriptionResult, String> {
    state
        .manager
        .get_job_result(&job_id)
        .await
        .map_err(|e| e.to_string())
}

/// Converts a completed transcription result into a canonical SubtitleTrack and non-destructively appends it.
#[tauri::command]
pub async fn apply_transcription_result_to_subtitles(
    job_id: String,
    custom_track_name: Option<String>,
    document: Option<SubtitleDocument>,
    state: State<'_, TranscriptionState>,
) -> Result<SubtitleDocument, String> {
    let result = state
        .manager
        .get_job_result(&job_id)
        .await
        .map_err(|e| e.to_string())?;

    let track = convert_transcription_result_to_track(&result, custom_track_name.as_deref())
        .map_err(|e| e.to_string())?;

    Ok(append_track_to_document(document, track))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transcription_state_initialization() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let storage = AiStorageManager::new(temp_dir.path()).expect("create storage");
        let state = TranscriptionState::new(storage);

        assert!(!state.dynamic_engine.is_available());
        assert_eq!(state.dynamic_engine.runtime_path(), None);
    }

    #[tokio::test]
    async fn test_dynamic_whisper_engine_custom_path() {
        let engine = DynamicWhisperEngine::new(None, None);
        assert_eq!(engine.get_effective_path().await, None);

        let temp_file = tempfile::NamedTempFile::new().expect("temp file");
        engine
            .set_custom_path(Some(temp_file.path().to_path_buf()))
            .await;

        assert_eq!(
            engine.get_effective_path().await,
            Some(temp_file.path().to_path_buf())
        );
        assert!(engine.is_available());
    }
}
