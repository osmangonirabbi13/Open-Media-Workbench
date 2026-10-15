//! Server-side voice compatibility, runtime verification, and speaker validation.

use super::errors::VoiceLibraryError;
use super::models::{VoiceAvailabilityStatus, VoiceCatalogEntry, VoiceCompatibility};
use crate::ai::storage::AiStorageManager;
use crate::ai::tts::engine::TtsEngine;

/// Evaluates complete local availability and compatibility for a catalog voice.
pub fn check_voice_compatibility(
    voice: &VoiceCatalogEntry,
    storage: &AiStorageManager,
    tts_engine: &dyn TtsEngine,
    target_language: Option<&str>,
) -> VoiceCompatibility {
    let mut warnings = Vec::new();
    let runtime_ready = tts_engine.verify_runtime().is_ok();

    // Check language compatibility if specified
    if let Some(target_lang) = target_language {
        let clean = target_lang.trim().to_lowercase();
        let lang_match = voice.language.to_lowercase() == clean
            || voice.locale.to_lowercase() == clean
            || voice
                .locale
                .to_lowercase()
                .starts_with(&format!("{clean}-"));

        if !lang_match {
            return VoiceCompatibility {
                voice_id: voice.id.clone(),
                status: VoiceAvailabilityStatus::UnsupportedLanguage,
                is_ready: false,
                is_installed: storage.is_installed(&voice.model_id),
                runtime_ready,
                supported_languages: vec![voice.language.clone(), voice.locale.clone()],
                speaker_count: voice.speaker_count,
                warnings: vec![format!(
                    "Voice '{display}' ({lang}) does not support requested language '{clean}'",
                    display = voice.display_name,
                    lang = voice.locale
                )],
            };
        }
    }

    // Check if model is installed in shared AI Model storage
    let is_installed = storage.is_installed(&voice.model_id);
    if !is_installed {
        return VoiceCompatibility {
            voice_id: voice.id.clone(),
            status: VoiceAvailabilityStatus::AvailableForInstall,
            is_ready: false,
            is_installed: false,
            runtime_ready,
            supported_languages: vec![voice.language.clone(), voice.locale.clone()],
            speaker_count: voice.speaker_count,
            warnings: vec![
                "Voice model is not installed. Download via AI Model Manager to use.".into(),
            ],
        };
    }

    // Verify all required model artifacts (e.g. .onnx and .onnx.json)
    if let Some(manifest) = crate::ai::catalog::find_catalog_model(&voice.model_id) {
        if let Err(e) = storage.verify_installed_artifacts(&manifest) {
            warnings.push(format!("Artifact verification issue: {e}"));
            return VoiceCompatibility {
                voice_id: voice.id.clone(),
                status: VoiceAvailabilityStatus::InstalledVerificationRequired,
                is_ready: false,
                is_installed: true,
                runtime_ready,
                supported_languages: vec![voice.language.clone(), voice.locale.clone()],
                speaker_count: voice.speaker_count,
                warnings,
            };
        }
    } else {
        warnings.push("Voice model manifest not found in catalog".into());
        return VoiceCompatibility {
            voice_id: voice.id.clone(),
            status: VoiceAvailabilityStatus::Incompatible,
            is_ready: false,
            is_installed: true,
            runtime_ready,
            supported_languages: vec![voice.language.clone(), voice.locale.clone()],
            speaker_count: voice.speaker_count,
            warnings,
        };
    }

    // Verify runtime presence and compatibility
    if !runtime_ready {
        warnings.push("Required offline TTS runtime is missing or incompatible.".into());
        return VoiceCompatibility {
            voice_id: voice.id.clone(),
            status: VoiceAvailabilityStatus::InstalledRuntimeMissing,
            is_ready: false,
            is_installed: true,
            runtime_ready: false,
            supported_languages: vec![voice.language.clone(), voice.locale.clone()],
            speaker_count: voice.speaker_count,
            warnings,
        };
    }

    // Model and runtime are both verified and ready
    VoiceCompatibility {
        voice_id: voice.id.clone(),
        status: VoiceAvailabilityStatus::Ready,
        is_ready: true,
        is_installed: true,
        runtime_ready: true,
        supported_languages: vec![voice.language.clone(), voice.locale.clone()],
        speaker_count: voice.speaker_count,
        warnings,
    }
}

/// Validates that an optional speaker ID is within valid bounds for a voice.
pub fn validate_speaker_id(
    voice: &VoiceCatalogEntry,
    speaker_id: Option<u32>,
) -> Result<u32, VoiceLibraryError> {
    let id = speaker_id.unwrap_or(0);
    if voice.speaker_count == 0 {
        return Ok(0);
    }
    if !voice.supported_speaker_ids.contains(&id) {
        return Err(VoiceLibraryError::InvalidSpeakerId {
            voice_id: voice.id.clone(),
            speaker_id: id,
            max_speakers: voice.speaker_count,
        });
    }
    Ok(id)
}
