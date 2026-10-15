use thiserror::Error;

/// Typed error domain for offline subtitle translation.
#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("Translation model '{model_id}' was not found in the curated catalog.")]
    ModelNotFound { model_id: String },

    #[error("Translation model '{model_id}' is not installed: {details}")]
    ModelNotInstalled { model_id: String, details: String },

    #[error("Integrity check failed for model '{model_id}': {details}")]
    ModelIntegrityFailed { model_id: String, details: String },

    #[error("Translation runtime is unavailable: {0}")]
    RuntimeUnavailable(String),

    #[error(
        "Language pair '{source_lang}' -> '{target_lang}' is not supported by model '{model_id}'."
    )]
    LanguagePairNotSupported {
        source_lang: String,
        target_lang: String,
        model_id: String,
    },

    #[error("Subtitle track '{0}' was not found in document.")]
    TrackNotFound(String),

    #[error("Subtitle track '{0}' has no cues to translate.")]
    EmptyTrack(String),

    #[error("Translation inference failed: {0}")]
    InferenceFailed(String),

    #[error("Output validation failed: {0}")]
    OutputValidationFailed(String),

    #[error("Translation job was cancelled by user.")]
    CancellationRequested,

    #[error("Failed to acquire model lock: {0}")]
    LockAcquisitionFailed(String),

    #[error("Project conflict: {0}")]
    ProjectConflict(String),

    #[error("IO error during translation: {0}")]
    IoError(String),
}
