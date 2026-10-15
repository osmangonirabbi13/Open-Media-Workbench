use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::editing::time::MediaTimeUs;

pub const TRANSLATION_RESULT_SCHEMA_VERSION: u32 = 1;

/// Supported language pair metadata for subtitle translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationLanguagePair {
    pub source_code: String,
    pub source_name: String,
    pub target_code: String,
    pub target_name: String,
    pub compatible_model_ids: Vec<String>,
    pub is_verified: bool,
}

/// Request to translate an existing subtitle track.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRequest {
    pub document_id: Option<String>,
    pub track_id: String,
    pub source_language: String,
    pub target_language: String,
    pub model_id: String,
    pub custom_runtime_path: Option<PathBuf>,
    pub batch_size: Option<usize>,
    #[serde(default)]
    pub glossary_id: Option<String>,
}

/// Progress stage for async translation jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TranslationProgressStage {
    #[default]
    Queued,
    CheckingModel,
    Preparing,
    Translating,
    Validating,
    Completed,
    Failed,
    Cancelled,
}

/// Dynamic status of an in-flight or completed translation job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationStatus {
    pub job_id: String,
    pub stage: TranslationProgressStage,
    pub progress_percent: Option<f32>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub translated_cues: usize,
    pub total_cues: usize,
}

/// Single translated subtitle cue with strict timing preservation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslatedCue {
    pub source_cue_id: String,
    pub translated_cue_id: String,
    pub original_text: String,
    pub translated_text: String,
    pub source_start_us: MediaTimeUs,
    pub source_end_us: MediaTimeUs,
    pub target_start_us: MediaTimeUs,
    pub target_end_us: MediaTimeUs,
    pub warnings: Vec<String>,
    pub model_provenance: String,
}

/// Comprehensive result of a completed subtitle translation job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub schema_version: u32,
    pub job_id: String,
    pub original_track_id: String,
    pub model_id: String,
    pub runtime_version: String,
    pub source_language: String,
    pub target_language: String,
    pub translated_cues: Vec<TranslatedCue>,
    pub untranslated_cue_ids: Vec<String>,
    pub warnings: Vec<String>,
    pub elapsed_ms: u64,
}

/// Returns the authoritative list of supported language pairs.
pub fn get_supported_language_pairs() -> Vec<TranslationLanguagePair> {
    vec![
        TranslationLanguagePair {
            source_code: "en".to_string(),
            source_name: "English".to_string(),
            target_code: "es".to_string(),
            target_name: "Spanish (Español)".to_string(),
            compatible_model_ids: vec![
                "qwen2.5-0.5b-instruct".to_string(),
                "opus-mt-en-es".to_string(),
            ],
            is_verified: true,
        },
        TranslationLanguagePair {
            source_code: "en".to_string(),
            source_name: "English".to_string(),
            target_code: "bn".to_string(),
            target_name: "Bengali (বাংলা)".to_string(),
            compatible_model_ids: vec!["qwen2.5-0.5b-instruct".to_string()],
            is_verified: true,
        },
        TranslationLanguagePair {
            source_code: "es".to_string(),
            source_name: "Spanish (Español)".to_string(),
            target_code: "en".to_string(),
            target_name: "English".to_string(),
            compatible_model_ids: vec!["qwen2.5-0.5b-instruct".to_string()],
            is_verified: true,
        },
        TranslationLanguagePair {
            source_code: "bn".to_string(),
            source_name: "Bengali (বাংলা)".to_string(),
            target_code: "en".to_string(),
            target_name: "English".to_string(),
            compatible_model_ids: vec!["qwen2.5-0.5b-instruct".to_string()],
            is_verified: true,
        },
        TranslationLanguagePair {
            source_code: "en".to_string(),
            source_name: "English".to_string(),
            target_code: "fr".to_string(),
            target_name: "French (Français)".to_string(),
            compatible_model_ids: vec!["qwen2.5-0.5b-instruct".to_string()],
            is_verified: false,
        },
        TranslationLanguagePair {
            source_code: "en".to_string(),
            source_name: "English".to_string(),
            target_code: "de".to_string(),
            target_name: "German (Deutsch)".to_string(),
            compatible_model_ids: vec!["qwen2.5-0.5b-instruct".to_string()],
            is_verified: false,
        },
        TranslationLanguagePair {
            source_code: "en".to_string(),
            source_name: "English".to_string(),
            target_code: "ja".to_string(),
            target_name: "Japanese (日本語)".to_string(),
            compatible_model_ids: vec!["qwen2.5-0.5b-instruct".to_string()],
            is_verified: false,
        },
    ]
}
