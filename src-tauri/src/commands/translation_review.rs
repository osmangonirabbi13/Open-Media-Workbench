//! Tauri IPC commands for Bilingual Subtitle Review, Glossary Management,
//! Terminology Protection, and Translation Memory.

use std::sync::Arc;
use tauri::State;

use opentake_core::ai::translation::review::{
    batch_approve_entries, batch_set_status, create_or_sync_review_document,
    edit_review_entry_target_text, embed_review_in_document, extract_review_from_document,
    set_review_entry_status, Glossary, ReviewStatus, ReviewStorageManager, TranslationMemory,
    TranslationMemoryEntry, TranslationReviewDocument,
};
use opentake_core::localization::subtitles::model::SubtitleDocument;

/// Managed state holding the ReviewStorageManager instance.
#[derive(Clone)]
pub struct TranslationReviewState {
    pub storage: Arc<ReviewStorageManager>,
}

impl TranslationReviewState {
    pub fn new(storage: ReviewStorageManager) -> Self {
        Self {
            storage: Arc::new(storage),
        }
    }
}

/// Retrieves or creates and syncs a TranslationReviewDocument for the specified track pair.
#[tauri::command]
pub async fn get_or_create_translation_review(
    document: SubtitleDocument,
    source_track_id: String,
    target_track_id: String,
    glossary_id: Option<String>,
    state: State<'_, TranslationReviewState>,
) -> Result<TranslationReviewDocument, String> {
    let source_track = document
        .tracks
        .iter()
        .find(|t| t.id == source_track_id)
        .ok_or_else(|| format!("Source track '{source_track_id}' not found in document."))?;

    let target_track = document
        .tracks
        .iter()
        .find(|t| t.id == target_track_id)
        .ok_or_else(|| format!("Target track '{target_track_id}' not found in document."))?;

    // Check if an existing review document is embedded
    let existing_review = extract_review_from_document(&document);

    // Load glossary entries if provided
    let glossary_entries = if let Some(ref gid) = glossary_id {
        state
            .storage
            .load_glossary(gid)
            .map(|g| g.entries)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let review = create_or_sync_review_document(
        &document.id,
        source_track,
        target_track,
        existing_review.as_ref(),
        &glossary_entries,
        true, // Language pair verified
    );

    Ok(review)
}

/// Saves the review document into the SubtitleDocument metadata under 'translationReview'.
#[tauri::command]
pub async fn save_translation_review_to_document(
    mut document: SubtitleDocument,
    review: TranslationReviewDocument,
) -> Result<SubtitleDocument, String> {
    embed_review_in_document(&mut document, &review).map_err(|e| e.to_string())?;
    Ok(document)
}

/// Updates the target text of a review entry, updating both the review document and
/// the corresponding SubtitleCue in the SubtitleDocument. If the cue was Approved,
/// its status is invalidated to NeedsReview.
#[tauri::command]
pub async fn update_review_entry_target_text(
    mut document: SubtitleDocument,
    mut review: TranslationReviewDocument,
    target_track_id: String,
    entry_id: String,
    new_text: String,
    glossary_id: Option<String>,
    state: State<'_, TranslationReviewState>,
) -> Result<(SubtitleDocument, TranslationReviewDocument), String> {
    // 1. Get mutable reference to target track
    let target_track = document
        .tracks
        .iter_mut()
        .find(|t| t.id == target_track_id)
        .ok_or_else(|| format!("Target track '{target_track_id}' not found in document."))?;

    // 2. Load glossary entries if provided
    let glossary_entries = if let Some(ref gid) = glossary_id {
        state
            .storage
            .load_glossary(gid)
            .map(|g| g.entries)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // 3. Edit review entry (synchronizes to target_track and invalidates approval if needed)
    edit_review_entry_target_text(
        &mut review,
        target_track,
        &entry_id,
        &new_text,
        &glossary_entries,
        true,
    )
    .map_err(|e| e.to_string())?;

    // 4. Re-embed updated review document into project metadata
    embed_review_in_document(&mut document, &review).map_err(|e| e.to_string())?;

    Ok((document, review))
}

/// Sets the review status of an individual review entry.
#[tauri::command]
pub async fn set_review_entry_status_command(
    mut review: TranslationReviewDocument,
    entry_id: String,
    status: ReviewStatus,
    allow_override: bool,
) -> Result<TranslationReviewDocument, String> {
    set_review_entry_status(&mut review, &entry_id, status, allow_override)
        .map_err(|e| e.to_string())?;
    Ok(review)
}

/// Batch updates the status of multiple review entries.
#[tauri::command]
pub async fn batch_set_review_status_command(
    mut review: TranslationReviewDocument,
    entry_ids: Vec<String>,
    status: ReviewStatus,
    allow_override: bool,
) -> Result<TranslationReviewDocument, String> {
    if status == ReviewStatus::Approved {
        batch_approve_entries(&mut review, &entry_ids, allow_override)
            .map_err(|e| e.to_string())?;
    } else {
        batch_set_status(&mut review, &entry_ids, status);
    }
    Ok(review)
}

// =========================================================================
// GLOSSARY MANAGEMENT IPC COMMANDS
// =========================================================================

/// Lists all saved glossaries.
#[tauri::command]
pub async fn list_glossaries_command(
    state: State<'_, TranslationReviewState>,
) -> Result<Vec<Glossary>, String> {
    state.storage.list_glossaries().map_err(|e| e.to_string())
}

/// Saves or updates a glossary.
#[tauri::command]
pub async fn save_glossary_command(
    glossary: Glossary,
    state: State<'_, TranslationReviewState>,
) -> Result<(), String> {
    state
        .storage
        .save_glossary(&glossary)
        .map_err(|e| e.to_string())
}

/// Deletes a glossary by its ID.
#[tauri::command]
pub async fn delete_glossary_command(
    glossary_id: String,
    state: State<'_, TranslationReviewState>,
) -> Result<(), String> {
    state
        .storage
        .delete_glossary(&glossary_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// =========================================================================
// TRANSLATION MEMORY IPC COMMANDS
// =========================================================================

/// Retrieves the entire translation memory.
#[tauri::command]
pub async fn get_translation_memory_command(
    state: State<'_, TranslationReviewState>,
) -> Result<TranslationMemory, String> {
    Ok(state.storage.get_memory())
}

/// Records a human-approved translation into translation memory.
#[tauri::command]
pub async fn record_approved_translation_command(
    source_text: String,
    target_text: String,
    source_language: String,
    target_language: String,
    domain: Option<String>,
    state: State<'_, TranslationReviewState>,
) -> Result<TranslationMemoryEntry, String> {
    state
        .storage
        .update_memory(|mem| {
            mem.record_approved(
                &source_text,
                &target_text,
                &source_language,
                &target_language,
                domain.as_deref(),
            )
        })
        .map_err(|e| e.to_string())
}

/// Looks up an exact-match translation memory suggestion for the given source text.
#[tauri::command]
pub async fn lookup_translation_memory_command(
    source_text: String,
    source_language: String,
    target_language: String,
    state: State<'_, TranslationReviewState>,
) -> Result<Option<TranslationMemoryEntry>, String> {
    let memory = state.storage.get_memory();
    Ok(memory
        .find_exact_match(&source_text, &source_language, &target_language)
        .cloned())
}

/// Deletes an entry from translation memory by ID.
#[tauri::command]
pub async fn delete_translation_memory_entry_command(
    entry_id: String,
    state: State<'_, TranslationReviewState>,
) -> Result<(), String> {
    state
        .storage
        .update_memory(|mem| {
            mem.delete_entry(&entry_id);
        })
        .map_err(|e| e.to_string())
}

/// Clears the entire translation memory.
#[tauri::command]
pub async fn clear_translation_memory_command(
    state: State<'_, TranslationReviewState>,
) -> Result<(), String> {
    state
        .storage
        .update_memory(|mem| {
            mem.clear();
        })
        .map_err(|e| e.to_string())
}
