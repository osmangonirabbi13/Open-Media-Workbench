//! Smart subtitle segmentation partitioning aligned words into broadcast-compliant SubtitleCues.

use crate::editing::time::MediaTimeUs;
use crate::localization::subtitles::model::SubtitleCue;

use super::models::{AlignedWord, SegmentationPolicy};

/// Sentence-ending punctuation symbols that indicate natural phrase boundaries.
const SENTENCE_ENDERS: &[char] = &['.', '!', '?', '।', '॥', '—'];

/// Intelligently formats a sequence of words into 1 or 2 lines respecting character boundaries.
pub fn format_cue_lines(
    words: &[AlignedWord],
    max_chars_per_line: usize,
    max_lines: usize,
) -> String {
    if words.is_empty() {
        return String::new();
    }

    let tokens: Vec<String> = words.iter().map(|w| w.display_text()).collect();
    let total_text = tokens.join(" ");

    // If fits on a single line, max_lines is 1, or there is only 1 token, return single line
    if total_text.chars().count() <= max_chars_per_line || max_lines <= 1 || tokens.len() <= 1 {
        return total_text;
    }

    // Try to split into 2 visually balanced lines at a word boundary near the midpoint
    let target_split_len = total_text.chars().count() / 2;
    let mut best_split_idx = 1;
    let mut best_diff = usize::MAX;

    let mut current_len = 0;
    for (idx, tok) in tokens.iter().enumerate().take(tokens.len() - 1) {
        current_len += tok.chars().count() + if idx > 0 { 1 } else { 0 };
        let diff = (current_len as isize - target_split_len as isize).unsigned_abs();
        if diff < best_diff {
            best_diff = diff;
            best_split_idx = idx + 1;
        }
    }

    let line1 = tokens[..best_split_idx].join(" ");
    let line2 = tokens[best_split_idx..].join(" ");

    if line2.is_empty() {
        line1
    } else {
        format!("{line1}\n{line2}")
    }
}

/// Segments a sequence of aligned words into canonical SubtitleCue objects guided by SegmentationPolicy.
pub fn segment_words_into_cues(
    words: &[AlignedWord],
    policy: &SegmentationPolicy,
    _language: &str,
) -> (Vec<SubtitleCue>, Vec<String>) {
    let mut cues = Vec::new();
    let warnings = Vec::new();

    if words.is_empty() {
        return (cues, warnings);
    }

    let mut current_words: Vec<AlignedWord> = Vec::new();
    let mut cue_idx = 0;

    let flush_cue = |current: &mut Vec<AlignedWord>,
                     cues: &mut Vec<SubtitleCue>,
                     cue_idx: &mut usize,
                     next_word_start: Option<MediaTimeUs>| {
        if current.is_empty() {
            return;
        }

        let first = &current[0];
        let last = &current[current.len() - 1];

        let start_us = first.start_us;
        let mut end_us = last.end_us;

        // Enforce minimum cue duration if bounded by next word
        let dur = end_us.as_micros() - start_us.as_micros();
        if dur < policy.min_cue_duration_us {
            let padded_end = start_us.as_micros() + policy.min_cue_duration_us;
            if let Some(next_start) = next_word_start {
                end_us = MediaTimeUs(padded_end.min(next_start.as_micros()));
            } else {
                end_us = MediaTimeUs(padded_end);
            }
        }

        let cue_id = format!("cue_align_{cue_idx}");
        *cue_idx += 1;

        let cue_text =
            format_cue_lines(current, policy.max_chars_per_line, policy.max_lines_per_cue);

        // Update word associations
        for w in current.iter_mut() {
            w.source_cue_id = Some(cue_id.clone());
        }

        if let Ok(mut cue) = SubtitleCue::new(cue_id, start_us, end_us, cue_text) {
            cue.words = Some(current.clone());
            cues.push(cue);
        }

        current.clear();
    };

    for i in 0..words.len() {
        let word = &words[i];
        let next_word = words.get(i + 1);

        // 1. Check pre-insertion split triggers if current buffer is non-empty
        if !current_words.is_empty() {
            let prev_word = current_words.last().unwrap();
            let silence_gap = word.start_us.as_micros() - prev_word.end_us.as_micros();

            // Trigger A: Silence gap between words exceeds threshold (e.g. >= 400ms)
            let is_silence_split = silence_gap >= policy.split_on_silence_gap_us;

            // Trigger B: Exceeds max cue duration
            let potential_duration =
                word.end_us.as_micros() - current_words[0].start_us.as_micros();
            let is_duration_split = potential_duration > policy.max_cue_duration_us;

            // Trigger C: Exceeds max characters for the cue
            let current_char_count: usize = current_words
                .iter()
                .map(|w| w.display_text().chars().count() + 1)
                .sum();
            let word_char_count = word.display_text().chars().count() + 1;
            let is_char_split = current_char_count + word_char_count
                > (policy.max_chars_per_line * policy.max_lines_per_cue);

            if is_silence_split || is_duration_split || is_char_split {
                flush_cue(
                    &mut current_words,
                    &mut cues,
                    &mut cue_idx,
                    Some(word.start_us),
                );
            }
        }

        // Add current word to buffer
        current_words.push(word.clone());

        // 2. Check post-insertion split triggers (punctuation boundaries)
        if policy.split_on_punctuation {
            let has_sentence_ender = word
                .punctuation_suffix
                .as_deref()
                .map(|s| s.chars().any(|c| SENTENCE_ENDERS.contains(&c)))
                .unwrap_or(false);

            if has_sentence_ender {
                let next_start = next_word.map(|w| w.start_us);
                flush_cue(&mut current_words, &mut cues, &mut cue_idx, next_start);
            }
        }
    }

    // Flush any remaining words in final cue
    flush_cue(&mut current_words, &mut cues, &mut cue_idx, None);

    (cues, warnings)
}
