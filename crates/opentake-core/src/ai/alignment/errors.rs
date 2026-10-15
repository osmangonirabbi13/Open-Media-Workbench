//! Error types for the word alignment and smart segmentation domain.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AlignmentError {
    #[error("No speech tokens or words found in transcription")]
    NoWordsFound,

    #[error("Invalid word timestamp for '{word}': start {start_us}µs >= end {end_us}µs")]
    InvalidWordTimestamp {
        word: String,
        start_us: i64,
        end_us: i64,
    },

    #[error("Negative timestamp encountered: {time_us}µs")]
    NegativeTime { time_us: i64 },

    #[error("Word '{word}' is out of chronological order (starts at {current_start_us}µs, before previous start {previous_start_us}µs)")]
    OutOfOrder {
        word: String,
        current_start_us: i64,
        previous_start_us: i64,
    },

    #[error("Segmentation policy violation: {0}")]
    PolicyViolation(String),

    #[error("Failed to parse runtime alignment output: {0}")]
    ParserError(String),

    #[error("Word '{word_id}' not found in active alignment")]
    WordNotFound { word_id: String },

    #[error("Alignment operation rejected: {0}")]
    OperationRejected(String),
}
