//! Tauri IPC commands for Offline Subtitle Translation.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use opentake_core::ai::translation::{
    append_translated_track_to_document, discover_llama_runtime, get_supported_language_pairs,
    probe_llama_binary, SystemLlamaCliTranslationEngine, TranslationEngine,
    TranslationLanguagePair, TranslationManager, TranslationRequest, TranslationResult,
    TranslationStatus,
};
use opentake_core::ai::AiStorageManager;
use opentake_core::localization::subtitles::model::{SubtitleCue, SubtitleDocument};
use tokio_util::sync::CancellationToken;

/// Dynamic Translation Engine delegating to user-selected or auto-discovered llama-cli executables.
#[derive(Clone)]
pub struct DynamicTranslationEngine {
    custom_path: Arc<RwLock<Option<PathBuf>>>,
    discovered_path: Arc<RwLock<Option<PathBuf>>>,
    discovered_version: Arc<RwLock<Option<String>>>,
}

impl DynamicTranslationEngine {
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

impl TranslationEngine for DynamicTranslationEngine {
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
        Some("llama.cpp (dynamic)".to_string())
    }

    fn translate<'a>(
        &'a self,
        model_path: &'a Path,
        texts: &'a [String],
        source_lang: &'a str,
        target_lang: &'a str,
        cancel_token: CancellationToken,
    ) -> opentake_core::ai::translation::engine::BoxFuture<
        'a,
        Result<Vec<String>, opentake_core::ai::translation::TranslationError>,
    > {
        Box::pin(async move {
            let effective = self.get_effective_path().await.ok_or_else(|| {
                opentake_core::ai::translation::TranslationError::RuntimeUnavailable(
                    "No valid llama-cli executable discovered or configured.".to_string(),
                )
            })?;

            let version = self
                .runtime_version()
                .unwrap_or_else(|| "llama.cpp (local)".to_string());
            let engine = SystemLlamaCliTranslationEngine::new(effective, version);
            engine
                .translate(model_path, texts, source_lang, target_lang, cancel_token)
                .await
        })
    }
}

/// Runtime capability info reported to frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRuntimeCapabilities {
    pub is_available: bool,
    pub runtime_path: Option<String>,
    pub runtime_version: Option<String>,
    pub supported_language_pairs: Vec<TranslationLanguagePair>,
}

/// Global translation state injected into Tauri runtime.
pub struct TranslationState {
    pub manager: Arc<TranslationManager>,
    pub dynamic_engine: Arc<DynamicTranslationEngine>,
}

impl TranslationState {
    pub fn new(storage: AiStorageManager) -> Self {
        let storage_arc = Arc::new(storage);
        let dynamic_engine = Arc::new(DynamicTranslationEngine::new(None, None));
        let manager = Arc::new(TranslationManager::new(storage_arc, dynamic_engine.clone()));

        Self {
            manager,
            dynamic_engine,
        }
    }

    pub async fn initialize_discovery(&self) {
        if let Ok(discovered) = discover_llama_runtime(None).await {
            if let Some(path) = discovered.runtime_path() {
                let mut path_guard = self.dynamic_engine.discovered_path.write().await;
                *path_guard = Some(path);
            }
            if let Some(ver) = discovered.runtime_version() {
                let mut ver_guard = self.dynamic_engine.discovered_version.write().await;
                *ver_guard = Some(ver);
            }
        }
    }
}

/// Queries discovered translation runtime capabilities.
#[tauri::command]
pub async fn get_translation_capabilities(
    state: State<'_, TranslationState>,
) -> Result<TranslationRuntimeCapabilities, String> {
    let is_available = state.dynamic_engine.is_available();
    let runtime_path = state
        .dynamic_engine
        .get_effective_path()
        .await
        .map(|p| p.to_string_lossy().to_string());
    let runtime_version = state.dynamic_engine.runtime_version();
    let supported_language_pairs = get_supported_language_pairs();

    Ok(TranslationRuntimeCapabilities {
        is_available,
        runtime_path,
        runtime_version,
        supported_language_pairs,
    })
}

/// Lists all supported language pairs.
#[tauri::command]
pub async fn list_translation_language_pairs() -> Result<Vec<TranslationLanguagePair>, String> {
    Ok(get_supported_language_pairs())
}

/// Allows user to specify custom llama-cli binary.
#[tauri::command]
pub async fn select_translation_runtime(
    path: String,
    state: State<'_, TranslationState>,
) -> Result<String, String> {
    let p = PathBuf::from(&path);
    match probe_llama_binary(&p).await {
        Ok(version) => {
            state.dynamic_engine.set_custom_path(Some(p.clone())).await;
            Ok(format!("Validated runtime at {}: {version}", p.display()))
        }
        Err(err) => Err(format!(
            "Selected binary '{}' is not a valid llama-cli executable: {err}",
            p.display()
        )),
    }
}

/// Starts an offline translation job for a set of subtitle cues.
#[tauri::command]
pub async fn start_offline_translation(
    request: TranslationRequest,
    cues: Vec<SubtitleCue>,
    state: State<'_, TranslationState>,
) -> Result<String, String> {
    state
        .manager
        .start_translation(request, cues)
        .await
        .map_err(|e| e.to_string())
}

/// Checks the status and progress of an active or completed translation job.
#[tauri::command]
pub async fn get_offline_translation_status(
    job_id: String,
    state: State<'_, TranslationState>,
) -> Result<Option<TranslationStatus>, String> {
    Ok(state.manager.get_status(&job_id).await)
}

/// Cancels an in-flight translation job.
#[tauri::command]
pub async fn cancel_offline_translation(
    job_id: String,
    state: State<'_, TranslationState>,
) -> Result<(), String> {
    state
        .manager
        .cancel_job(&job_id)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieves the finalized result of a completed translation job.
#[tauri::command]
pub async fn get_offline_translation_result(
    job_id: String,
    state: State<'_, TranslationState>,
) -> Result<Option<TranslationResult>, String> {
    Ok(state.manager.get_result(&job_id).await)
}

/// Converts translation result into a new SubtitleTrack and appends it to SubtitleDocument.
#[tauri::command]
pub async fn apply_translation_result_to_subtitles(
    result: TranslationResult,
    custom_track_name: Option<String>,
    mut document: SubtitleDocument,
) -> Result<SubtitleDocument, String> {
    append_translated_track_to_document(&mut document, &result, custom_track_name.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(document)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translation_state_initialization() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let storage = AiStorageManager::new(temp_dir.path()).expect("create storage");
        let state = TranslationState::new(storage);

        assert!(!state.dynamic_engine.is_available());
        assert_eq!(state.dynamic_engine.runtime_path(), None);
    }
}
