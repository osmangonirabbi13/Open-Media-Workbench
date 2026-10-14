use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::model::{SubtitleCue, SubtitleDocument, SubtitleTrack};
use crate::editing::time::MediaTimeUs;

/// Severity classification for subtitle quality inspection findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FindingSeverity {
    /// Pure informational advice or minor style notice.
    Info,
    /// Non-fatal quality deviation, potential overlap, or bounds discrepancy.
    Warning,
    /// Critical structural defect violating time invariants or rendering integrity.
    Error,
}

/// A structured validation diagnostic finding for subtitle tracks and cues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleFinding {
    /// Stable finding code (e.g. `EMPTY_CUE_TEXT`, `OVERLAPPING_CUES`, `OUT_OF_MEDIA_BOUNDS`).
    pub code: String,
    /// Finding severity level.
    pub severity: FindingSeverity,
    /// Human-readable diagnostic explanation.
    pub message: String,
    /// Optional associated track ID.
    pub track_id: Option<String>,
    /// Optional associated cue ID.
    pub cue_id: Option<String>,
    /// Optional timestamp range start in integer microseconds.
    pub start_us: Option<MediaTimeUs>,
    /// Optional timestamp range end in integer microseconds.
    pub end_us: Option<MediaTimeUs>,
}

/// Maximum sensible duration for a single subtitle cue before triggering an `EXCESSIVE_DURATION` warning (30 seconds).
pub const MAX_NORMAL_CUE_DURATION_US: i64 = 30_000_000;

/// Minimum sensible duration for a readable subtitle cue before triggering an `EXTREMELY_SHORT_DURATION` notice (100 ms).
pub const MIN_READABLE_CUE_DURATION_US: i64 = 100_000;

/// Validates an entire `SubtitleTrack` and returns all diagnostic findings.
pub fn validate_track(
    track: &SubtitleTrack,
    known_media_duration_us: Option<MediaTimeUs>,
) -> Vec<SubtitleFinding> {
    let mut findings = Vec::new();

    if track.cues.is_empty() {
        findings.push(SubtitleFinding {
            code: "EMPTY_TRACK".to_string(),
            severity: FindingSeverity::Warning,
            message: format!("Track '{}' contains no subtitle cues.", track.display_name),
            track_id: Some(track.id.clone()),
            cue_id: None,
            start_us: None,
            end_us: None,
        });
        return findings;
    }

    let mut seen_ids = HashSet::new();
    let mut prev_cue: Option<&SubtitleCue> = None;

    for (idx, cue) in track.cues.iter().enumerate() {
        // 1. Check duplicate cue ID
        if !seen_ids.insert(&cue.id) {
            findings.push(SubtitleFinding {
                code: "DUPLICATE_CUE_ID".to_string(),
                severity: FindingSeverity::Warning,
                message: format!("Duplicate cue identifier detected: '{}'.", cue.id),
                track_id: Some(track.id.clone()),
                cue_id: Some(cue.id.clone()),
                start_us: Some(cue.start_us),
                end_us: Some(cue.end_us),
            });
        }

        // 2. Check negative start time
        if cue.start_us.as_micros() < 0 {
            findings.push(SubtitleFinding {
                code: "NEGATIVE_START_TIME".to_string(),
                severity: FindingSeverity::Error,
                message: format!(
                    "Cue #{} has negative start timestamp ({}µs).",
                    idx + 1,
                    cue.start_us.as_micros()
                ),
                track_id: Some(track.id.clone()),
                cue_id: Some(cue.id.clone()),
                start_us: Some(cue.start_us),
                end_us: Some(cue.end_us),
            });
        }

        // 3. Check invalid time range: end <= start
        if cue.end_us <= cue.start_us {
            findings.push(SubtitleFinding {
                code: "INVALID_TIME_RANGE".to_string(),
                severity: FindingSeverity::Error,
                message: format!(
                    "Cue #{} end time ({}µs) must be strictly greater than start time ({}µs).",
                    idx + 1,
                    cue.end_us.as_micros(),
                    cue.start_us.as_micros()
                ),
                track_id: Some(track.id.clone()),
                cue_id: Some(cue.id.clone()),
                start_us: Some(cue.start_us),
                end_us: Some(cue.end_us),
            });
        }

        // 4. Check empty text
        if cue.text.trim().is_empty() {
            findings.push(SubtitleFinding {
                code: "EMPTY_CUE_TEXT".to_string(),
                severity: FindingSeverity::Warning,
                message: format!("Cue #{} contains empty or whitespace-only text.", idx + 1),
                track_id: Some(track.id.clone()),
                cue_id: Some(cue.id.clone()),
                start_us: Some(cue.start_us),
                end_us: Some(cue.end_us),
            });
        }

        let duration = cue.duration_us().as_micros();

        // 5. Check excessively long cue duration (> 30s)
        if duration > MAX_NORMAL_CUE_DURATION_US {
            findings.push(SubtitleFinding {
                code: "EXCESSIVE_DURATION".to_string(),
                severity: FindingSeverity::Warning,
                message: format!(
                    "Cue #{} duration ({:.1}s) exceeds recommended maximum (30s).",
                    idx + 1,
                    duration as f64 / 1_000_000.0
                ),
                track_id: Some(track.id.clone()),
                cue_id: Some(cue.id.clone()),
                start_us: Some(cue.start_us),
                end_us: Some(cue.end_us),
            });
        }

        // 6. Check extremely short cue duration (< 100ms)
        if duration > 0 && duration < MIN_READABLE_CUE_DURATION_US {
            findings.push(SubtitleFinding {
                code: "EXTREMELY_SHORT_DURATION".to_string(),
                severity: FindingSeverity::Info,
                message: format!(
                    "Cue #{} duration is extremely short ({}ms) and may be unreadable.",
                    idx + 1,
                    duration / 1_000
                ),
                track_id: Some(track.id.clone()),
                cue_id: Some(cue.id.clone()),
                start_us: Some(cue.start_us),
                end_us: Some(cue.end_us),
            });
        }

        // 7. Check out of media duration bounds
        if let Some(media_dur) = known_media_duration_us {
            if cue.start_us > media_dur {
                findings.push(SubtitleFinding {
                    code: "OUT_OF_MEDIA_BOUNDS".to_string(),
                    severity: FindingSeverity::Warning,
                    message: format!(
                        "Cue #{} starts at {:.2}s, exceeding known media duration of {:.2}s.",
                        idx + 1,
                        cue.start_us.to_seconds_f64_lossy(),
                        media_dur.to_seconds_f64_lossy()
                    ),
                    track_id: Some(track.id.clone()),
                    cue_id: Some(cue.id.clone()),
                    start_us: Some(cue.start_us),
                    end_us: Some(cue.end_us),
                });
            }
        }

        // 8. Ordering and overlap checks relative to previous cue
        if let Some(prev) = prev_cue {
            if cue.start_us < prev.start_us {
                findings.push(SubtitleFinding {
                    code: "UNSORTED_CUES".to_string(),
                    severity: FindingSeverity::Warning,
                    message: format!(
                        "Cue #{} starts ({:.3}s) before preceding cue starts ({:.3}s).",
                        idx + 1,
                        cue.start_us.to_seconds_f64_lossy(),
                        prev.start_us.to_seconds_f64_lossy()
                    ),
                    track_id: Some(track.id.clone()),
                    cue_id: Some(cue.id.clone()),
                    start_us: Some(cue.start_us),
                    end_us: Some(cue.end_us),
                });
            } else if cue.start_us < prev.end_us {
                findings.push(SubtitleFinding {
                    code: "OVERLAPPING_CUES".to_string(),
                    severity: FindingSeverity::Warning,
                    message: format!(
                        "Cue #{} overlaps with preceding cue by {}ms.",
                        idx + 1,
                        (prev.end_us.as_micros() - cue.start_us.as_micros()) / 1_000
                    ),
                    track_id: Some(track.id.clone()),
                    cue_id: Some(cue.id.clone()),
                    start_us: Some(cue.start_us),
                    end_us: Some(cue.end_us),
                });
            }
        }

        prev_cue = Some(cue);
    }

    findings
}

/// Validates all tracks contained in a `SubtitleDocument`.
pub fn validate_document(
    doc: &SubtitleDocument,
    known_media_duration_us: Option<MediaTimeUs>,
) -> Vec<SubtitleFinding> {
    let mut findings = Vec::new();

    if doc.tracks.is_empty() {
        findings.push(SubtitleFinding {
            code: "EMPTY_DOCUMENT".to_string(),
            severity: FindingSeverity::Warning,
            message: "Subtitle document contains no tracks.".to_string(),
            track_id: None,
            cue_id: None,
            start_us: None,
            end_us: None,
        });
        return findings;
    }

    for track in &doc.tracks {
        findings.extend(validate_track(track, known_media_duration_us));
    }

    findings
}
