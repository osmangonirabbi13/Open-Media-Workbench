//! Error types for subtitle styling, ASS generation, and burn-in export.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StylingError {
    #[error("Invalid style property: {0}")]
    InvalidProperty(String),

    #[error("Color parse error: {0}")]
    InvalidColor(String),

    #[error("Preset error: {0}")]
    PresetError(String),

    #[error("Cannot modify or delete built-in preset '{0}'")]
    BuiltinPresetImmutable(String),

    #[error("ASS generation error: {0}")]
    AssGenerationError(String),

    #[error("FFmpeg execution error: {0}")]
    FfmpegError(String),

    #[error("FFprobe validation error: {0}")]
    ValidationFailed(String),

    #[error("Output file not found or empty: {0}")]
    EmptyOutput(String),

    #[error("Export job was cancelled")]
    Cancelled,

    #[error("Staging directory error: {0}")]
    StagingError(String),

    #[error("Media authorization error: {0}")]
    Unauthorized(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}
