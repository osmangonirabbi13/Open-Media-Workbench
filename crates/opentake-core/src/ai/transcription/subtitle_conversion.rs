use uuid::Uuid;

use crate::localization::subtitles::model::{
    SubtitleCue, SubtitleDocument, SubtitleFormat, SubtitleTrack,
};
use crate::localization::subtitles::validation::{validate_track, FindingSeverity};

use super::errors::TranscriptionError;
use super::models::TranscriptionResult;

/// Converts a completed TranscriptionResult into a canonical, validated SubtitleTrack.
pub fn convert_transcription_result_to_track(
    result: &TranscriptionResult,
    custom_track_name: Option<&str>,
) -> Result<SubtitleTrack, TranscriptionError> {
    let track_id = Uuid::new_v4().to_string();
    let language = result
        .detected_language
        .as_deref()
        .unwrap_or(&result.requested_language)
        .to_string();

    let display_name = if let Some(custom) = custom_track_name {
        if !custom.trim().is_empty() {
            custom.trim().to_string()
        } else {
            format!("AI Subtitles ({language})")
        }
    } else {
        format!("AI Subtitles ({language})")
    };

    let mut cues = Vec::with_capacity(result.segments.len());

    for seg in &result.segments {
        let mut cue = SubtitleCue::new(seg.id.clone(), seg.start_us, seg.end_us, seg.text.clone())
            .map_err(|e| {
                TranscriptionError::SubtitleConversionFailed(format!(
                    "Failed to construct SubtitleCue for segment {}: {e}",
                    seg.id
                ))
            })?;

        if let Some(conf) = seg.confidence {
            cue.custom_metadata
                .insert("confidence".to_string(), format!("{conf:.3}"));
        }

        cues.push(cue);
    }

    let track = SubtitleTrack {
        id: track_id,
        language,
        display_name,
        source_format: SubtitleFormat::NativeProject,
        cues,
        is_enabled: true,
        is_visible: true,
        schema_version: 1,
    };

    // Strict validation against STEP 16 subtitle foundation
    let findings = validate_track(&track, None);
    let has_fatal_error = findings
        .iter()
        .any(|f| f.severity == FindingSeverity::Error);

    if has_fatal_error {
        let errors: Vec<String> = findings
            .into_iter()
            .filter(|f| f.severity == FindingSeverity::Error)
            .map(|f| f.message)
            .collect();
        return Err(TranscriptionError::SubtitleConversionFailed(format!(
            "Generated track failed validation: {}",
            errors.join("; ")
        )));
    }

    Ok(track)
}

/// Non-destructively adds a generated SubtitleTrack to an existing or new SubtitleDocument.
pub fn append_track_to_document(
    mut document: Option<SubtitleDocument>,
    track: SubtitleTrack,
) -> SubtitleDocument {
    if let Some(mut doc) = document.take() {
        // Non-destructive: append as a new track, keeping all existing tracks untouched
        doc.tracks.push(track);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        doc.updated_at_epoch_ms = now;
        doc
    } else {
        let mut doc = SubtitleDocument::new(None);
        doc.tracks.push(track);
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::transcription::models::TranscriptionSegment;
    use crate::editing::time::MediaTimeUs;

    #[test]
    fn test_convert_transcription_result_to_track_preserves_unicode() {
        let result = TranscriptionResult {
            schema_version: 1,
            job_id: "test-job-1".to_string(),
            asset_id: "asset-1".to_string(),
            model_id: "whisper-tiny".to_string(),
            runtime_version: "whisper.cpp v1.7.0".to_string(),
            requested_language: "bn".to_string(),
            detected_language: Some("bn".to_string()),
            segments: vec![
                TranscriptionSegment {
                    id: "seg-1".to_string(),
                    start_us: MediaTimeUs(0),
                    end_us: MediaTimeUs(2_000_000),
                    text: "ওপেন মিডিয়া ওয়ার্কবেঞ্চ".to_string(),
                    detected_language: Some("bn".to_string()),
                    confidence: Some(0.98),
                },
                TranscriptionSegment {
                    id: "seg-2".to_string(),
                    start_us: MediaTimeUs(2_500_000),
                    end_us: MediaTimeUs(4_500_000),
                    text: "অফলাইন ট্রান্সক্রিপশন সফল।".to_string(),
                    detected_language: Some("bn".to_string()),
                    confidence: Some(0.95),
                },
            ],
            processing_duration_ms: 1200,
            audio_duration_us: Some(5_000_000),
            warnings: vec![],
        };

        let track = convert_transcription_result_to_track(&result, None).unwrap();
        assert_eq!(track.language, "bn");
        assert_eq!(track.cues.len(), 2);
        assert_eq!(track.cues[0].text, "ওপেন মিডিয়া ওয়ার্কবেঞ্চ");
        assert_eq!(track.display_name, "AI Subtitles (bn)");

        let doc = append_track_to_document(None, track);
        assert_eq!(doc.tracks.len(), 1);
        assert_eq!(doc.tracks[0].cues.len(), 2);
    }
}
