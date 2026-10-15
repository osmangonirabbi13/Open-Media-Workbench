use thiserror::Error;

/// Strongly typed errors for bilingual review and terminology management.
#[derive(Debug, Error)]
pub enum TranslationReviewError {
    #[error("Review document not found for id: {0}")]
    ReviewDocumentNotFound(String),

    #[error("Track '{0}' not found in subtitle document")]
    TrackNotFound(String),

    #[error("Review entry '{0}' not found")]
    EntryNotFound(String),

    #[error("Glossary '{0}' not found")]
    GlossaryNotFound(String),

    #[error("Glossary entry '{0}' not found in glossary '{1}'")]
    GlossaryEntryNotFound(String, String),

    #[error("Conflicting glossary term: '{term}' already exists with target '{existing}' but conflicting with '{new_target}'")]
    ConflictingGlossaryTerm {
        term: String,
        existing: String,
        new_target: String,
    },

    #[error("Protected term '{term}' was altered or missing in translation (found '{actual}')")]
    ProtectedTermViolation { term: String, actual: String },

    #[error("Placeholder restoration failed: {0}")]
    PlaceholderRestorationFailed(String),

    #[error("Invalid review state transition from {from:?} to {to:?}: {reason}")]
    InvalidStateTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Blocking quality errors present in entry '{0}'; cannot approve without resolution")]
    BlockingQualityError(String),

    #[error("IO error in translation review storage: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}
