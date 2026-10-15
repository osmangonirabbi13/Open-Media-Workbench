//! Evidence-based timing refinement aligning subtitle cues with verified word boundaries.

use crate::editing::time::MediaTimeUs;
use crate::localization::subtitles::model::SubtitleCue;

use super::errors::AlignmentError;
use super::models::{AlignedWord, WordTimingProvenance};

/// Snaps cue start and end boundaries to match the earliest and latest spoken word timestamps.
pub fn refine_cues_with_words(cues: &mut [SubtitleCue], min_cue_duration_us: i64) {
    if cues.is_empty() {
        return;
    }

    for i in 0..cues.len() {
        let (first_word_start, last_word_end) = if let Some(ref words) = cues[i].words {
            if words.is_empty() {
                continue;
            }
            (
                words.first().unwrap().start_us,
                words.last().unwrap().end_us,
            )
        } else {
            continue;
        };

        let mut refined_start = first_word_start;
        let mut refined_end = last_word_end;

        // Ensure minimum duration
        if refined_end.as_micros() - refined_start.as_micros() < min_cue_duration_us {
            refined_end = MediaTimeUs(refined_start.as_micros() + min_cue_duration_us);
        }

        // Clamp to not cross into subsequent cue's start time
        if i + 1 < cues.len() {
            let next_start = cues[i + 1].start_us;
            if refined_end > next_start {
                refined_end = next_start;
            }
        }

        // Clamp to not cross earlier than preceding cue's end time
        if i > 0 {
            let prev_end = cues[i - 1].end_us;
            if refined_start < prev_end {
                refined_start = prev_end;
            }
        }

        if refined_end > refined_start {
            cues[i].start_us = refined_start;
            cues[i].end_us = refined_end;
        }
    }
}

/// Updates the timestamp boundaries of an individual word, marking its provenance as UserAdjusted.
pub fn update_word_timing_in_cue(
    cue: &mut SubtitleCue,
    word_id: &str,
    new_start_us: MediaTimeUs,
    new_end_us: MediaTimeUs,
) -> Result<AlignedWord, AlignmentError> {
    if new_start_us.as_micros() < 0 {
        return Err(AlignmentError::NegativeTime {
            time_us: new_start_us.as_micros(),
        });
    }
    if new_end_us <= new_start_us {
        return Err(AlignmentError::InvalidWordTimestamp {
            word: word_id.to_string(),
            start_us: new_start_us.as_micros(),
            end_us: new_end_us.as_micros(),
        });
    }

    let words = cue
        .words
        .as_mut()
        .ok_or_else(|| AlignmentError::WordNotFound {
            word_id: word_id.to_string(),
        })?;

    let word =
        words
            .iter_mut()
            .find(|w| w.id == word_id)
            .ok_or_else(|| AlignmentError::WordNotFound {
                word_id: word_id.to_string(),
            })?;

    word.start_us = new_start_us;
    word.end_us = new_end_us;
    word.provenance = WordTimingProvenance::UserAdjusted;

    let updated_word = word.clone();

    // Re-synchronize cue boundaries to enclose all words
    if let (Some(first), Some(last)) = (words.first(), words.last()) {
        if first.start_us < cue.start_us {
            cue.start_us = first.start_us;
        }
        if last.end_us > cue.end_us {
            cue.end_us = last.end_us;
        }
    }

    Ok(updated_word)
}
