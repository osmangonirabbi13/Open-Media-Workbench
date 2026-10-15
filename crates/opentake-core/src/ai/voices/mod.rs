//! # Offline AI Voice Library for Open Media Workbench
//!
//! Provides a unified, offline-first voice discovery, preview, comparison,
//! compatibility, favorites, and track assignment domain.

pub mod assignments;
pub mod catalog;
pub mod compatibility;
pub mod errors;
pub mod models;
pub mod preferences;
pub mod previews;

use std::path::PathBuf;
use std::sync::Arc;

pub use assignments::{
    assign_voice_to_track, get_project_voice_assignments, remove_track_voice_assignment,
    set_project_default_voice, VOICE_ASSIGNMENTS_METADATA_KEY,
};
pub use catalog::{
    filter_voices_by_language, find_voice_by_id, find_voice_by_model_id, get_supported_languages,
    get_supported_locales, get_voice_catalog,
};
pub use compatibility::{check_voice_compatibility, validate_speaker_id};
pub use errors::VoiceLibraryError;
pub use models::*;
pub use preferences::VoicePreferencesManager;
pub use previews::VoicePreviewManager;

use crate::ai::storage::AiStorageManager;
use crate::ai::tts::engine::TtsEngine;

/// Unified coordinator for the Voice Library domain.
pub struct VoiceLibraryManager {
    storage: Arc<AiStorageManager>,
    tts_engine: Arc<dyn TtsEngine>,
    preview_manager: Arc<VoicePreviewManager>,
    preferences_manager: Arc<VoicePreferencesManager>,
}

impl VoiceLibraryManager {
    pub fn new(
        storage: Arc<AiStorageManager>,
        tts_engine: Arc<dyn TtsEngine>,
        cache_dir: PathBuf,
        preferences_path: PathBuf,
    ) -> Self {
        let preview_manager = Arc::new(VoicePreviewManager::new(
            cache_dir,
            storage.clone(),
            tts_engine.clone(),
        ));
        let preferences_manager = Arc::new(VoicePreferencesManager::new(preferences_path));

        Self {
            storage,
            tts_engine,
            preview_manager,
            preferences_manager,
        }
    }

    /// Lists catalog voices, optionally filtered by language or locale tag.
    pub fn list_voices(&self, filter_lang: Option<&str>) -> Vec<VoiceCatalogEntry> {
        match filter_lang {
            Some(lang) if !lang.trim().is_empty() => filter_voices_by_language(lang),
            _ => get_voice_catalog(),
        }
    }

    /// Retrieves detailed metadata for a single voice.
    pub fn get_voice_details(&self, voice_id: &str) -> Option<VoiceCatalogEntry> {
        find_voice_by_id(voice_id)
    }

    /// Evaluates server-side voice compatibility against host runtime and storage.
    pub fn get_voice_compatibility(
        &self,
        voice_id: &str,
        target_language: Option<&str>,
    ) -> Result<VoiceCompatibility, VoiceLibraryError> {
        let voice = find_voice_by_id(voice_id)
            .ok_or_else(|| VoiceLibraryError::VoiceNotFound(voice_id.to_string()))?;
        Ok(check_voice_compatibility(
            &voice,
            &self.storage,
            self.tts_engine.as_ref(),
            target_language,
        ))
    }

    /// Generates or retrieves a cached voice preview.
    pub fn generate_preview(
        &self,
        req: &VoicePreviewRequest,
    ) -> Result<VoicePreviewResult, VoiceLibraryError> {
        let res = self.preview_manager.generate_preview(req)?;
        let _ = self
            .preferences_manager
            .record_recent(&req.voice_id, req.speaker_id);
        Ok(res)
    }

    /// Performs side-by-side comparison between two distinct voices.
    pub fn compare_voices(
        &self,
        req: &VoiceComparisonRequest,
    ) -> Result<VoiceComparisonResult, VoiceLibraryError> {
        self.preview_manager.compare_voices(req)
    }

    /// Retrieves user voice preferences.
    pub fn get_preferences(&self) -> VoicePreferences {
        self.preferences_manager.get_preferences()
    }

    /// Toggles favorite status for a voice.
    pub fn toggle_favorite(&self, voice_id: &str) -> Result<bool, VoiceLibraryError> {
        self.preferences_manager.toggle_favorite(voice_id)
    }

    /// Sets or clears preferred default voice.
    pub fn set_preferred_voice(&self, voice_id: Option<String>) -> Result<(), VoiceLibraryError> {
        self.preferences_manager.set_preferred_voice(voice_id)
    }

    /// Clears the bounded preview cache.
    pub fn clear_preview_cache(&self) -> Result<usize, VoiceLibraryError> {
        self.preview_manager.clear_cache()
    }
}
