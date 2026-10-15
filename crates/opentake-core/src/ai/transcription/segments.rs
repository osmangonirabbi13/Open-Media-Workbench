use uuid::Uuid;

use crate::editing::time::MediaTimeUs;

use super::errors::TranscriptionError;
use super::models::TranscriptionSegment;

/// Minimum allowable segment duration (100 ms = 100,000 µs).
pub const MIN_SEGMENT_DURATION_US: i64 = 100_000;

/// Parses standard Whisper timestamp string (`HH:MM:SS,mmm` or `HH:MM:SS.mmm`) into integer microseconds.
pub fn parse_timestamp_str_to_micros(ts: &str) -> Result<i64, TranscriptionError> {
    let clean = ts.trim().replace(',', ".");
    let parts: Vec<&str> = clean.split(':').collect();
    if parts.len() != 3 {
        return Err(TranscriptionError::OutputParsingFailed(format!(
            "Invalid timestamp format (expected HH:MM:SS.mmm): '{ts}'"
        )));
    }

    let hours: i64 = parts[0].parse().map_err(|_| {
        TranscriptionError::OutputParsingFailed(format!("Invalid hours in timestamp: '{ts}'"))
    })?;
    let minutes: i64 = parts[1].parse().map_err(|_| {
        TranscriptionError::OutputParsingFailed(format!("Invalid minutes in timestamp: '{ts}'"))
    })?;

    let seconds_parts: Vec<&str> = parts[2].split('.').collect();
    let seconds: i64 = seconds_parts[0].parse().map_err(|_| {
        TranscriptionError::OutputParsingFailed(format!("Invalid seconds in timestamp: '{ts}'"))
    })?;

    let mut millis: i64 = 0;
    if seconds_parts.len() > 1 {
        let millis_str = seconds_parts[1];
        // Normalize up to 3 digits (milliseconds)
        let padded = format!("{:0<3}", millis_str);
        millis = padded[..3].parse().unwrap_or(0);
    }

    let total_micros = (hours * 3600 + minutes * 60 + seconds) * 1_000_000 + millis * 1_000;
    Ok(total_micros)
}

/// Raw unvalidated segment produced from parser before timing reconciliation.
#[derive(Debug, Clone)]
pub struct RawTranscriptionSegment {
    pub start_us: i64,
    pub end_us: i64,
    pub text: String,
    pub detected_language: Option<String>,
    pub confidence: Option<f32>,
}

/// Normalizes, validates, and sorts a collection of raw transcription segments into canonical segments.
pub fn process_raw_segments(
    raw_segments: Vec<RawTranscriptionSegment>,
    offset_us: i64,
) -> Vec<TranscriptionSegment> {
    let mut processed: Vec<TranscriptionSegment> = Vec::new();

    for raw in raw_segments {
        let text = raw.text.trim().to_string();
        if text.is_empty() {
            continue;
        }

        // Apply audio offset (e.g. from seek or chunking)
        let mut start_us = raw.start_us + offset_us;
        let mut end_us = raw.end_us + offset_us;

        // Ensure non-negative start
        if start_us < 0 {
            start_us = 0;
        }

        // Ensure positive duration
        if end_us <= start_us {
            end_us = start_us + MIN_SEGMENT_DURATION_US;
        }

        let segment = TranscriptionSegment {
            id: Uuid::new_v4().to_string(),
            start_us: MediaTimeUs(start_us),
            end_us: MediaTimeUs(end_us),
            text,
            detected_language: raw.detected_language,
            confidence: raw.confidence,
        };

        processed.push(segment);
    }

    // Sort chronologically by start timestamp, then end timestamp
    processed.sort_by(|a, b| {
        a.start_us
            .as_micros()
            .cmp(&b.start_us.as_micros())
            .then_with(|| a.end_us.as_micros().cmp(&b.end_us.as_micros()))
    });

    // Deduplicate adjacent identical segments
    deduplicate_segments(processed)
}

/// Removes adjacent duplicate segments that have identical start times and text.
fn deduplicate_segments(segments: Vec<TranscriptionSegment>) -> Vec<TranscriptionSegment> {
    let mut result: Vec<TranscriptionSegment> = Vec::with_capacity(segments.len());

    for seg in segments {
        if let Some(last) = result.last() {
            if last.start_us == seg.start_us && last.text == seg.text {
                continue;
            }
        }
        result.push(seg);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp_str_to_micros() {
        assert_eq!(parse_timestamp_str_to_micros("00:00:00,000").unwrap(), 0);
        assert_eq!(
            parse_timestamp_str_to_micros("00:00:01,500").unwrap(),
            1_500_000
        );
        assert_eq!(
            parse_timestamp_str_to_micros("01:02:03.456").unwrap(),
            (3600 + 120 + 3) * 1_000_000 + 456_000
        );
    }

    #[test]
    fn test_process_raw_segments_sorting_and_invariants() {
        let raw = vec![
            RawTranscriptionSegment {
                start_us: 2_000_000,
                end_us: 4_000_000,
                text: "Second line".to_string(),
                detected_language: Some("en".to_string()),
                confidence: Some(0.95),
            },
            RawTranscriptionSegment {
                start_us: 0,
                end_us: 1_500_000,
                text: "First line".to_string(),
                detected_language: Some("en".to_string()),
                confidence: Some(0.98),
            },
            RawTranscriptionSegment {
                start_us: 0,
                end_us: 1_500_000,
                text: "First line".to_string(), // duplicate
                detected_language: Some("en".to_string()),
                confidence: Some(0.98),
            },
            RawTranscriptionSegment {
                start_us: 5_000_000,
                end_us: 5_000_000, // zero duration
                text: "Third line বাংলা".to_string(),
                detected_language: Some("bn".to_string()),
                confidence: Some(0.92),
            },
        ];

        let processed = process_raw_segments(raw, 0);
        assert_eq!(processed.len(), 3);
        assert_eq!(processed[0].text, "First line");
        assert_eq!(processed[0].start_us.as_micros(), 0);
        assert_eq!(processed[1].text, "Second line");
        assert_eq!(processed[1].start_us.as_micros(), 2_000_000);
        assert_eq!(processed[2].text, "Third line বাংলা");
        assert_eq!(processed[2].start_us.as_micros(), 5_000_000);
        assert!(processed[2].end_us.as_micros() > 5_000_000);
    }
}
