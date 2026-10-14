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
                "Permission denied while attempting to execute media binary.".to_string()
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
            Self::Internal(msg) => format!("Media engine error: {msg}"),
        }
    }
}
