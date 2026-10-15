//! Strongly typed errors for the AI Voice Library domain.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceLibraryError {
    #[error("Voice not found in catalog: {0}")]
    VoiceNotFound(String),

    #[error("Model not found in catalog: {0}")]
    ModelNotFound(String),

    #[error("Voice model '{0}' is not installed")]
    ModelNotInstalled(String),

    #[error("Voice model '{0}' is missing required artifacts")]
    MissingArtifacts(String),

    #[error("Voice model '{0}' failed SHA-256 verification")]
    VerificationFailed(String),

    #[error("TTS runtime '{0}' is not available")]
    RuntimeUnavailable(String),

    #[error("Unsupported language '{language}' for voice '{voice_id}'")]
    UnsupportedLanguage { voice_id: String, language: String },

    #[error("Invalid speaker ID {speaker_id} for voice '{voice_id}' (supported speakers: {max_speakers})")]
    InvalidSpeakerId {
        voice_id: String,
        speaker_id: u32,
        max_speakers: u32,
    },

    #[error("Empty or invalid preview text")]
    InvalidPreviewText,

    #[error("Audio validation failed: {0}")]
    AudioValidationFailed(String),

    #[error("TTS synthesis error: {0}")]
    TtsError(#[from] crate::ai::tts::errors::TtsError),

    #[error("AI storage error: {0}")]
    StorageError(#[from] crate::ai::errors::AiError),

    #[error("Comparison requires two distinct installed voices: {0} vs {1}")]
    ComparisonRequiresDistinctVoices(String, String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
