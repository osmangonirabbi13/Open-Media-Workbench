//! Validation rules ensuring word-level timestamps, boundaries, and Unicode strings are valid.

use super::errors::AlignmentError;
use super::models::AlignedWord;

/// Validates word boundary invariants, chronological monotonicity, and Unicode integrity.
pub fn validate_word_sequence(words: &[AlignedWord]) -> Result<Vec<String>, AlignmentError> {
    let mut warnings = Vec::new();

    if words.is_empty() {
        return Ok(warnings);
    }

    let mut prev_start_us: i64 = 0;
    let mut prev_end_us: i64 = 0;

    for (idx, word) in words.iter().enumerate() {
        let start = word.start_us.as_micros();
        let end = word.end_us.as_micros();

        // 1. Invariant: non-negative start
        if start < 0 {
            return Err(AlignmentError::NegativeTime { time_us: start });
        }

        // 2. Invariant: end > start
        if end <= start {
            return Err(AlignmentError::InvalidWordTimestamp {
                word: word.text.clone(),
                start_us: start,
                end_us: end,
            });
        }

        // 3. Chronological monotonicity: start must not precede previous start
        if idx > 0 && start < prev_start_us {
            return Err(AlignmentError::OutOfOrder {
                word: word.text.clone(),
                current_start_us: start,
                previous_start_us: prev_start_us,
            });
        }

        // 4. Diagnostic: overlap with previous word
        if idx > 0 && start < prev_end_us {
            let overlap_us = prev_end_us - start;
            warnings.push(format!(
                "Word '{word}' overlaps preceding word by {overlap_ms}ms",
                word = word.text,
                overlap_ms = overlap_us / 1000
            ));
        }

        prev_start_us = start;

        // 4. Diagnostic: anomalously long word duration (> 5 seconds)
        let dur_us = end - start;
        if dur_us > 5_000_000 {
            warnings.push(format!(
                "Word '{word}' has an exceptionally long duration ({dur_s:.1}s)",
                word = word.text,
                dur_s = dur_us as f64 / 1_000_000.0
            ));
        }

        // 5. Unicode integrity: check for replacement character (\u{FFFD})
        if word.text.contains('\u{FFFD}') {
            warnings.push(format!(
                "Word at index {idx} contains corrupted Unicode replacement character"
            ));
        }

        prev_end_us = end;
    }

    Ok(warnings)
}
