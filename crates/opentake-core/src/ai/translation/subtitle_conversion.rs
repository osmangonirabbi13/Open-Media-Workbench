use crate::localization::subtitles::model::{
    SubtitleCue, SubtitleDocument, SubtitleFormat, SubtitleTrack,
};

use super::errors::TranslationError;
use super::models::TranslationResult;

/// Converts a completed TranslationResult into a canonical SubtitleTrack.
pub fn convert_translation_to_track(
    result: &TranslationResult,
    track_name: Option<&str>,
) -> Result<SubtitleTrack, TranslationError> {
    let fallback_name = format!("Translated ({})", result.target_language.to_uppercase());
    let display_name = track_name.unwrap_or(&fallback_name);
    let track_id = format!("track-tr-{}", uuid::Uuid::new_v4());

    let mut track = SubtitleTrack::new(
        &track_id,
        &result.target_language,
        display_name,
        SubtitleFormat::NativeProject,
    );

    let mut cues = Vec::with_capacity(result.translated_cues.len());
    for item in &result.translated_cues {
        // Enforce timing preservation invariant
        let start_us = item.target_start_us;
        let end_us = item.target_end_us;

        let mut cue = SubtitleCue::new(
            &item.translated_cue_id,
            start_us,
            end_us,
            &item.translated_text,
        )
        .map_err(|e| {
            TranslationError::OutputValidationFailed(format!("Invalid cue timing: {e}"))
        })?;

        // Word timing from source is NOT valid for translated text
        cue.words = None;
        cues.push(cue);
    }

    track.cues = cues;
    Ok(track)
}

/// Appends a translated subtitle track non-destructively to an existing SubtitleDocument.
pub fn append_translated_track_to_document(
    document: &mut SubtitleDocument,
    result: &TranslationResult,
    track_name: Option<&str>,
) -> Result<String, TranslationError> {
    let track = convert_translation_to_track(result, track_name)?;
    let track_id = track.id.clone();
    document.tracks.push(track);
    document.active_track_id = Some(track_id.clone());
    document.touch();
    Ok(track_id)
}
