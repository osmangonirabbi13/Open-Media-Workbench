//! Bounded local voice preview cache, real speech synthesis, and voice comparison.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::catalog::find_voice_by_id;
use super::compatibility::validate_speaker_id;
use super::errors::VoiceLibraryError;
use super::models::{
    VoiceComparisonRequest, VoiceComparisonResult, VoicePreviewRequest, VoicePreviewResult,
};
use crate::ai::storage::AiStorageManager;
use crate::ai::tts::audio::validate_wav_file;
use crate::ai::tts::engine::TtsEngine;
use crate::ai::tts::models::{TtsSynthesisInput, TtsVoiceSettings};

const DEFAULT_PREVIEW_TEXT: &str = "Welcome to Open Media Workbench.";
const MAX_CACHE_ENTRIES: usize = 50;
const MAX_CACHE_SIZE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheEntryMeta {
    preview_id: String,
    file_name: String,
    size_bytes: u64,
    sample_rate: u32,
    channel_count: u16,
    duration_us: u64,
    last_accessed_epoch_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct CacheLedger {
    entries: HashMap<String, CacheEntryMeta>,
}

/// Manages real speech preview generation and bounded LRU caching.
pub struct VoicePreviewManager {
    cache_dir: PathBuf,
    ledger_path: PathBuf,
    storage: Arc<AiStorageManager>,
    tts_engine: Arc<dyn TtsEngine>,
    ledger: Mutex<CacheLedger>,
}

impl VoicePreviewManager {
    pub fn new(
        cache_dir: PathBuf,
        storage: Arc<AiStorageManager>,
        tts_engine: Arc<dyn TtsEngine>,
    ) -> Self {
        let _ = fs::create_dir_all(&cache_dir);
        let ledger_path = cache_dir.join("preview_cache_ledger.json");
        let ledger_data = Self::load_ledger(&ledger_path).unwrap_or_default();

        Self {
            cache_dir,
            ledger_path,
            storage,
            tts_engine,
            ledger: Mutex::new(ledger_data),
        }
    }

    fn load_ledger(path: &Path) -> Option<CacheLedger> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_ledger(&self, ledger: &CacheLedger) {
        if let Ok(data) = serde_json::to_string_pretty(ledger) {
            let temp_path = self.ledger_path.with_extension("tmp");
            let _ = fs::write(&temp_path, data);
            let _ = fs::rename(&temp_path, &self.ledger_path);
        }
    }

    /// Computes deterministic content-addressed preview cache key.
    pub fn compute_cache_key(
        voice_id: &str,
        model_hash: &str,
        speaker_id: u32,
        settings: &TtsVoiceSettings,
        text: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(voice_id.as_bytes());
        hasher.update(b":");
        hasher.update(model_hash.as_bytes());
        hasher.update(b":");
        hasher.update(speaker_id.to_string().as_bytes());
        hasher.update(b":");
        hasher.update(settings.speed_scale.to_bits().to_le_bytes());
        hasher.update(settings.noise_scale.to_bits().to_le_bytes());
        hasher.update(settings.noise_w.to_bits().to_le_bytes());
        hasher.update(b":");
        hasher.update(text.trim().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generates or serves cached voice preview audio.
    pub fn generate_preview(
        &self,
        req: &VoicePreviewRequest,
    ) -> Result<VoicePreviewResult, VoiceLibraryError> {
        let voice = find_voice_by_id(&req.voice_id)
            .ok_or_else(|| VoiceLibraryError::VoiceNotFound(req.voice_id.clone()))?;

        // 1. Verify model is installed
        if !self.storage.is_installed(&voice.model_id) {
            return Err(VoiceLibraryError::ModelNotInstalled(voice.model_id));
        }

        // 2. Resolve manifest & validate artifacts
        let manifest = crate::ai::catalog::find_catalog_model(&voice.model_id)
            .ok_or_else(|| VoiceLibraryError::ModelNotFound(voice.model_id.clone()))?;
        self.storage
            .verify_installed_artifacts(&manifest)
            .map_err(|e| VoiceLibraryError::MissingArtifacts(e.to_string()))?;

        // 3. Resolve paths
        let model_dir = self.storage.resolve_model_dir(&voice.model_id)?;
        let model_path = model_dir.join(&manifest.artifact_filename);
        let config_artifact = manifest
            .additional_artifacts
            .iter()
            .find(|a| a.role == "config")
            .ok_or_else(|| {
                VoiceLibraryError::MissingArtifacts("Voice configuration artifact not found".into())
            })?;
        let config_path = model_dir.join(&config_artifact.filename);

        // 4. Validate speaker and text
        let speaker_id = validate_speaker_id(&voice, req.speaker_id)?;
        let text = req
            .custom_text
            .as_deref()
            .unwrap_or(DEFAULT_PREVIEW_TEXT)
            .trim();
        if text.is_empty() {
            return Err(VoiceLibraryError::InvalidPreviewText);
        }

        let settings = req.settings.clone().unwrap_or_default();
        let model_hash = manifest.expected_sha256.as_deref().unwrap_or("unverified");

        let cache_key = Self::compute_cache_key(&voice.id, model_hash, speaker_id, &settings, text);
        let cache_file_name = format!("{cache_key}.wav");
        let cached_path = self.cache_dir.join(&cache_file_name);

        // Check if valid cached preview already exists
        {
            let mut ledger = self.ledger.lock().unwrap();
            if let Some(entry) = ledger.entries.get_mut(&cache_key) {
                if cached_path.exists() {
                    if let Ok(meta) = validate_wav_file(&cached_path) {
                        entry.last_accessed_epoch_ms = current_epoch_ms();
                        self.save_ledger(&ledger);
                        return Ok(VoicePreviewResult {
                            preview_id: cache_key,
                            voice_id: voice.id,
                            text: text.to_string(),
                            staged_audio_path: cached_path.to_string_lossy().to_string(),
                            sample_rate: meta.sample_rate,
                            channel_count: meta.channel_count,
                            duration_us: meta.duration_us,
                            cached: true,
                            created_at_epoch_ms: current_epoch_ms(),
                        });
                    }
                }
                // Corrupt or missing, remove from ledger
                ledger.entries.remove(&cache_key);
            }
        }

        // Lock model during synthesis to prevent concurrent deletion
        let _lock = self
            .storage
            .acquire_lock(&voice.model_id)
            .map_err(VoiceLibraryError::StorageError)?;

        // Synthesize fresh preview
        let temp_wav = self.cache_dir.join(format!("{cache_key}.part.wav"));
        let synth_input = TtsSynthesisInput {
            text: text.to_string(),
            voice_id: voice.id.clone(),
            model_path,
            config_path,
            settings: TtsVoiceSettings {
                speaker_id: Some(speaker_id),
                ..settings
            },
        };

        let wav_meta = self
            .tts_engine
            .synthesize(&synth_input, &temp_wav, None)
            .map_err(VoiceLibraryError::TtsError)?;

        // Validate audio output
        let validated = validate_wav_file(&temp_wav)
            .map_err(|e| VoiceLibraryError::AudioValidationFailed(e.to_string()))?;

        // Atomically commit to cache
        let _ = fs::rename(&temp_wav, &cached_path);
        let file_size = fs::metadata(&cached_path).map(|m| m.len()).unwrap_or(0);

        // Update ledger and evict if necessary
        {
            let mut ledger = self.ledger.lock().unwrap();
            ledger.entries.insert(
                cache_key.clone(),
                CacheEntryMeta {
                    preview_id: cache_key.clone(),
                    file_name: cache_file_name,
                    size_bytes: file_size,
                    sample_rate: validated.sample_rate,
                    channel_count: validated.channel_count,
                    duration_us: validated.duration_us,
                    last_accessed_epoch_ms: current_epoch_ms(),
                },
            );
            self.enforce_cache_limits(&mut ledger);
            self.save_ledger(&ledger);
        }

        Ok(VoicePreviewResult {
            preview_id: cache_key,
            voice_id: voice.id,
            text: text.to_string(),
            staged_audio_path: cached_path.to_string_lossy().to_string(),
            sample_rate: wav_meta.sample_rate,
            channel_count: wav_meta.channel_count,
            duration_us: wav_meta.duration_us,
            cached: false,
            created_at_epoch_ms: current_epoch_ms(),
        })
    }

    /// Synthesizes and compares two voices using identical sample text.
    pub fn compare_voices(
        &self,
        req: &VoiceComparisonRequest,
    ) -> Result<VoiceComparisonResult, VoiceLibraryError> {
        if req.voice_id_a == req.voice_id_b {
            return Err(VoiceLibraryError::ComparisonRequiresDistinctVoices(
                req.voice_id_a.clone(),
                req.voice_id_b.clone(),
            ));
        }

        let preview_a = self.generate_preview(&VoicePreviewRequest {
            voice_id: req.voice_id_a.clone(),
            custom_text: req.text.clone(),
            speaker_id: None,
            settings: None,
        })?;

        let preview_b = self.generate_preview(&VoicePreviewRequest {
            voice_id: req.voice_id_b.clone(),
            custom_text: req.text.clone(),
            speaker_id: None,
            settings: None,
        })?;

        let duration_diff_us = (preview_a.duration_us as i64) - (preview_b.duration_us as i64);

        Ok(VoiceComparisonResult {
            voice_a: preview_a,
            voice_b: preview_b,
            duration_diff_us,
        })
    }

    /// Evicts oldest cache entries when limits are exceeded.
    fn enforce_cache_limits(&self, ledger: &mut CacheLedger) {
        let total_size: u64 = ledger.entries.values().map(|e| e.size_bytes).sum();
        if ledger.entries.len() <= MAX_CACHE_ENTRIES && total_size <= MAX_CACHE_SIZE_BYTES {
            return;
        }

        let mut sorted_keys: Vec<(String, u64)> = ledger
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.last_accessed_epoch_ms))
            .collect();
        sorted_keys.sort_by_key(|&(_, time)| time);

        for (k, _) in sorted_keys {
            if ledger.entries.len() <= MAX_CACHE_ENTRIES
                && ledger.entries.values().map(|e| e.size_bytes).sum::<u64>()
                    <= MAX_CACHE_SIZE_BYTES
            {
                break;
            }
            if let Some(entry) = ledger.entries.remove(&k) {
                let file_path = self.cache_dir.join(&entry.file_name);
                let _ = fs::remove_file(file_path);
            }
        }
    }

    /// Purges all cached voice preview files and resets the ledger.
    pub fn clear_cache(&self) -> Result<usize, VoiceLibraryError> {
        let mut ledger = self.ledger.lock().unwrap();
        let count = ledger.entries.len();
        for entry in ledger.entries.values() {
            let file_path = self.cache_dir.join(&entry.file_name);
            let _ = fs::remove_file(file_path);
        }
        ledger.entries.clear();
        self.save_ledger(&ledger);
        Ok(count)
    }
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
