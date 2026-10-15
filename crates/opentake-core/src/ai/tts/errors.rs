//! Error types for the Text-to-Speech domain.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TtsError {
    #[error("TTS runtime binary not found or unavailable: {0}")]
    MissingRuntime(String),

    #[error("Voice model not found in catalog or storage: {0}")]
    MissingModel(String),

    #[error("Required voice model artifact is missing: {0}")]
    MissingArtifact(String),

    #[error("Voice model file is corrupted or failed integrity check: {0}")]
    CorruptedModel(String),

    #[error("Voice model {0} is currently locked by another active job")]
    ModelInUse(String),

    #[error(
        "Language '{requested}' is incompatible with voice '{voice_id}'. Supported: {supported:?}"
    )]
    LanguageIncompatible {
        requested: String,
        voice_id: String,
        supported: Vec<String>,
    },

    #[error("Invalid or empty TTS input text: {0}")]
    InvalidInput(String),

    #[error("Invalid voice settings: {0}")]
    InvalidVoiceSettings(String),

    #[error("Synthesis process failed: {0}")]
    ProcessFailed(String),

    #[error("Generated audio validation failed: {0}")]
    AudioValidationFailed(String),

    #[error("Synthesis job was cancelled by user")]
    Cancellation,

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Subtitle track {0} not found")]
    TrackNotFound(String),

    #[error("Active synthesis job {0} not found")]
    JobNotFound(String),
}

impl From<std::io::Error> for TtsError {
    fn from(err: std::io::Error) -> Self {
        TtsError::IoError(err.to_string())
    }
}
