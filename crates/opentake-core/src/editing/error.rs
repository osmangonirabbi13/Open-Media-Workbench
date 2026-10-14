use thiserror::Error;

/// Domain error variants for non-destructive timeline editing, EDL manipulation, and timebase math.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EditError {
    #[error("Invalid source duration: {duration_us} us (duration must be strictly positive)")]
    InvalidSourceDuration { duration_us: i64 },

    #[error("Invalid interval: start {start_us} us must be strictly less than end {end_us} us")]
    InvalidInterval { start_us: i64, end_us: i64 },

    #[error("Negative time is forbidden in source coordinates: {time_us} us")]
    NegativeTime { time_us: i64 },

    #[error("Zero-length interval is prohibited: start and end are both {time_us} us")]
    ZeroLengthRange { time_us: i64 },

    #[error("Interval [{start_us}, {end_us}) exceeds source bounds [0, {source_duration_us})")]
    SourceOutOfBounds {
        start_us: i64,
        end_us: i64,
        source_duration_us: i64,
    },

    #[error("Timeline position or range [{start_us}, {end_us}) exceeds edited timeline duration {timeline_duration_us} us")]
    TimelineOutOfBounds {
        start_us: i64,
        end_us: i64,
        timeline_duration_us: i64,
    },

    #[error("Source mismatch: expected source '{expected}', but encountered '{actual}'")]
    SourceMismatch { expected: String, actual: String },

    #[error("Non-canonical segments: {reason}")]
    NonCanonicalSegments { reason: String },

    #[error("Arithmetic overflow during time calculation")]
    ArithmeticOverflow,

    #[error("Timeline is empty: operation requires at least one retained segment")]
    EmptyTimeline,

    #[error("Unsupported edit operation: {0}")]
    UnsupportedOperation(String),

    #[error("Invalid EDL schema version: {found} (supported: {supported})")]
    InvalidSchemaVersion { found: u32, supported: u32 },
}

impl EditError {
    /// Returns a stable programmatic error code for IPC and diagnostics.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidSourceDuration { .. } => "INVALID_SOURCE_DURATION",
            Self::InvalidInterval { .. } => "INVALID_INTERVAL",
            Self::NegativeTime { .. } => "NEGATIVE_TIME",
            Self::ZeroLengthRange { .. } => "ZERO_LENGTH_RANGE",
            Self::SourceOutOfBounds { .. } => "SOURCE_OUT_OF_BOUNDS",
            Self::TimelineOutOfBounds { .. } => "TIMELINE_OUT_OF_BOUNDS",
            Self::SourceMismatch { .. } => "SOURCE_MISMATCH",
            Self::NonCanonicalSegments { .. } => "NON_CANONICAL_SEGMENTS",
            Self::ArithmeticOverflow => "ARITHMETIC_OVERFLOW",
            Self::EmptyTimeline => "EMPTY_TIMELINE",
            Self::UnsupportedOperation(_) => "UNSUPPORTED_OPERATION",
            Self::InvalidSchemaVersion { .. } => "INVALID_SCHEMA_VERSION",
        }
    }
}
