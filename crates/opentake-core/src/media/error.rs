use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Structured, user-safe error model for OpenTake media engine operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum MediaEngineError {
    #[error("Executable binary for {tool} was not found in any search location.")]
    BinaryNotFound {
        tool: String,
        searched_paths: Vec<String>,
    },

    #[error("File at {path} exists but is not executable: {reason}")]
    BinaryNotExecutable { path: String, reason: String },

    #[error("Executable candidate at {path} is invalid: {reason}")]
    InvalidExecutable { path: String, reason: String },

    #[error("Tool at executable path reported unexpected identity: expected '{expected}', found '{found}'")]
    WrongToolIdentity { expected: String, found: String },

    #[error("Installed {tool} version '{found}' is unsupported (minimum required: '{required}')")]
    UnsupportedVersion {
        tool: String,
        found: String,
        required: String,
    },

    #[error("Failed to spawn process '{command}': {details}")]
    SpawnFailed { command: String, details: String },

    #[error("Permission denied when accessing '{path}'")]
    PermissionDenied { path: String },

    #[error("Process '{command}' timed out after {duration_ms} ms")]
    Timeout { command: String, duration_ms: u64 },

    #[error("Process '{command}' was cancelled by user or application")]
    Cancelled { command: String },

    #[error("Process '{command}' exited with non-zero status ({code:?}): {stderr}")]
    NonZeroExit {
        command: String,
        code: Option<i32>,
        stderr: String,
    },

    #[error("Process output limit exceeded ({limit_bytes} bytes max)")]
    OutputLimitExceeded { limit_bytes: usize },

    #[error("Failed to parse {field} from tool output: {raw}")]
    OutputParseFailed { field: String, raw: String },

    #[error("Failed to reap or clean up child process PID {pid}: {reason}")]
    ProcessCleanupFailed { pid: u32, reason: String },

    #[error("File at '{path}' was not found")]
    FileNotFound { path: String },

    #[error("Path '{path}' is not a regular file")]
    NotARegularFile { path: String },

    #[error("File at '{path}' is empty (0 bytes)")]
    EmptyFile { path: String },

    #[error("File at '{path}' ({size_bytes} bytes) exceeds maximum supported import size ({limit_bytes} bytes)")]
    FileTooLarge {
        path: String,
        size_bytes: u64,
        limit_bytes: u64,
    },

    #[error("Invalid media file path '{path}': {reason}")]
    InvalidPath { path: String, reason: String },

    #[error("Media file at '{path}' appears corrupted or unreadable: {details}")]
    CorruptMedia { path: String, details: String },

    #[error("Media file at '{path}' contains no audio or video streams")]
    NoStreamsFound { path: String },

    #[error("File at '{path}' has already been imported into this session")]
    DuplicateFile { path: String },

    #[error("Path '{path}' was not authorized through native user selection")]
    UnauthorizedPath { path: String },

    #[error("Media asset '{id}' was not found in the active session")]
    AssetNotFound { id: String },

    #[error("Media asset '{id}' has been revoked or removed from the session")]
    AssetRevoked { id: String },

    #[error("Invalid HTTP byte range request: {details}")]
    InvalidRange { details: String },

    #[error("HTTP byte range {start}-{end} is unsatisfiable for resource size {total}")]
    RangeNotSatisfiable { start: u64, end: u64, total: u64 },

    #[error("Failed to generate preview proxy for asset '{id}': {reason}")]
    ProxyGenerationFailed { id: String, reason: String },

    #[error("Preview proxy generation for asset '{id}' was cancelled")]
    ProxyCancelled { id: String },

    #[error("Internal media engine error: {0}")]
    Internal(String),
}

impl MediaEngineError {
    /// Returns a stable programmatic error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BinaryNotFound { .. } => "BINARY_NOT_FOUND",
            Self::BinaryNotExecutable { .. } => "BINARY_NOT_EXECUTABLE",
            Self::InvalidExecutable { .. } => "INVALID_EXECUTABLE",
            Self::WrongToolIdentity { .. } => "WRONG_TOOL_IDENTITY",
            Self::UnsupportedVersion { .. } => "UNSUPPORTED_VERSION",
            Self::SpawnFailed { .. } => "SPAWN_FAILED",
            Self::PermissionDenied { .. } => "PERMISSION_DENIED",
            Self::Timeout { .. } => "TIMEOUT",
            Self::Cancelled { .. } => "CANCELLED",
            Self::NonZeroExit { .. } => "NON_ZERO_EXIT",
            Self::OutputLimitExceeded { .. } => "OUTPUT_LIMIT_EXCEEDED",
            Self::OutputParseFailed { .. } => "OUTPUT_PARSE_FAILED",
            Self::ProcessCleanupFailed { .. } => "PROCESS_CLEANUP_FAILED",
            Self::FileNotFound { .. } => "FILE_NOT_FOUND",
            Self::NotARegularFile { .. } => "NOT_A_REGULAR_FILE",
            Self::EmptyFile { .. } => "EMPTY_FILE",
            Self::FileTooLarge { .. } => "FILE_TOO_LARGE",
            Self::InvalidPath { .. } => "INVALID_PATH",
            Self::CorruptMedia { .. } => "CORRUPT_MEDIA",
            Self::NoStreamsFound { .. } => "NO_STREAMS_FOUND",
            Self::DuplicateFile { .. } => "DUPLICATE_FILE",
            Self::UnauthorizedPath { .. } => "UNAUTHORIZED_PATH",
            Self::AssetNotFound { .. } => "ASSET_NOT_FOUND",
            Self::AssetRevoked { .. } => "ASSET_REVOKED",
            Self::InvalidRange { .. } => "INVALID_RANGE",
            Self::RangeNotSatisfiable { .. } => "RANGE_NOT_SATISFIABLE",
            Self::ProxyGenerationFailed { .. } => "PROXY_GENERATION_FAILED",
            Self::ProxyCancelled { .. } => "PROXY_CANCELLED",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Returns a user-safe message suitable for frontend display without leaking private paths.
    pub fn user_message(&self) -> String {
        match self {
            Self::BinaryNotFound { tool, .. } => {
                format!("{tool} executable was not found on this system. Please verify FFmpeg/FFprobe installation.")
            }
            Self::BinaryNotExecutable { .. } => {
                "Media processing binary exists but lacks execution permissions.".to_string()
            }
            Self::InvalidExecutable { reason, .. } => {
                format!("Media binary is invalid: {reason}")
            }
            Self::WrongToolIdentity { expected, found } => {
                format!("Unexpected tool identity: expected {expected}, detected {found}.")
            }
            Self::UnsupportedVersion {
                tool,
                found,
                required,
            } => {
                format!("{tool} version {found} is not supported. Minimum required is {required}.")
            }
            Self::SpawnFailed { .. } => {
                "Failed to launch background media diagnostic process.".to_string()
            }
            Self::PermissionDenied { .. } => {
                "Permission denied while attempting to access or execute media file.".to_string()
            }
            Self::Timeout { duration_ms, .. } => {
                format!("Media tool execution timed out after {} ms.", duration_ms)
            }
            Self::Cancelled { .. } => "Media operation was cancelled.".to_string(),
            Self::NonZeroExit { code, .. } => {
                format!("Media tool exited with error code {:?}.", code)
            }
            Self::OutputLimitExceeded { limit_bytes } => {
                format!(
                    "Media tool diagnostic output exceeded safety limit of {} bytes.",
                    limit_bytes
                )
            }
            Self::OutputParseFailed { field, .. } => {
                format!("Failed to parse {field} from media tool response.")
            }
            Self::ProcessCleanupFailed { .. } => {
                "Warning: An orphaned media process could not be terminated cleanly.".to_string()
            }
            Self::FileNotFound { .. } => "The selected media file does not exist.".to_string(),
            Self::NotARegularFile { .. } => {
                "The selected path is a directory or special device, not a regular file."
                    .to_string()
            }
            Self::EmptyFile { .. } => "The selected media file is empty (0 bytes).".to_string(),
            Self::FileTooLarge {
                size_bytes,
                limit_bytes,
                ..
            } => format!(
                "The media file size ({} MB) exceeds the maximum allowed import size ({} MB).",
                size_bytes / (1024 * 1024),
                limit_bytes / (1024 * 1024)
            ),
            Self::InvalidPath { reason, .. } => {
                format!("Invalid file selection: {reason}")
            }
            Self::CorruptMedia { details, .. } => {
                format!("The media file could not be read or is corrupted: {details}")
            }
            Self::NoStreamsFound { .. } => {
                "The media file contains no detectable video or audio streams.".to_string()
            }
            Self::DuplicateFile { .. } => "This file is already in your media library.".to_string(),
            Self::UnauthorizedPath { .. } => {
                "The selected file was not authorized via native file picker or drag-and-drop."
                    .to_string()
            }
            Self::AssetNotFound { .. } => "The requested media asset was not found.".to_string(),
            Self::AssetRevoked { .. } => {
                "Access to this media asset has been revoked or removed.".to_string()
            }
            Self::InvalidRange { details } => {
                format!("Invalid media seek/range request: {details}")
            }
            Self::RangeNotSatisfiable { .. } => {
                "Requested media range is beyond the end of the file.".to_string()
            }
            Self::ProxyGenerationFailed { reason, .. } => {
                format!("Preview proxy transcode failed: {reason}")
            }
            Self::ProxyCancelled { .. } => "Preview proxy generation was cancelled.".to_string(),
            Self::Internal(msg) => format!("Media engine error: {msg}"),
        }
    }
}
