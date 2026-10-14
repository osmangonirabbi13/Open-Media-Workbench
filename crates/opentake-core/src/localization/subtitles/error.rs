use thiserror::Error;

/// Errors arising during subtitle parsing, validation, serialization, and persistence.
#[derive(Debug, Error)]
pub enum SubtitleError {
    #[error("Empty subtitle content or file")]
    EmptyInput,

    #[error("File size {size_bytes} bytes exceeds maximum allowed limit {max_bytes} bytes")]
    ExceededMaxFileSize { size_bytes: usize, max_bytes: usize },

    #[error("Cue count {count} exceeds maximum allowed limit {max_count} cues")]
    ExceededMaxCueCount { count: usize, max_count: usize },

    #[error("Invalid UTF-8 byte sequence in subtitle stream: {source}")]
    InvalidUtf8 {
        #[from]
        source: std::str::Utf8Error,
    },

    #[error("Missing timing specification at line {line_number}")]
    MissingTimingLine { line_number: usize },

    #[error("Malformed timestamp '{raw}': {reason}")]
    MalformedTimestamp { raw: String, reason: String },

    #[error(
        "Invalid time interval: start ({start_us}µs) must be strictly less than end ({end_us}µs)"
    )]
    InvalidTimeRange { start_us: i64, end_us: i64 },

    #[error("Negative start time ({time_us}µs) is not permitted")]
    NegativeTime { time_us: i64 },

    #[error("Unsupported subtitle format: '{format_str}'")]
    UnsupportedFormat { format_str: String },

    #[error("WebVTT header missing: file must begin with 'WEBVTT'")]
    MissingWebVttHeader,

    #[error("Persistence I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Deserialization error: {message}")]
    DeserializationError { message: String },

    #[error("Unsupported schema version {version} (maximum supported version is {supported})")]
    UnsupportedSchemaVersion { version: u32, supported: u32 },

    #[error("Track not found with ID '{0}'")]
    TrackNotFound(String),
}
