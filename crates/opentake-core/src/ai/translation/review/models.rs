use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::editing::time::MediaTimeUs;

pub const TRANSLATION_REVIEW_SCHEMA_VERSION: u32 = 1;

/// Human review status for a single translated subtitle cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ReviewStatus {
    #[default]
    Pending,
    NeedsReview,
    Reviewed,
    Approved,
    Rejected,
}

/// Severity classification for quality findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QualitySeverity {
    Info,
    Warning,
    Error,
}

/// Standardized quality finding codes for bilingual subtitle validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QualityFindingCode {
    MissingTranslation,
    EmptyTargetText,
    DuplicateTargetCue,
    SourceTargetTimingMismatch,
    UnmappedCue,
    PossibleUntranslatedText,
    ProtectedTermChanged,
    PreferredTermMissing,
    ExcessiveReadingSpeed,
    ExcessiveLineLength,
    LanguagePairUnverified,
    StaleReviewApproval,
    MissingGlossaryReference,
}

impl QualityFindingCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingTranslation => "MISSING_TRANSLATION",
            Self::EmptyTargetText => "EMPTY_TARGET_TEXT",
            Self::DuplicateTargetCue => "DUPLICATE_TARGET_CUE",
            Self::SourceTargetTimingMismatch => "SOURCE_TARGET_TIMING_MISMATCH",
            Self::UnmappedCue => "UNMAPPED_CUE",
            Self::PossibleUntranslatedText => "POSSIBLE_UNTRANSLATED_TEXT",
            Self::ProtectedTermChanged => "PROTECTED_TERM_CHANGED",
            Self::PreferredTermMissing => "PREFERRED_TERM_MISSING",
            Self::ExcessiveReadingSpeed => "EXCESSIVE_READING_SPEED",
            Self::ExcessiveLineLength => "EXCESSIVE_LINE_LENGTH",
            Self::LanguagePairUnverified => "LANGUAGE_PAIR_UNVERIFIED",
            Self::StaleReviewApproval => "STALE_REVIEW_APPROVAL",
            Self::MissingGlossaryReference => "MISSING_GLOSSARY_REFERENCE",
        }
    }
}

/// Structured diagnostic quality finding on a subtitle review entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityFinding {
    pub code: QualityFindingCode,
    pub severity: QualitySeverity,
    pub message: String,
    pub details: Option<String>,
}

/// Individual bilingual review entry matching a source cue to a target cue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationReviewEntry {
    pub id: String,
    pub source_cue_id: String,
    pub target_cue_id: String,
    pub source_text: String,
    pub target_text: String,
    pub source_start_us: MediaTimeUs,
    pub source_end_us: MediaTimeUs,
    pub target_start_us: MediaTimeUs,
    pub target_end_us: MediaTimeUs,
    pub status: ReviewStatus,
    pub findings: Vec<QualityFinding>,
    /// Cryptographic fingerprint of target text and timing at time of approval.
    pub approved_fingerprint: Option<String>,
    pub reviewer_notes: Option<String>,
    pub updated_at_epoch_ms: u64,
}

impl TranslationReviewEntry {
    /// Generates a revision fingerprint for the current target text and timestamps.
    pub fn compute_target_fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.target_text.as_bytes());
        hasher.update(self.target_start_us.0.to_le_bytes());
        hasher.update(self.target_end_us.0.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Checks if a previously approved entry has become stale due to edits.
    pub fn is_approval_stale(&self) -> bool {
        if self.status != ReviewStatus::Approved {
            return false;
        }
        match &self.approved_fingerprint {
            Some(saved_fp) => saved_fp != &self.compute_target_fingerprint(),
            None => true,
        }
    }
}

/// Complete bilingual review document tracking original vs translated tracks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationReviewDocument {
    pub schema_version: u32,
    pub review_id: String,
    pub document_id: String,
    pub source_track_id: String,
    pub target_track_id: String,
    pub source_language: String,
    pub target_language: String,
    pub entries: Vec<TranslationReviewEntry>,
    pub glossary_ids: Vec<String>,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}

/// Grounded statistics and metrics on bilingual review progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReviewProgressStats {
    pub total_cues: usize,
    pub pending_count: usize,
    pub needs_review_count: usize,
    pub reviewed_count: usize,
    pub approved_count: usize,
    pub rejected_count: usize,
    pub with_warnings_count: usize,
    pub with_errors_count: usize,
    pub percent_approved: f32,
    pub percent_reviewed_or_approved: f32,
}

impl TranslationReviewDocument {
    pub fn compute_stats(&self) -> ReviewProgressStats {
        let total = self.entries.len();
        if total == 0 {
            return ReviewProgressStats::default();
        }

        let mut pending = 0;
        let mut needs_review = 0;
        let mut reviewed = 0;
        let mut approved = 0;
        let mut rejected = 0;
        let mut with_warnings = 0;
        let mut with_errors = 0;

        for entry in &self.entries {
            match entry.status {
                ReviewStatus::Pending => pending += 1,
                ReviewStatus::NeedsReview => needs_review += 1,
                ReviewStatus::Reviewed => reviewed += 1,
                ReviewStatus::Approved => approved += 1,
                ReviewStatus::Rejected => rejected += 1,
            }

            let has_error = entry
                .findings
                .iter()
                .any(|f| f.severity == QualitySeverity::Error);
            let has_warning = entry
                .findings
                .iter()
                .any(|f| f.severity == QualitySeverity::Warning);

            if has_error {
                with_errors += 1;
            }
            if has_warning {
                with_warnings += 1;
            }
        }

        let percent_approved = (approved as f32 / total as f32) * 100.0;
        let percent_reviewed_or_approved = ((approved + reviewed) as f32 / total as f32) * 100.0;

        ReviewProgressStats {
            total_cues: total,
            pending_count: pending,
            needs_review_count: needs_review,
            reviewed_count: reviewed,
            approved_count: approved,
            rejected_count: rejected,
            with_warnings_count: with_warnings,
            with_errors_count: with_errors,
            percent_approved,
            percent_reviewed_or_approved,
        }
    }
}
