//! AI Model Management error types.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    #[error("Model not found in catalog: {0}")]
    ModelNotFound(String),

    #[error("Model is already installed: {0}")]
    ModelAlreadyInstalled(String),

    #[error("Model cannot be installed: {0}")]
    ModelNotInstallable(String),

    #[error("Model is currently in use: {0}")]
    ModelInUse(String),

    #[error("Invalid model identifier: {0}")]
    InvalidModelId(String),

    #[error("Untrusted or invalid download URL: {0}")]
    UntrustedUrl(String),

    #[error("Network error during download: {0}")]
    DownloadError(String),

    #[error("Download was cancelled by user")]
    DownloadCancelled,

    #[error("Artifact size mismatch: expected {expected} bytes, got {actual} bytes")]
    SizeMismatch { expected: u64, actual: u64 },

    #[error("SHA-256 integrity mismatch: expected {expected}, computed {actual}")]
    Sha256Mismatch { expected: String, actual: String },

    #[error("Filesystem/storage error: {0}")]
    StorageError(String),

    #[error("Path traversal or insecure path rejected: {0}")]
    PathTraversalError(String),

    #[error("Insufficient disk space: required {required} bytes, available {available} bytes")]
    InsufficientDiskSpace { required: u64, available: u64 },

    #[error("Network connection timed out: {0}")]
    NetworkTimeout(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Model artifact is corrupted or missing: {0}")]
    CorruptedModel(String),

    #[error("Inference runtime is not available: {0}")]
    RuntimeUnavailable(String),

    #[error("Active download already in progress for model: {0}")]
    DownloadInProgress(String),
}
