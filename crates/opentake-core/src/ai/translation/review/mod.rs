pub mod errors;
pub mod glossary;
pub mod memory;
pub mod models;
pub mod operations;
pub mod persistence;
pub mod quality;
pub mod terminology;

pub use errors::TranslationReviewError;
pub use glossary::{Glossary, GlossaryEntry, GLOSSARY_SCHEMA_VERSION};
pub use memory::{TranslationMemory, TranslationMemoryEntry, TRANSLATION_MEMORY_SCHEMA_VERSION};
pub use models::{
    QualityFinding, QualityFindingCode, QualitySeverity, ReviewProgressStats, ReviewStatus,
    TranslationReviewDocument, TranslationReviewEntry, TRANSLATION_REVIEW_SCHEMA_VERSION,
};
pub use operations::{
    batch_approve_entries, batch_set_status, create_or_sync_review_document,
    edit_review_entry_target_text, set_review_entry_status,
};
pub use persistence::{
    embed_review_in_document, extract_review_from_document, ReviewStorageManager,
    REVIEW_METADATA_KEY,
};
pub use quality::evaluate_entry_quality;
pub use terminology::{text_contains_word, validate_terminology, ProtectedTermMasker};
