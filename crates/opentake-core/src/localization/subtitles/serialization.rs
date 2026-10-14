use super::error::SubtitleError;
use super::model::{SubtitleFormat, SubtitleTrack};
use super::srt::{parse_srt, serialize_srt};
use super::webvtt::{parse_webvtt, serialize_webvtt};

/// Inspects the content of a subtitle text to detect whether it is WebVTT or SRT.
pub fn detect_subtitle_format(content: &str) -> SubtitleFormat {
    let sanitized = content.trim_start_matches('\u{feff}').trim_start();
    if sanitized.starts_with("WEBVTT") {
        SubtitleFormat::WebVtt
    } else {
        SubtitleFormat::Srt
    }
}

/// Parses subtitle content from a string according to the specified or auto-detected format.
pub fn parse_subtitles_from_str(
    content: &str,
    format: Option<SubtitleFormat>,
    track_id: Option<String>,
    language: Option<String>,
    display_name: Option<String>,
) -> Result<(SubtitleTrack, Vec<String>), SubtitleError> {
    let target_format = format.unwrap_or_else(|| detect_subtitle_format(content));

    match target_format {
        SubtitleFormat::Srt => {
            let res = parse_srt(content, track_id, language, display_name)?;
            Ok((res.track, res.warnings))
        }
        SubtitleFormat::WebVtt => {
            let res = parse_webvtt(content, track_id, language, display_name)?;
            Ok((res.track, res.warnings))
        }
        SubtitleFormat::NativeProject => {
            let track: SubtitleTrack =
                serde_json::from_str(content).map_err(|e| SubtitleError::DeserializationError {
                    message: format!("Failed to parse native subtitle track JSON: {e}"),
                })?;
            Ok((track, Vec::new()))
        }
    }
}

/// Exports a `SubtitleTrack` into the target string format.
pub fn export_subtitles_to_string(
    track: &SubtitleTrack,
    format: SubtitleFormat,
) -> Result<String, SubtitleError> {
    match format {
        SubtitleFormat::Srt => serialize_srt(track),
        SubtitleFormat::WebVtt => serialize_webvtt(track),
        SubtitleFormat::NativeProject => {
            serde_json::to_string_pretty(track).map_err(|e| SubtitleError::SerializationError {
                message: format!("Failed to serialize native subtitle track JSON: {e}"),
            })
        }
    }
}
