use serde::{Deserialize, Serialize};

use crate::editing::time::MediaTimeUs;

/// Current schema version for TranscriptionResult serialization.
pub const TRANSCRIPTION_RESULT_SCHEMA_VERSION: u32 = 1;

/// Request payload to initiate an offline transcription job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionRequest {
    /// Authorized media asset ID from MediaLibrarySession.
    pub asset_id: String,
    /// Verified installed Whisper model ID from AiModelManager (e.g. "whisper-tiny").
    pub model_id: String,
    /// Selected transcription language (e.g. "auto", "en", "bn").
    pub language: String,
    /// Optional custom track display name for the generated subtitle track.
    pub output_track_name: Option<String>,
    /// Optional user-authorized custom whisper runtime path.
    pub custom_runtime_path: Option<String>,
    /// Optional CPU thread limit for inference (defaults to safe concurrency).
    pub threads: Option<u32>,
    /// Optional start boundary in integer microseconds.
    pub start_time_us: Option<i64>,
    /// Optional end boundary in integer microseconds.
    pub end_time_us: Option<i64>,
}

/// Operational processing stages of an active transcription job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptionProgressStage {
    Queued,
    PreparingAudio,
    LoadingModel,
    Transcribing,
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

/// Dynamic status update for an in-flight or completed transcription job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionStatus {
    pub job_id: String,
    pub stage: TranscriptionProgressStage,
    pub progress_percent: Option<f32>,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// A single timestamped speech recognition segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionSegment {
    /// Stable segment identifier.
    pub id: String,
    /// Inclusive start timestamp in integer microseconds ($start\_us \ge 0$).
    pub start_us: MediaTimeUs,
    /// Exclusive end timestamp in integer microseconds ($end\_us > start\_us$).
    pub end_us: MediaTimeUs,
    /// Recognized transcription text (normalized Unicode LF breaks).
    pub text: String,
    /// Language detected for this segment if reported by runtime.
    pub detected_language: Option<String>,
    /// Optional confidence evidence where available (0.0 to 1.0).
    pub confidence: Option<f32>,
}

/// Canonical result of a completed offline transcription job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub schema_version: u32,
    pub job_id: String,
    pub asset_id: String,
    pub model_id: String,
    pub runtime_version: String,
    pub requested_language: String,
    pub detected_language: Option<String>,
    pub segments: Vec<TranscriptionSegment>,
    pub processing_duration_ms: u64,
    pub audio_duration_us: Option<i64>,
    pub warnings: Vec<String>,
}

/// Metadata describing a supported transcription language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionLanguage {
    pub code: String,
    pub label: String,
    pub is_multilingual_only: bool,
}

/// System and user-configured whisper runtime availability and capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperRuntimeCapabilities {
    pub is_available: bool,
    pub runtime_path: Option<String>,
    pub runtime_version: Option<String>,
    pub is_custom: bool,
    pub supported_languages: Vec<TranscriptionLanguage>,
    pub errors: Vec<String>,
}

/// Curated list of validated languages supported by Whisper.
pub fn get_supported_whisper_languages() -> Vec<TranscriptionLanguage> {
    vec![
        TranscriptionLanguage {
            code: "auto".to_string(),
            label: "Auto Detect".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "en".to_string(),
            label: "English".to_string(),
            is_multilingual_only: false,
        },
        TranscriptionLanguage {
            code: "bn".to_string(),
            label: "Bengali (বাংলা)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "es".to_string(),
            label: "Spanish (Español)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "fr".to_string(),
            label: "French (Français)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "de".to_string(),
            label: "German (Deutsch)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "hi".to_string(),
            label: "Hindi (हिन्दी)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "ja".to_string(),
            label: "Japanese (日本語)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "zh".to_string(),
            label: "Chinese (中文)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "ar".to_string(),
            label: "Arabic (العربية)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "ru".to_string(),
            label: "Russian (Русский)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "pt".to_string(),
            label: "Portuguese (Português)".to_string(),
            is_multilingual_only: true,
        },
        TranscriptionLanguage {
            code: "it".to_string(),
            label: "Italian (Italiano)".to_string(),
            is_multilingual_only: true,
        },
    ]
}
