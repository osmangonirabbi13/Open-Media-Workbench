use thiserror::Error;

/// Strongly typed errors originating from the offline transcription engine.
#[derive(Debug, Error)]
pub enum TranscriptionError {
    #[error("Audio extraction failed: {0}")]
    AudioExtractionFailed(String),

    #[error("No audio stream found in source media: {0}")]
    NoAudioStream(String),

    #[error("Unauthorized media asset: {0}")]
    UnauthorizedAsset(String),

    #[error("Asset not found in media library: {0}")]
    AssetNotFound(String),

    #[error("AI model '{model_id}' is not installed: {details}")]
    ModelNotInstalled { model_id: String, details: String },

    #[error("AI model '{model_id}' is incompatible: {details}")]
    ModelIncompatible { model_id: String, details: String },

    #[error("AI model '{model_id}' failed integrity check: {details}")]
    ModelIntegrityFailed { model_id: String, details: String },

    #[error("Language '{language}' is not supported by model '{model_id}': {details}")]
    LanguageNotSupported {
        language: String,
        model_id: String,
        details: String,
    },

    #[error("Whisper runtime is unavailable or not found: {0}")]
    RuntimeUnavailable(String),

    #[error("Whisper runtime is incompatible or failed validation: {0}")]
    RuntimeIncompatible(String),

    #[error("Whisper process execution failed with exit code {exit_code:?}: {details}")]
    ProcessFailed {
        exit_code: Option<i32>,
        details: String,
    },

    #[error("Failed to parse Whisper output: {0}")]
    OutputParsingFailed(String),

    #[error("Subtitle conversion failed: {0}")]
    SubtitleConversionFailed(String),

    #[error("Transcription job was cancelled by user")]
    JobCancelled,

    #[error("Transcription job not found: {0}")]
    JobNotFound(String),

    #[error("Maximum concurrent transcription jobs reached ({limit})")]
    ConcurrencyLimitReached { limit: usize },

    #[error("I/O error during transcription: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}
