//! Authoritative voice catalog and discovery for Open Media Workbench.

use super::models::VoiceCatalogEntry;

/// Returns the static, verified AI voice catalog.
///
/// Every voice references a corresponding model entry in the AI Model Manager.
pub fn get_voice_catalog() -> Vec<VoiceCatalogEntry> {
    vec![
        VoiceCatalogEntry {
            id: "piper-en-lessac-medium".to_string(),
            display_name: "Lessac (English - US)".to_string(),
            model_id: "piper-en-lessac-medium".to_string(),
            runtime_backend: "piper".to_string(),
            language: "en".to_string(),
            locale: "en-US".to_string(),
            version: "v1.0.0".to_string(),
            quality_tier: "medium".to_string(),
            artifact_size_bytes: 63_206_179,
            source_url: "https://huggingface.co/rhasspy/piper-voices".to_string(),
            license: "Public Domain".to_string(),
            attribution: "Donald Derrick / Lessac Dataset (Public Domain)".to_string(),
            speaker_count: 1,
            supported_speaker_ids: vec![0],
            sample_rate: 22050,
            description: "Clear, natural North American English voice suitable for narration, subtitles, and video voiceover.".to_string(),
            accent: Some("North American English".to_string()),
        },
        VoiceCatalogEntry {
            id: "piper-en-libritts-high".to_string(),
            display_name: "LibriTTS (English - Multi-speaker)".to_string(),
            model_id: "piper-en-libritts-high".to_string(),
            runtime_backend: "piper".to_string(),
            language: "en".to_string(),
            locale: "en-US".to_string(),
            version: "v1.0.0".to_string(),
            quality_tier: "high".to_string(),
            artifact_size_bytes: 110_450_000,
            source_url: "https://huggingface.co/rhasspy/piper-voices".to_string(),
            license: "CC-BY-4.0".to_string(),
            attribution: "Heiga Zen / LibriTTS Corpus (CC-BY-4.0)".to_string(),
            speaker_count: 904,
            supported_speaker_ids: (0..904).collect(),
            sample_rate: 22050,
            description: "Expressive multi-speaker English voice model with 904 selectable speaker variants.".to_string(),
            accent: Some("Varied English Accents".to_string()),
        },
        VoiceCatalogEntry {
            id: "piper-es-sharvard-medium".to_string(),
            display_name: "Sharvard (Spanish - Spain)".to_string(),
            model_id: "piper-es-sharvard-medium".to_string(),
            runtime_backend: "piper".to_string(),
            language: "es".to_string(),
            locale: "es-ES".to_string(),
            version: "v1.0.0".to_string(),
            quality_tier: "medium".to_string(),
            artifact_size_bytes: 64_800_000,
            source_url: "https://huggingface.co/rhasspy/piper-voices".to_string(),
            license: "Public Domain".to_string(),
            attribution: "Spanish Harvard Sentences Dataset (Public Domain)".to_string(),
            speaker_count: 1,
            supported_speaker_ids: vec![0],
            sample_rate: 22050,
            description: "Standard Castilian Spanish voice suitable for Spanish subtitling and voiceover.".to_string(),
            accent: Some("European Spanish".to_string()),
        },
        VoiceCatalogEntry {
            id: "piper-bn-ibrahim-medium".to_string(),
            display_name: "Ibrahim (Bengali - Bangladesh - Catalog Only)".to_string(),
            model_id: "piper-bn-ibrahim-medium".to_string(),
            runtime_backend: "piper".to_string(),
            language: "bn".to_string(),
            locale: "bn-BD".to_string(),
            version: "v1.0.0".to_string(),
            quality_tier: "medium".to_string(),
            artifact_size_bytes: 62_100_000,
            source_url: "https://huggingface.co/rhasspy/piper-voices".to_string(),
            license: "Public Domain".to_string(),
            attribution: "Ibrahim Bengali Speech Dataset (Public Domain)".to_string(),
            speaker_count: 1,
            supported_speaker_ids: vec![0],
            sample_rate: 22050,
            description: "Bengali speech synthesis voice. Not verified for inference in the current environment.".to_string(),
            accent: Some("Bangladeshi Bengali (বাংলা)".to_string()),
        },
    ]
}

/// Locates a voice catalog entry by its voice ID.
pub fn find_voice_by_id(id: &str) -> Option<VoiceCatalogEntry> {
    get_voice_catalog().into_iter().find(|v| v.id == id)
}

/// Locates a voice catalog entry by its underlying AI Model ID.
pub fn find_voice_by_model_id(model_id: &str) -> Option<VoiceCatalogEntry> {
    get_voice_catalog()
        .into_iter()
        .find(|v| v.model_id == model_id)
}

/// Filters catalog voices matching a given language or locale tag.
pub fn filter_voices_by_language(lang_or_locale: &str) -> Vec<VoiceCatalogEntry> {
    let query = lang_or_locale.trim().to_lowercase();
    get_voice_catalog()
        .into_iter()
        .filter(|v| {
            v.language.to_lowercase() == query
                || v.locale.to_lowercase() == query
                || v.locale.to_lowercase().starts_with(&format!("{query}-"))
        })
        .collect()
}

/// Returns a deduplicated list of supported language tags from the catalog.
pub fn get_supported_languages() -> Vec<String> {
    let mut langs: Vec<String> = get_voice_catalog()
        .into_iter()
        .map(|v| v.language)
        .collect();
    langs.sort();
    langs.dedup();
    langs
}

/// Returns a deduplicated list of supported locale tags from the catalog.
pub fn get_supported_locales() -> Vec<String> {
    let mut locales: Vec<String> = get_voice_catalog().into_iter().map(|v| v.locale).collect();
    locales.sort();
    locales.dedup();
    locales
}
