//! Tauri IPC commands for the AI Voice Library.

use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use opentake_core::ai::storage::AiStorageManager;
use opentake_core::ai::tts::engine::{SystemPiperCliEngine, TtsEngine};
use opentake_core::ai::voices::assignments::{
    assign_voice_to_track as core_assign_voice_to_track,
    get_project_voice_assignments as core_get_project_voice_assignments,
    remove_track_voice_assignment as core_remove_track_voice_assignment,
    set_project_default_voice as core_set_project_default_voice,
};
use opentake_core::ai::voices::models::{
    ProjectVoiceAssignments, VoiceCatalogEntry, VoiceComparisonRequest, VoiceComparisonResult,
    VoiceCompatibility, VoicePreferences, VoicePreviewRequest, VoicePreviewResult,
};
use opentake_core::ai::voices::VoiceLibraryManager;
use opentake_core::localization::subtitles::model::SubtitleDocument;

/// Managed application state for the Voice Library.
pub struct VoiceLibraryState {
    pub manager: Arc<VoiceLibraryManager>,
}

impl VoiceLibraryState {
    pub fn new(
        storage: Arc<AiStorageManager>,
        engine: Arc<dyn TtsEngine>,
        base_dir: PathBuf,
    ) -> Self {
        let cache_dir = base_dir.join("cache").join("voice_previews");
        let preferences_path = base_dir.join("voice_preferences.json");
        let manager = Arc::new(VoiceLibraryManager::new(
            storage,
            engine,
            cache_dir,
            preferences_path,
        ));
        Self { manager }
    }

    pub fn default_local(storage: Arc<AiStorageManager>) -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let base_dir = PathBuf::from(home).join(".local/share/opentake");
        let engine = Arc::new(SystemPiperCliEngine::new(None));
        Self::new(storage, engine, base_dir)
    }
}

/// Lists available voices from the catalog, optionally filtered by language or locale.
#[tauri::command]
pub async fn list_voice_library(
    state: State<'_, VoiceLibraryState>,
    filter_lang: Option<String>,
) -> Result<Vec<VoiceCatalogEntry>, String> {
    Ok(state.manager.list_voices(filter_lang.as_deref()))
}

/// Retrieves metadata details for a specific voice ID.
#[tauri::command]
pub async fn get_voice_details(
    state: State<'_, VoiceLibraryState>,
    voice_id: String,
) -> Result<Option<VoiceCatalogEntry>, String> {
    Ok(state.manager.get_voice_details(&voice_id))
}

/// Evaluates server-side readiness and compatibility for a voice.
#[tauri::command]
pub async fn get_voice_compatibility(
    state: State<'_, VoiceLibraryState>,
    voice_id: String,
    target_language: Option<String>,
) -> Result<VoiceCompatibility, String> {
    state
        .manager
        .get_voice_compatibility(&voice_id, target_language.as_deref())
        .map_err(|e| e.to_string())
}

/// Generates or retrieves a cached voice preview.
#[tauri::command]
pub async fn start_voice_preview(
    state: State<'_, VoiceLibraryState>,
    request: VoicePreviewRequest,
) -> Result<VoicePreviewResult, String> {
    state
        .manager
        .generate_preview(&request)
        .map_err(|e| e.to_string())
}

/// Performs side-by-side comparison synthesis of two voices using the same text.
#[tauri::command]
pub async fn compare_voices(
    state: State<'_, VoiceLibraryState>,
    request: VoiceComparisonRequest,
) -> Result<VoiceComparisonResult, String> {
    state
        .manager
        .compare_voices(&request)
        .map_err(|e| e.to_string())
}

/// Retrieves user voice preferences, favorites, and recents.
#[tauri::command]
pub async fn get_voice_preferences(
    state: State<'_, VoiceLibraryState>,
) -> Result<VoicePreferences, String> {
    Ok(state.manager.get_preferences())
}

/// Toggles favorite status for a voice ID.
#[tauri::command]
pub async fn toggle_voice_favorite(
    state: State<'_, VoiceLibraryState>,
    voice_id: String,
) -> Result<bool, String> {
    state
        .manager
        .toggle_favorite(&voice_id)
        .map_err(|e| e.to_string())
}

/// Sets or clears the preferred global default voice.
#[tauri::command]
pub async fn set_preferred_voice(
    state: State<'_, VoiceLibraryState>,
    voice_id: Option<String>,
) -> Result<(), String> {
    state
        .manager
        .set_preferred_voice(voice_id)
        .map_err(|e| e.to_string())
}

/// Clears the bounded voice preview cache on disk.
#[tauri::command]
pub async fn clear_voice_preview_cache(
    state: State<'_, VoiceLibraryState>,
) -> Result<usize, String> {
    state
        .manager
        .clear_preview_cache()
        .map_err(|e| e.to_string())
}

/// Assigns a voice to a subtitle track within a SubtitleDocument.
#[tauri::command]
pub async fn assign_voice_to_track_command(
    mut document: SubtitleDocument,
    track_id: String,
    voice_id: String,
    speaker_id: Option<u32>,
) -> Result<SubtitleDocument, String> {
    core_assign_voice_to_track(&mut document, &track_id, &voice_id, speaker_id)
        .map_err(|e| e.to_string())?;
    Ok(document)
}

/// Removes a voice assignment from a subtitle track.
#[tauri::command]
pub async fn remove_voice_assignment_command(
    mut document: SubtitleDocument,
    track_id: String,
) -> Result<SubtitleDocument, String> {
    core_remove_track_voice_assignment(&mut document, &track_id).map_err(|e| e.to_string())?;
    Ok(document)
}

/// Sets the project-level default voice inside a SubtitleDocument.
#[tauri::command]
pub async fn set_project_default_voice_command(
    mut document: SubtitleDocument,
    voice_id: Option<String>,
) -> Result<SubtitleDocument, String> {
    core_set_project_default_voice(&mut document, voice_id).map_err(|e| e.to_string())?;
    Ok(document)
}

/// Retrieves all voice assignments embedded in a SubtitleDocument.
#[tauri::command]
pub async fn get_project_voice_assignments_command(
    document: SubtitleDocument,
) -> Result<ProjectVoiceAssignments, String> {
    Ok(core_get_project_voice_assignments(&document))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_voice_library_state_initialization() {
        let temp = tempdir().unwrap();
        let storage = Arc::new(AiStorageManager::new(temp.path()).unwrap());
        let engine = Arc::new(opentake_core::ai::tts::engine::MockTtsEngine::new(22050));
        let state = VoiceLibraryState::new(storage, engine, temp.path().to_path_buf());

        let voices = state.manager.list_voices(None);
        assert!(!voices.is_empty());
    }
}
