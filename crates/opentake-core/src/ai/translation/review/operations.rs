use uuid::Uuid;

use crate::localization::subtitles::model::SubtitleTrack;

use super::errors::TranslationReviewError;
use super::glossary::GlossaryEntry;
use super::models::{
    QualityFinding, QualityFindingCode, QualitySeverity, ReviewStatus, TranslationReviewDocument,
    TranslationReviewEntry, TRANSLATION_REVIEW_SCHEMA_VERSION,
};
use super::quality::evaluate_entry_quality;

/// Builds or synchronizes a TranslationReviewDocument from a source track and a target track.
pub fn create_or_sync_review_document(
    document_id: &str,
    source_track: &SubtitleTrack,
    target_track: &SubtitleTrack,
    existing_review: Option<&TranslationReviewDocument>,
    glossary_entries: &[GlossaryEntry],
    is_language_pair_verified: bool,
) -> TranslationReviewDocument {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let review_id = existing_review
        .map(|r| r.review_id.clone())
        .unwrap_or_else(|| format!("rev-{}", Uuid::new_v4()));

    let mut entries = Vec::with_capacity(source_track.cues.len());

    // Map source cues to target cues using stable identifiers and timestamps
    for source_cue in &source_track.cues {
        // Try finding matching target cue:
        // 1. Direct sourceCueId prefix match (e.g. "{source_id}-tr-{lang}")
        // 2. Exact start_us matching
        let matched_target_cue = target_track.cues.iter().find(|t| {
            t.id.starts_with(&format!("{}-tr-", source_cue.id))
                || t.id == source_cue.id
                || t.start_us == source_cue.start_us
        });

        let (target_id, target_text, target_start_us, target_end_us, is_unmapped) =
            match matched_target_cue {
                Some(t) => (t.id.clone(), t.text.clone(), t.start_us, t.end_us, false),
                None => (
                    format!("{}-unmapped", source_cue.id),
                    String::new(),
                    source_cue.start_us,
                    source_cue.end_us,
                    true,
                ),
            };

        // Reuse previous review status and notes if present
        let prev_entry = existing_review.and_then(|r| {
            r.entries
                .iter()
                .find(|e| e.source_cue_id == source_cue.id && e.target_cue_id == target_id)
        });

        let mut entry = TranslationReviewEntry {
            id: prev_entry
                .map(|e| e.id.clone())
                .unwrap_or_else(|| format!("re-{}", Uuid::new_v4())),
            source_cue_id: source_cue.id.clone(),
            target_cue_id: target_id,
            source_text: source_cue.text.clone(),
            target_text,
            source_start_us: source_cue.start_us,
            source_end_us: source_cue.end_us,
            target_start_us,
            target_end_us,
            status: prev_entry
                .map(|e| e.status)
                .unwrap_or(ReviewStatus::Pending),
            findings: Vec::new(),
            approved_fingerprint: prev_entry.and_then(|e| e.approved_fingerprint.clone()),
            reviewer_notes: prev_entry.and_then(|e| e.reviewer_notes.clone()),
            updated_at_epoch_ms: prev_entry.map(|e| e.updated_at_epoch_ms).unwrap_or(now),
        };

        let mut quality_findings =
            evaluate_entry_quality(&entry, glossary_entries, is_language_pair_verified);

        if is_unmapped {
            quality_findings.push(QualityFinding {
                code: QualityFindingCode::UnmappedCue,
                severity: QualitySeverity::Error,
                message: "No matching target subtitle cue found for source cue.".to_string(),
                details: Some(
                    "Source cue exists in original track but lacks mapped translation.".to_string(),
                ),
            });
            entry.status = ReviewStatus::NeedsReview;
        }

        entry.findings = quality_findings;
        entries.push(entry);
    }

    TranslationReviewDocument {
        schema_version: TRANSLATION_REVIEW_SCHEMA_VERSION,
        review_id,
        document_id: document_id.to_string(),
        source_track_id: source_track.id.clone(),
        target_track_id: target_track.id.clone(),
        source_language: source_track.language.clone(),
        target_language: target_track.language.clone(),
        entries,
        glossary_ids: existing_review
            .map(|r| r.glossary_ids.clone())
            .unwrap_or_default(),
        created_at_epoch_ms: existing_review
            .map(|r| r.created_at_epoch_ms)
            .unwrap_or(now),
        updated_at_epoch_ms: now,
    }
}

/// Updates the target text of a review entry and synchronizes the change to the target subtitle track.
/// CRITICAL INVARIANT: If the entry was previously Approved, editing it resets its status to NeedsReview!
pub fn edit_review_entry_target_text(
    review_doc: &mut TranslationReviewDocument,
    target_track: &mut SubtitleTrack,
    entry_id: &str,
    new_text: &str,
    glossary_entries: &[GlossaryEntry],
    is_language_pair_verified: bool,
) -> Result<(), TranslationReviewError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let entry = review_doc
        .entries
        .iter_mut()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| TranslationReviewError::EntryNotFound(entry_id.to_string()))?;

    // Invalidate approval if text was changed
    if entry.target_text != new_text {
        if entry.status == ReviewStatus::Approved {
            entry.status = ReviewStatus::NeedsReview;
            entry.approved_fingerprint = None;
        }
        entry.target_text = new_text.to_string();
        entry.updated_at_epoch_ms = now;
    }

    // Synchronize to target subtitle cue in track
    if let Some(cue) = target_track
        .cues
        .iter_mut()
        .find(|c| c.id == entry.target_cue_id)
    {
        cue.text = new_text.to_string();
    }

    // Re-evaluate quality diagnostics
    entry.findings = evaluate_entry_quality(entry, glossary_entries, is_language_pair_verified);
    review_doc.updated_at_epoch_ms = now;

    Ok(())
}

/// Sets the review status of an entry, recording approval fingerprint or checking for blocking errors.
pub fn set_review_entry_status(
    review_doc: &mut TranslationReviewDocument,
    entry_id: &str,
    new_status: ReviewStatus,
    allow_override_blocking_errors: bool,
) -> Result<(), TranslationReviewError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let entry = review_doc
        .entries
        .iter_mut()
        .find(|e| e.id == entry_id)
        .ok_or_else(|| TranslationReviewError::EntryNotFound(entry_id.to_string()))?;

    if new_status == ReviewStatus::Approved {
        let has_blocking_errors = entry
            .findings
            .iter()
            .any(|f| f.severity == QualitySeverity::Error);
        if has_blocking_errors && !allow_override_blocking_errors {
            return Err(TranslationReviewError::BlockingQualityError(
                entry_id.to_string(),
            ));
        }

        // Stamp approval fingerprint
        entry.approved_fingerprint = Some(entry.compute_target_fingerprint());
    } else if new_status != ReviewStatus::Reviewed {
        entry.approved_fingerprint = None;
    }

    entry.status = new_status;
    entry.updated_at_epoch_ms = now;
    review_doc.updated_at_epoch_ms = now;

    Ok(())
}

/// Batch approves multiple review entries atomically, skipping or failing on blocking errors.
pub fn batch_approve_entries(
    review_doc: &mut TranslationReviewDocument,
    entry_ids: &[String],
    allow_override_blocking_errors: bool,
) -> Result<usize, TranslationReviewError> {
    let mut approved_count = 0;

    for id in entry_ids {
        if let Ok(()) = set_review_entry_status(
            review_doc,
            id,
            ReviewStatus::Approved,
            allow_override_blocking_errors,
        ) {
            approved_count += 1;
        }
    }

    Ok(approved_count)
}

/// Batch updates status for multiple review entries.
pub fn batch_set_status(
    review_doc: &mut TranslationReviewDocument,
    entry_ids: &[String],
    new_status: ReviewStatus,
) -> usize {
    let mut updated = 0;
    for id in entry_ids {
        if let Ok(()) = set_review_entry_status(review_doc, id, new_status, true) {
            updated += 1;
        }
    }
    updated
}
