//! Tauri IPC commands for Word-level Timestamp Alignment and Smart Subtitle Segmentation.

use tauri::State;
use uuid::Uuid;

use super::transcription::TranscriptionState;
use opentake_core::ai::alignment::models::{
    AlignedWord, AlignmentResult, SegmentationPolicy, TimingQuality,
};
use opentake_core::ai::alignment::refinement::{refine_cues_with_words, update_word_timing_in_cue};
use opentake_core::ai::alignment::segmentation::segment_words_into_cues as core_segment_words;
use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::model::{SubtitleCue, SubtitleDocument, SubtitleTrack};

/// Retrieves the word alignment result from a completed transcription job.
#[tauri::command]
pub async fn get_word_alignment_result(
    job_id: String,
    state: State<'_, TranscriptionState>,
) -> Result<AlignmentResult, String> {
    let result = state
        .manager
        .get_job_result(&job_id)
        .await
        .map_err(|e| e.to_string())?;

    let words = result.words.unwrap_or_default();
    let timing_quality = result.timing_quality.unwrap_or(if words.is_empty() {
        TimingQuality::Unavailable
    } else {
        TimingQuality::Estimated
    });

    Ok(AlignmentResult {
        schema_version: 1,
        source_asset_id: result.asset_id,
        job_id: result.job_id,
        model_id: result.model_id,
        language: result
            .detected_language
            .unwrap_or(result.requested_language),
        timing_quality,
        words,
        warnings: result.warnings,
    })
}

/// Segments a list of aligned words into formatted subtitle cues using configurable rules.
#[tauri::command]
pub fn segment_words_to_cues(
    words: Vec<AlignedWord>,
    policy: SegmentationPolicy,
    language: String,
) -> Result<Vec<SubtitleCue>, String> {
    let (cues, _warnings) = core_segment_words(&words, &policy, &language);
    Ok(cues)
}

/// Refines cue boundaries based on contained word timestamps and duration policies.
#[tauri::command]
pub fn refine_subtitle_cues(
    mut cues: Vec<SubtitleCue>,
    min_duration_us: i64,
) -> Result<Vec<SubtitleCue>, String> {
    refine_cues_with_words(&mut cues, min_duration_us);
    Ok(cues)
}

/// Updates the start and end timing of an individual word within a subtitle cue.
#[tauri::command]
pub fn update_word_timing_in_subtitle_cue(
    mut cue: SubtitleCue,
    word_id: String,
    start_us: i64,
    end_us: i64,
) -> Result<SubtitleCue, String> {
    let start = MediaTimeUs(start_us.max(0));
    let end = MediaTimeUs(end_us.max(start_us));
    update_word_timing_in_cue(&mut cue, &word_id, start, end).map_err(|e| e.to_string())?;
    Ok(cue)
}

/// Applies smart segmentation to a completed transcription job and non-destructively appends the track to the document.
#[tauri::command]
pub async fn apply_smart_segmented_subtitles(
    job_id: String,
    policy: SegmentationPolicy,
    custom_track_name: Option<String>,
    document: Option<SubtitleDocument>,
    state: State<'_, TranscriptionState>,
) -> Result<SubtitleDocument, String> {
    let result = state
        .manager
        .get_job_result(&job_id)
        .await
        .map_err(|e| e.to_string())?;

    let words = result.words.unwrap_or_default();
    if words.is_empty() {
        return Err("No word timing data available in transcription result.".to_string());
    }

    let lang = result
        .detected_language
        .unwrap_or(result.requested_language);
    let (mut cues, _) = core_segment_words(&words, &policy, &lang);

    // Refine boundaries
    refine_cues_with_words(&mut cues, policy.min_cue_duration_us);

    let track_id = format!("track-ai-smart-{}", Uuid::new_v4().as_simple());
    let display_name = custom_track_name.unwrap_or_else(|| format!("Smart AI Subtitles ({lang})"));

    let mut track = SubtitleTrack::new(
        track_id,
        lang,
        display_name,
        opentake_core::localization::subtitles::model::SubtitleFormat::NativeProject,
    );
    track.cues = cues;

    Ok(
        opentake_core::ai::transcription::subtitle_conversion::append_track_to_document(
            document, track,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentake_core::ai::alignment::models::WordTimingProvenance;

    #[test]
    fn test_segment_words_to_cues_ipc_handler() {
        let words = vec![
            AlignedWord::new(
                "w1",
                "Hello",
                MediaTimeUs(0),
                MediaTimeUs(500_000),
                WordTimingProvenance::WhisperNativeEstimate,
            )
            .unwrap(),
            AlignedWord::new(
                "w2",
                "world",
                MediaTimeUs(600_000),
                MediaTimeUs(1_200_000),
                WordTimingProvenance::WhisperNativeEstimate,
            )
            .unwrap(),
        ];

        let policy = SegmentationPolicy::default();
        let cues = segment_words_to_cues(words, policy, "en".to_string()).expect("segment");
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Hello world");
        assert_eq!(cues[0].words.as_ref().unwrap().len(), 2);
    }
}
