//! Persistent user voice preferences, favorites, and recents management.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use super::errors::VoiceLibraryError;
use super::models::VoicePreferences;

/// Thread-safe manager for user voice preferences.
pub struct VoicePreferencesManager {
    file_path: PathBuf,
    preferences: RwLock<VoicePreferences>,
}

impl VoicePreferencesManager {
    /// Initializes the preferences manager at the specified path.
    pub fn new(file_path: PathBuf) -> Self {
        let prefs = Self::load_from_path(&file_path).unwrap_or_default();
        Self {
            file_path,
            preferences: RwLock::new(prefs),
        }
    }

    /// Loads preferences from disk or returns default if not found.
    fn load_from_path(path: &Path) -> Result<VoicePreferences, VoiceLibraryError> {
        if !path.exists() {
            return Ok(VoicePreferences::default());
        }
        let data = fs::read_to_string(path)?;
        let prefs: VoicePreferences = serde_json::from_str(&data)?;
        Ok(prefs)
    }

    /// Atomically persists current preferences to disk.
    fn save_to_disk(&self, prefs: &VoicePreferences) -> Result<(), VoiceLibraryError> {
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(prefs)?;
        let temp_path = self.file_path.with_extension("tmp");
        fs::write(&temp_path, data)?;
        fs::rename(&temp_path, &self.file_path)?;
        Ok(())
    }

    /// Returns a snapshot of the current voice preferences.
    pub fn get_preferences(&self) -> VoicePreferences {
        self.preferences.read().unwrap().clone()
    }

    /// Toggles favorite status for a voice ID.
    pub fn toggle_favorite(&self, voice_id: &str) -> Result<bool, VoiceLibraryError> {
        let mut prefs = self.preferences.write().unwrap();
        let was_favorited = prefs.favorite_voice_ids.iter().any(|id| id == voice_id);
        if was_favorited {
            prefs.favorite_voice_ids.retain(|id| id != voice_id);
        } else {
            prefs.favorite_voice_ids.push(voice_id.to_string());
        }
        prefs.updated_at_epoch_ms = current_epoch_ms();
        self.save_to_disk(&prefs)?;
        Ok(!was_favorited)
    }

    /// Sets or clears the preferred global default voice.
    pub fn set_preferred_voice(&self, voice_id: Option<String>) -> Result<(), VoiceLibraryError> {
        let mut prefs = self.preferences.write().unwrap();
        prefs.preferred_voice_id = voice_id;
        prefs.updated_at_epoch_ms = current_epoch_ms();
        self.save_to_disk(&prefs)?;
        Ok(())
    }

    /// Records recent usage of a voice, optionally updating its last speaker ID.
    pub fn record_recent(
        &self,
        voice_id: &str,
        speaker_id: Option<u32>,
    ) -> Result<(), VoiceLibraryError> {
        let mut prefs = self.preferences.write().unwrap();
        // Remove existing occurrence to place it at the front
        prefs.recent_voice_ids.retain(|id| id != voice_id);
        prefs.recent_voice_ids.insert(0, voice_id.to_string());
        if prefs.recent_voice_ids.len() > 10 {
            prefs.recent_voice_ids.truncate(10);
        }
        if let Some(spk) = speaker_id {
            prefs.last_speaker_ids.insert(voice_id.to_string(), spk);
        }
        prefs.updated_at_epoch_ms = current_epoch_ms();
        self.save_to_disk(&prefs)?;
        Ok(())
    }
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
