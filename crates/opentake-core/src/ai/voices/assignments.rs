//! Project-level and track-level voice assignments for SubtitleDocument.

use std::time::{SystemTime, UNIX_EPOCH};

use super::catalog::find_voice_by_id;
use super::compatibility::validate_speaker_id;
use super::errors::VoiceLibraryError;
use super::models::{ProjectVoiceAssignments, VoiceAssignment};
use crate::localization::subtitles::model::SubtitleDocument;

pub const VOICE_ASSIGNMENTS_METADATA_KEY: &str = "voiceAssignments";

/// Retrieves voice assignments embedded in the SubtitleDocument metadata.
pub fn get_project_voice_assignments(doc: &SubtitleDocument) -> ProjectVoiceAssignments {
    if let Some(json_str) = doc.metadata.get(VOICE_ASSIGNMENTS_METADATA_KEY) {
        if let Ok(assignments) = serde_json::from_str::<ProjectVoiceAssignments>(json_str) {
            return assignments;
        }
    }
    ProjectVoiceAssignments::default()
}

/// Sets or updates the project-level default voice ID.
pub fn set_project_default_voice(
    doc: &mut SubtitleDocument,
    voice_id: Option<String>,
) -> Result<(), VoiceLibraryError> {
    if let Some(ref id) = voice_id {
        if find_voice_by_id(id).is_none() {
            return Err(VoiceLibraryError::VoiceNotFound(id.clone()));
        }
    }

    let mut assignments = get_project_voice_assignments(doc);
    assignments.default_voice_id = voice_id;
    save_assignments_to_document(doc, &assignments)?;
    Ok(())
}

/// Assigns a voice to a specific subtitle track within the SubtitleDocument.
pub fn assign_voice_to_track(
    doc: &mut SubtitleDocument,
    track_id: &str,
    voice_id: &str,
    speaker_id: Option<u32>,
) -> Result<VoiceAssignment, VoiceLibraryError> {
    // 1. Verify track exists
    let track = doc
        .tracks
        .iter()
        .find(|t| t.id == track_id)
        .ok_or_else(|| VoiceLibraryError::VoiceNotFound(format!("Track '{track_id}' not found")))?;

    // 2. Verify voice exists in catalog
    let voice = find_voice_by_id(voice_id)
        .ok_or_else(|| VoiceLibraryError::VoiceNotFound(voice_id.to_string()))?;

    // 3. Validate speaker ID
    let validated_speaker = validate_speaker_id(&voice, speaker_id)?;

    let assignment = VoiceAssignment {
        track_id: track_id.to_string(),
        voice_id: voice_id.to_string(),
        language: track.language.clone(),
        speaker_id: if voice.speaker_count > 1 {
            Some(validated_speaker)
        } else {
            None
        },
        assigned_at_epoch_ms: current_epoch_ms(),
    };

    let mut assignments = get_project_voice_assignments(doc);
    // Replace existing assignment for track or append
    assignments
        .track_assignments
        .retain(|a| a.track_id != track_id);
    assignments.track_assignments.push(assignment.clone());

    save_assignments_to_document(doc, &assignments)?;
    Ok(assignment)
}

/// Removes voice assignment for a specific track.
pub fn remove_track_voice_assignment(
    doc: &mut SubtitleDocument,
    track_id: &str,
) -> Result<bool, VoiceLibraryError> {
    let mut assignments = get_project_voice_assignments(doc);
    let original_len = assignments.track_assignments.len();
    assignments
        .track_assignments
        .retain(|a| a.track_id != track_id);
    let removed = assignments.track_assignments.len() < original_len;
    if removed {
        save_assignments_to_document(doc, &assignments)?;
    }
    Ok(removed)
}

fn save_assignments_to_document(
    doc: &mut SubtitleDocument,
    assignments: &ProjectVoiceAssignments,
) -> Result<(), VoiceLibraryError> {
    let json_str = serde_json::to_string(assignments)?;
    doc.metadata
        .insert(VOICE_ASSIGNMENTS_METADATA_KEY.to_string(), json_str);
    doc.updated_at_epoch_ms = current_epoch_ms();
    Ok(())
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
