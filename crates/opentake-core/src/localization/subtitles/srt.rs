use uuid::Uuid;

use super::error::SubtitleError;
use super::model::{
    SubtitleCue, SubtitleFormat, SubtitleTrack, MAX_SUBTITLE_CUE_COUNT, MAX_SUBTITLE_FILE_SIZE,
};
use crate::editing::time::MediaTimeUs;

/// Result of parsing an SRT file including parsed track and any non-fatal parsing warnings.
#[derive(Debug, Clone)]
pub struct SrtParseResult {
    pub track: SubtitleTrack,
    pub warnings: Vec<String>,
}

/// Parses raw SubRip (.srt) text content into a canonical `SubtitleTrack`.
///
/// # Tolerant Features
/// - Supports UTF-8 with or without Byte Order Mark (BOM: `\u{feff}`).
/// - Normalizes Windows CRLF (`\r\n`), classic Mac CR (`\r`), and Unix LF (`\n`).
/// - Tolerates missing or non-sequential cue index numbers.
/// - Handles both comma (standard `,`) and dot (`.`) millisecond delimiters.
/// - Strips trailing video coordinate metadata (e.g. `X1:0 Y1:0`).
/// - Recovers safely across multiple consecutive empty blank lines.
///
/// # Strict Safety Guards
/// - Rejects inputs exceeding `MAX_SUBTITLE_FILE_SIZE` (10 MB).
/// - Enforces `MAX_SUBTITLE_CUE_COUNT` (20,000 cues).
/// - Rejects invalid or negative timestamp intervals where $end \le start$.
pub fn parse_srt(
    content: &str,
    track_id: Option<String>,
    language: Option<String>,
    display_name: Option<String>,
) -> Result<SrtParseResult, SubtitleError> {
    if content.len() > MAX_SUBTITLE_FILE_SIZE {
        return Err(SubtitleError::ExceededMaxFileSize {
            size_bytes: content.len(),
            max_bytes: MAX_SUBTITLE_FILE_SIZE,
        });
    }

    // Strip UTF-8 BOM if present
    let sanitized = content.trim_start_matches('\u{feff}');
    if sanitized.trim().is_empty() {
        return Err(SubtitleError::EmptyInput);
    }

    // Normalize all line endings to '\n'
    let normalized = sanitized.replace("\r\n", "\n").replace('\r', "\n");

    let mut track = SubtitleTrack::new(
        track_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        language.unwrap_or_else(|| "und".to_string()),
        display_name.unwrap_or_else(|| "SubRip Subtitle".to_string()),
        SubtitleFormat::Srt,
    );

    let mut warnings = Vec::new();
    let mut expected_index: u32 = 1;

    // Split text into cue chunks by double newlines
    let raw_blocks: Vec<&str> = normalized.split("\n\n").collect();

    for raw_block in raw_blocks {
        let trimmed_block = raw_block.trim();
        if trimmed_block.is_empty() {
            continue;
        }

        let lines: Vec<&str> = trimmed_block
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }

        let mut line_cursor = 0;
        let mut raw_identifier = None;

        // Check if first line is a numeric sequence identifier
        if let Ok(seq) = lines[line_cursor].parse::<u32>() {
            raw_identifier = Some(lines[line_cursor].to_string());
            if seq != expected_index {
                warnings.push(format!(
                    "Non-sequential cue index at cue #{expected_index}: expected {expected_index}, found {seq}"
                ));
            }
            line_cursor += 1;
        } else if !lines[line_cursor].contains("-->") {
            // Non-numeric identifier line
            raw_identifier = Some(lines[line_cursor].to_string());
            line_cursor += 1;
        }

        if line_cursor >= lines.len() {
            warnings.push(format!(
                "Orphaned cue header without timestamps: '{trimmed_block}'"
            ));
            continue;
        }

        // Timing line must contain "-->"
        let timing_line = lines[line_cursor];
        if !timing_line.contains("-->") {
            warnings.push(format!(
                "Expected timing specification with '-->' at cue #{expected_index}, found '{timing_line}'"
            ));
            continue;
        }
        line_cursor += 1;

        let (start_us, end_us) = match parse_srt_timing_line(timing_line) {
            Ok(times) => times,
            Err(e) => {
                warnings.push(format!("Failed parsing timing line '{timing_line}': {e}"));
                continue;
            }
        };

        // Validate time ordering
        if end_us <= start_us {
            warnings.push(format!(
                "Invalid time range in cue #{expected_index}: start ({}µs) >= end ({}µs)",
                start_us.as_micros(),
                end_us.as_micros()
            ));
            continue;
        }

        // Remaining lines are cue text
        let text_lines = &lines[line_cursor..];
        let text = text_lines.join("\n").trim().to_string();

        let cue_id = format!("cue-{}-{}", expected_index, Uuid::new_v4().as_simple());
        let mut cue = match SubtitleCue::new(cue_id, start_us, end_us, text) {
            Ok(c) => c,
            Err(e) => {
                warnings.push(format!(
                    "Cue construction error at index {expected_index}: {e}"
                ));
                continue;
            }
        };
        cue.raw_identifier = raw_identifier;

        track.cues.push(cue);
        expected_index += 1;

        if track.cues.len() > MAX_SUBTITLE_CUE_COUNT {
            return Err(SubtitleError::ExceededMaxCueCount {
                count: track.cues.len(),
                max_count: MAX_SUBTITLE_CUE_COUNT,
            });
        }
    }

    if track.cues.is_empty() {
        return Err(SubtitleError::EmptyInput);
    }

    track.sort_cues();

    Ok(SrtParseResult { track, warnings })
}

/// Parses an SRT timing line: `00:01:20,000 --> 00:01:24,400 [optional coordinates]`
pub fn parse_srt_timing_line(line: &str) -> Result<(MediaTimeUs, MediaTimeUs), SubtitleError> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return Err(SubtitleError::MalformedTimestamp {
            raw: line.to_string(),
            reason: "Timing line does not contain exactly one '-->' separator".to_string(),
        });
    }

    let start_str = parts[0].trim();
    // End string may have trailing coordinates like "00:00:04,000 X1:0 Y1:0"
    let end_raw = parts[1].trim();
    let end_str = end_raw.split_whitespace().next().unwrap_or(end_raw);

    let start_us = parse_srt_timestamp(start_str)?;
    let end_us = parse_srt_timestamp(end_str)?;

    Ok((start_us, end_us))
}

/// Parses a single SRT timestamp in `HH:MM:SS,mmm` or `HH:MM:SS.mmm` format into `MediaTimeUs`.
pub fn parse_srt_timestamp(raw: &str) -> Result<MediaTimeUs, SubtitleError> {
    let clean = raw.trim();

    // Find milliseconds separator (either ',' or '.')
    let (time_part, ms_part) = if let Some(idx) = clean.rfind(',') {
        (&clean[..idx], &clean[idx + 1..])
    } else if let Some(idx) = clean.rfind('.') {
        (&clean[..idx], &clean[idx + 1..])
    } else {
        return Err(SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: "Missing millisecond separator (',' or '.')".to_string(),
        });
    };

    let ms: i64 = ms_part
        .parse()
        .map_err(|_| SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: format!("Invalid millisecond digits '{ms_part}'"),
        })?;

    let time_tokens: Vec<&str> = time_part.split(':').collect();
    if time_tokens.len() != 3 {
        return Err(SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: "Expected HH:MM:SS format".to_string(),
        });
    }

    let hours: i64 = time_tokens[0]
        .parse()
        .map_err(|_| SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: format!("Invalid hours '{}'", time_tokens[0]),
        })?;

    let minutes: i64 = time_tokens[1]
        .parse()
        .map_err(|_| SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: format!("Invalid minutes '{}'", time_tokens[1]),
        })?;

    let seconds: i64 = time_tokens[2]
        .parse()
        .map_err(|_| SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: format!("Invalid seconds '{}'", time_tokens[2]),
        })?;

    if hours < 0
        || !(0..60).contains(&minutes)
        || !(0..60).contains(&seconds)
        || !(0..1000).contains(&ms)
    {
        return Err(SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: "Timestamp values out of normal chronological range".to_string(),
        });
    }

    let total_us = hours
        .checked_mul(3_600_000_000)
        .and_then(|h| h.checked_add(minutes.checked_mul(60_000_000)?))
        .and_then(|m| m.checked_add(seconds.checked_mul(1_000_000)?))
        .and_then(|s| s.checked_add(ms.checked_mul(1_000)?))
        .ok_or_else(|| SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: "Microsecond timestamp arithmetic overflow".to_string(),
        })?;

    Ok(MediaTimeUs::from_micros(total_us))
}

/// Formats a `MediaTimeUs` value into an SRT timestamp string (`HH:MM:SS,mmm`).
///
/// # Rounding Policy
/// Microseconds are rounded to the nearest integer millisecond using tie-breaking away from zero:
/// `total_ms = (micros + 500) / 1000`.
pub fn format_srt_timestamp(time_us: MediaTimeUs) -> String {
    let micros = time_us.as_micros().max(0);
    let total_ms = (micros + 500) / 1_000;

    let ms = total_ms % 1_000;
    let total_seconds = total_ms / 1_000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;

    format!("{hours:02}:{minutes:02}:{seconds:02},{ms:03}")
}

/// Serializes a `SubtitleTrack` into standard SubRip (.srt) format.
pub fn serialize_srt(track: &SubtitleTrack) -> Result<String, SubtitleError> {
    let mut out = String::new();

    for (idx, cue) in track.cues.iter().enumerate() {
        let seq = idx + 1;
        let start_str = format_srt_timestamp(cue.start_us);
        let end_str = format_srt_timestamp(cue.end_us);

        out.push_str(&format!("{seq}\n"));
        out.push_str(&format!("{start_str} --> {end_str}\n"));
        out.push_str(&cue.text);
        out.push_str("\n\n");
    }

    Ok(out)
}
