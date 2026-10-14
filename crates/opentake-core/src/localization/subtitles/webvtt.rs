use uuid::Uuid;

use super::error::SubtitleError;
use super::model::{
    SubtitleCue, SubtitleFormat, SubtitleTrack, WebVttCueSettings, MAX_SUBTITLE_CUE_COUNT,
    MAX_SUBTITLE_FILE_SIZE,
};
use crate::editing::time::MediaTimeUs;

/// Result of parsing a WebVTT file including the parsed track and compatibility warnings.
#[derive(Debug, Clone)]
pub struct WebVttParseResult {
    pub track: SubtitleTrack,
    pub warnings: Vec<String>,
}

/// Parses raw WebVTT (.vtt) text content into a canonical `SubtitleTrack`.
///
/// # Standards Support
/// - Required `WEBVTT` signature header (with optional trailing description or headers).
/// - Skips `NOTE` comment blocks cleanly.
/// - Gracefully bypasses `STYLE` and `REGION` blocks while issuing transparent compatibility warnings.
/// - Supports both full `HH:MM:SS.mmm` and short `MM:SS.mmm` timestamp representations.
/// - Parses WebVTT cue settings (`line:`, `position:`, `size:`, `align:`, `vertical:`).
/// - Extracts voice tags (`<v Speaker>`) into the canonical speaker field.
/// - Strips unsupported markup and unescapes WebVTT entities (`&amp;`, `&lt;`, `&gt;`, `&nbsp;`).
pub fn parse_webvtt(
    content: &str,
    track_id: Option<String>,
    language: Option<String>,
    display_name: Option<String>,
) -> Result<WebVttParseResult, SubtitleError> {
    if content.len() > MAX_SUBTITLE_FILE_SIZE {
        return Err(SubtitleError::ExceededMaxFileSize {
            size_bytes: content.len(),
            max_bytes: MAX_SUBTITLE_FILE_SIZE,
        });
    }

    // Strip UTF-8 BOM
    let sanitized = content.trim_start_matches('\u{feff}');
    if sanitized.trim().is_empty() {
        return Err(SubtitleError::EmptyInput);
    }

    // Normalize line endings to '\n'
    let normalized = sanitized.replace("\r\n", "\n").replace('\r', "\n");

    // Must start with WEBVTT signature
    let trimmed = normalized.trim_start();
    if !trimmed.starts_with("WEBVTT") {
        return Err(SubtitleError::MissingWebVttHeader);
    }

    let mut track = SubtitleTrack::new(
        track_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        language.unwrap_or_else(|| "und".to_string()),
        display_name.unwrap_or_else(|| "WebVTT Subtitle".to_string()),
        SubtitleFormat::WebVtt,
    );

    let mut warnings = Vec::new();
    let mut cue_counter: u32 = 1;

    // Split blocks by double newlines
    let raw_blocks: Vec<&str> = normalized.split("\n\n").collect();

    for (block_idx, raw_block) in raw_blocks.iter().enumerate() {
        let block = raw_block.trim();
        if block.is_empty() {
            continue;
        }

        // Check if first block contains WEBVTT header
        if block_idx == 0 && block.starts_with("WEBVTT") {
            continue;
        }

        // Handle NOTE comment blocks
        if block.starts_with("NOTE") {
            continue;
        }

        // Handle STYLE blocks
        if block.starts_with("STYLE") {
            warnings.push(
                "STYLE block encountered: custom CSS styling is not rendered in plain-text preview."
                    .to_string(),
            );
            continue;
        }

        // Handle REGION blocks
        if block.starts_with("REGION") {
            warnings.push(
                "REGION block encountered: custom rendering regions are mapped to standard canvas display."
                    .to_string(),
            );
            continue;
        }

        let lines: Vec<&str> = block
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }

        let mut line_cursor = 0;
        let mut raw_identifier = None;

        // Check for cue identifier (line before timestamp line that doesn't contain "-->")
        if !lines[line_cursor].contains("-->") {
            raw_identifier = Some(lines[line_cursor].to_string());
            line_cursor += 1;
        }

        if line_cursor >= lines.len() {
            continue;
        }

        let timing_line = lines[line_cursor];
        if !timing_line.contains("-->") {
            warnings.push(format!("Skipping invalid WebVTT block: '{timing_line}'"));
            continue;
        }
        line_cursor += 1;

        let (start_us, end_us, settings) = match parse_webvtt_timing_line(timing_line) {
            Ok(res) => res,
            Err(e) => {
                warnings.push(format!(
                    "Failed parsing WebVTT timing line '{timing_line}': {e}"
                ));
                continue;
            }
        };

        if end_us <= start_us {
            warnings.push(format!(
                "Invalid time range in WebVTT cue #{cue_counter}: start ({}µs) >= end ({}µs)",
                start_us.as_micros(),
                end_us.as_micros()
            ));
            continue;
        }

        // Remaining lines are text
        let raw_text = lines[line_cursor..].join("\n");
        let (clean_text, speaker) = sanitize_webvtt_text(&raw_text);

        let cue_id = format!("vtt-cue-{}-{}", cue_counter, Uuid::new_v4().as_simple());
        let mut cue = match SubtitleCue::new(cue_id, start_us, end_us, clean_text) {
            Ok(c) => c,
            Err(e) => {
                warnings.push(format!("Failed creating WebVTT cue #{cue_counter}: {e}"));
                continue;
            }
        };

        cue.speaker = speaker;
        cue.settings = settings;
        cue.raw_identifier = raw_identifier;

        track.cues.push(cue);
        cue_counter += 1;

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

    Ok(WebVttParseResult { track, warnings })
}

/// Parses a WebVTT timing line: `00:01:20.000 --> 00:01:24.400 line:90% align:center`
pub fn parse_webvtt_timing_line(
    line: &str,
) -> Result<(MediaTimeUs, MediaTimeUs, Option<WebVttCueSettings>), SubtitleError> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return Err(SubtitleError::MalformedTimestamp {
            raw: line.to_string(),
            reason: "Timing line does not contain exactly one '-->' separator".to_string(),
        });
    }

    let start_str = parts[0].trim();
    let right_part = parts[1].trim();

    // Right part contains end timestamp and optional cue settings separated by whitespace
    let mut tokens = right_part.split_whitespace();
    let end_str = tokens
        .next()
        .ok_or_else(|| SubtitleError::MalformedTimestamp {
            raw: line.to_string(),
            reason: "Missing end timestamp".to_string(),
        })?;

    let start_us = parse_webvtt_timestamp(start_str)?;
    let end_us = parse_webvtt_timestamp(end_str)?;

    let settings_tokens: Vec<&str> = tokens.collect();
    let settings = if !settings_tokens.is_empty() {
        Some(parse_webvtt_cue_settings(&settings_tokens))
    } else {
        None
    };

    Ok((start_us, end_us, settings))
}

/// Parses cue settings tokens (e.g. `["line:90%", "align:center"]`) into `WebVttCueSettings`.
pub fn parse_webvtt_cue_settings(tokens: &[&str]) -> WebVttCueSettings {
    let raw_settings = tokens.join(" ");
    let mut settings = WebVttCueSettings {
        raw_settings: Some(raw_settings),
        ..Default::default()
    };

    for token in tokens {
        if let Some(val) = token.strip_prefix("vertical:") {
            settings.vertical = Some(val.to_string());
        } else if let Some(val) = token.strip_prefix("line:") {
            settings.line = Some(val.to_string());
        } else if let Some(val) = token.strip_prefix("position:") {
            settings.position = Some(val.to_string());
        } else if let Some(val) = token.strip_prefix("size:") {
            settings.size = Some(val.to_string());
        } else if let Some(val) = token.strip_prefix("align:") {
            settings.align = Some(val.to_string());
        }
    }

    settings
}

/// Parses a WebVTT timestamp in `HH:MM:SS.mmm` or `MM:SS.mmm` format.
pub fn parse_webvtt_timestamp(raw: &str) -> Result<MediaTimeUs, SubtitleError> {
    let clean = raw.trim();

    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() != 2 {
        return Err(SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: "Missing millisecond separator '.' in WebVTT timestamp".to_string(),
        });
    }

    let ms_str = parts[1];
    let ms: i64 = ms_str
        .parse()
        .map_err(|_| SubtitleError::MalformedTimestamp {
            raw: raw.to_string(),
            reason: format!("Invalid millisecond digits '{ms_str}'"),
        })?;

    let time_tokens: Vec<&str> = parts[0].split(':').collect();

    let (hours, minutes, seconds) = match time_tokens.len() {
        2 => {
            // MM:SS format
            let m: i64 = time_tokens[0]
                .parse()
                .map_err(|_| SubtitleError::MalformedTimestamp {
                    raw: raw.to_string(),
                    reason: format!("Invalid minutes '{}'", time_tokens[0]),
                })?;
            let s: i64 = time_tokens[1]
                .parse()
                .map_err(|_| SubtitleError::MalformedTimestamp {
                    raw: raw.to_string(),
                    reason: format!("Invalid seconds '{}'", time_tokens[1]),
                })?;
            (0, m, s)
        }
        3 => {
            // HH:MM:SS format
            let h: i64 = time_tokens[0]
                .parse()
                .map_err(|_| SubtitleError::MalformedTimestamp {
                    raw: raw.to_string(),
                    reason: format!("Invalid hours '{}'", time_tokens[0]),
                })?;
            let m: i64 = time_tokens[1]
                .parse()
                .map_err(|_| SubtitleError::MalformedTimestamp {
                    raw: raw.to_string(),
                    reason: format!("Invalid minutes '{}'", time_tokens[1]),
                })?;
            let s: i64 = time_tokens[2]
                .parse()
                .map_err(|_| SubtitleError::MalformedTimestamp {
                    raw: raw.to_string(),
                    reason: format!("Invalid seconds '{}'", time_tokens[2]),
                })?;
            (h, m, s)
        }
        _ => {
            return Err(SubtitleError::MalformedTimestamp {
                raw: raw.to_string(),
                reason: "Timestamp must have MM:SS or HH:MM:SS format".to_string(),
            });
        }
    };

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

/// Sanitizes WebVTT cue text by extracting `<v Speaker>` tags and unescaping WebVTT entities.
///
/// Returns `(cleaned_plain_text, extracted_speaker)`.
pub fn sanitize_webvtt_text(raw: &str) -> (String, Option<String>) {
    let mut speaker = None;
    let text = raw.to_string();

    // Check for <v SpeakerName> or <v.voice SpeakerName>
    if let Some(v_start) = text.find("<v") {
        if let Some(v_end) = text[v_start..].find('>') {
            let tag_content = &text[v_start + 2..v_start + v_end].trim();
            let speaker_name = tag_content.trim_start_matches(['.', ' ']);
            if !speaker_name.is_empty() {
                speaker = Some(speaker_name.to_string());
            }
        }
    }

    // Strip tags: <...>
    let mut in_tag = false;
    let mut filtered = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            filtered.push(ch);
        }
    }

    // Replace WebVTT HTML character entities
    let unescaped = filtered
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&lrm;", "\u{200E}")
        .replace("&rlm;", "\u{200F}")
        .replace("&nbsp;", "\u{00A0}");

    (unescaped.trim().to_string(), speaker)
}

/// Formats a `MediaTimeUs` value into standard WebVTT timestamp string (`HH:MM:SS.mmm`).
pub fn format_webvtt_timestamp(time_us: MediaTimeUs) -> String {
    let micros = time_us.as_micros().max(0);
    let total_ms = (micros + 500) / 1_000;

    let ms = total_ms % 1_000;
    let total_seconds = total_ms / 1_000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}.{ms:03}")
}

/// Serializes a `SubtitleTrack` into standards-compliant WebVTT format.
pub fn serialize_webvtt(track: &SubtitleTrack) -> Result<String, SubtitleError> {
    let mut out = String::from("WEBVTT\n\n");

    for (idx, cue) in track.cues.iter().enumerate() {
        if let Some(ref identifier) = cue.raw_identifier {
            out.push_str(identifier);
            out.push('\n');
        } else {
            out.push_str(&(idx + 1).to_string());
            out.push('\n');
        }

        let start_str = format_webvtt_timestamp(cue.start_us);
        let end_str = format_webvtt_timestamp(cue.end_us);

        out.push_str(&format!("{start_str} --> {end_str}"));

        if let Some(ref settings) = cue.settings {
            let setting_str = settings.to_setting_string();
            if !setting_str.is_empty() {
                out.push(' ');
                out.push_str(&setting_str);
            }
        }
        out.push('\n');

        if let Some(ref speaker) = cue.speaker {
            out.push_str(&format!("<v {speaker}>"));
            out.push_str(&cue.text);
            out.push_str("</v>\n\n");
        } else {
            out.push_str(&cue.text);
            out.push_str("\n\n");
        }
    }

    Ok(out)
}
