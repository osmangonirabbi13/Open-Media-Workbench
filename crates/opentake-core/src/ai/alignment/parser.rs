//! Parser converting raw Whisper token/segment data into canonical AlignedWord sequences.

use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;

use super::errors::AlignmentError;
use super::models::{AlignedWord, TimingQuality, WordTimingProvenance};
use crate::editing::time::MediaTimeUs;

/// Punctuation characters to recognize across Latin, Bengali, and universal scripts.
const SENTENCE_PUNCTUATION: &[char] = &['.', '!', '?', '।', '॥', '—', '…'];
const CLAUSE_PUNCTUATION: &[char] = &[
    ',', ';', ':', '-', '(', ')', '"', '\'', '“', '”', '‘', '’', '¿', '¡',
];

/// Raw token entry from Whisper JSON-full output.
#[derive(Debug, Clone, Deserialize)]
pub struct RawWhisperToken {
    pub text: String,
    #[serde(default)]
    pub offsets: Option<RawOffsets>,
    #[serde(default)]
    pub timestamps: Option<RawTimestamps>,
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub p: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawOffsets {
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawTimestamps {
    pub from: String,
    pub to: String,
}

/// Raw segment entry from Whisper JSON output.
#[derive(Debug, Clone, Deserialize)]
pub struct RawWhisperSegment {
    #[serde(default)]
    pub offsets: Option<RawOffsets>,
    #[serde(default)]
    pub timestamps: Option<RawTimestamps>,
    pub text: String,
    #[serde(default)]
    pub tokens: Option<Vec<RawWhisperToken>>,
}

/// Root JSON structure emitted by whisper.cpp.
#[derive(Debug, Clone, Deserialize)]
pub struct RawWhisperJson {
    pub result: Option<RawWhisperResult>,
    pub transcription: Option<Vec<RawWhisperSegment>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawWhisperResult {
    pub language: Option<String>,
}

/// Identifies whether a character is a punctuation symbol.
pub fn is_punctuation_char(c: char) -> bool {
    SENTENCE_PUNCTUATION.contains(&c) || CLAUSE_PUNCTUATION.contains(&c) || c.is_ascii_punctuation()
}

/// Splits a raw token into (punctuation_prefix, clean_word, punctuation_suffix).
pub fn split_word_punctuation(raw: &str) -> (Option<String>, String, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (None, String::new(), None);
    }

    // If string is entirely punctuation, return it as word or suffix
    if trimmed.chars().all(is_punctuation_char) {
        return (None, trimmed.to_string(), None);
    }

    let mut start_idx = 0;
    let mut end_idx = trimmed.len();

    // Scan prefix punctuation
    for (idx, ch) in trimmed.char_indices() {
        if is_punctuation_char(ch) {
            start_idx = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    // Scan suffix punctuation from end
    for (idx, ch) in trimmed.char_indices().rev() {
        if is_punctuation_char(ch) {
            end_idx = idx;
        } else {
            break;
        }
    }

    if start_idx >= end_idx {
        return (None, trimmed.to_string(), None);
    }

    let prefix = if start_idx > 0 {
        Some(trimmed[..start_idx].to_string())
    } else {
        None
    };

    let word = trimmed[start_idx..end_idx].to_string();

    let suffix = if end_idx < trimmed.len() {
        Some(trimmed[end_idx..].to_string())
    } else {
        None
    };

    (prefix, word, suffix)
}

/// Parses raw whisper.cpp JSON output into a sequence of canonical AlignedWord objects.
pub fn parse_whisper_alignment_output(
    json_str: &str,
    source_segment_id_prefix: Option<&str>,
) -> Result<(Vec<AlignedWord>, TimingQuality), AlignmentError> {
    let parsed: RawWhisperJson = serde_json::from_str(json_str)
        .map_err(|e| AlignmentError::ParserError(format!("Invalid whisper JSON: {e}")))?;

    let segments = parsed.transcription.ok_or(AlignmentError::NoWordsFound)?;

    if segments.is_empty() {
        return Err(AlignmentError::NoWordsFound);
    }

    let mut words = Vec::new();
    let mut has_any_timing = false;
    let mut total_tokens = 0;

    for (seg_idx, seg) in segments.into_iter().enumerate() {
        let seg_text = seg.text.trim();
        let seg_id = format!("{}{}", source_segment_id_prefix.unwrap_or("seg_"), seg_idx);

        // Case 1: whisper emitted token-level breakdown with timing
        if let Some(tokens) = seg.tokens {
            if !tokens.is_empty() {
                for (tok_idx, tok) in tokens.into_iter().enumerate() {
                    let raw_text = tok.text.trim();
                    if raw_text.is_empty() || raw_text.starts_with("[_") {
                        continue; // Skip special tokens like [_BEG_]
                    }

                    total_tokens += 1;
                    let (start_ms, end_ms) = if let Some(ref offs) = tok.offsets {
                        has_any_timing = true;
                        (offs.from, offs.to)
                    } else if let Some(ref ts) = tok.timestamps {
                        has_any_timing = true;
                        let f = crate::ai::transcription::segments::parse_timestamp_str_to_micros(
                            &ts.from,
                        )
                        .unwrap_or(0)
                            / 1000;
                        let t = crate::ai::transcription::segments::parse_timestamp_str_to_micros(
                            &ts.to,
                        )
                        .unwrap_or(f + 50)
                            / 1000;
                        (f, t)
                    } else if let Some(ref seg_offs) = seg.offsets {
                        has_any_timing = true;
                        (seg_offs.from, seg_offs.to)
                    } else if let Some(ref seg_ts) = seg.timestamps {
                        has_any_timing = true;
                        let f = crate::ai::transcription::segments::parse_timestamp_str_to_micros(
                            &seg_ts.from,
                        )
                        .unwrap_or(0)
                            / 1000;
                        let t = crate::ai::transcription::segments::parse_timestamp_str_to_micros(
                            &seg_ts.to,
                        )
                        .unwrap_or(f + 50)
                            / 1000;
                        (f, t)
                    } else {
                        (0, 0)
                    };

                    let start_us = MediaTimeUs(start_ms.max(0) * 1000);
                    // Ensure end > start (at least 50ms)
                    let end_us = MediaTimeUs(
                        (end_ms.max(start_ms) * 1000).max(start_us.as_micros() + 50_000),
                    );

                    let (prefix, clean_word, suffix) = split_word_punctuation(raw_text);

                    // If token was pure punctuation and we have a preceding word, attach as suffix
                    if clean_word.chars().all(is_punctuation_char) && !words.is_empty() {
                        let last: &mut AlignedWord = words.last_mut().unwrap();
                        let existing_suffix = last.punctuation_suffix.clone().unwrap_or_default();
                        last.punctuation_suffix =
                            Some(format!("{}{}", existing_suffix, clean_word));
                        if end_us > last.end_us {
                            last.end_us = end_us;
                        }
                        continue;
                    }

                    let word_id = format!("w_{seg_idx}_{tok_idx}");
                    let mut aligned = AlignedWord::new(
                        word_id,
                        clean_word,
                        start_us,
                        end_us,
                        WordTimingProvenance::WhisperNativeEstimate,
                    )?;

                    aligned.source_segment_id = Some(seg_id.clone());
                    aligned.confidence = tok.p;
                    aligned.punctuation_prefix = prefix;
                    aligned.punctuation_suffix = suffix;

                    words.push(aligned);
                }
                continue;
            }
        }

        // Case 2: Segment-level output (e.g. from -ml 1 or standard segment)
        if seg_text.is_empty() {
            continue;
        }

        total_tokens += 1;
        let (start_ms, end_ms) = if let Some(ref offs) = seg.offsets {
            has_any_timing = true;
            (offs.from, offs.to)
        } else {
            (0, 0)
        };

        let start_us = MediaTimeUs(start_ms.max(0) * 1000);
        let end_us = MediaTimeUs((end_ms.max(start_ms) * 1000).max(start_us.as_micros() + 50_000));

        let (prefix, clean_word, suffix) = split_word_punctuation(seg_text);

        // If pure punctuation and we have a preceding word, attach as suffix
        if clean_word.chars().all(is_punctuation_char) && !words.is_empty() {
            let last: &mut AlignedWord = words.last_mut().unwrap();
            let existing_suffix = last.punctuation_suffix.clone().unwrap_or_default();
            last.punctuation_suffix = Some(format!("{}{}", existing_suffix, clean_word));
            if end_us > last.end_us {
                last.end_us = end_us;
            }
            continue;
        }

        // If segment contained multiple words separated by whitespace, tokenize them
        let word_tokens: Vec<&str> = clean_word.unicode_words().collect();
        if word_tokens.len() > 1 {
            let total_words = word_tokens.len();
            let total_dur = (end_us.as_micros() - start_us.as_micros()).max(50_000);
            let per_word = total_dur / total_words as i64;

            for (w_idx, w_text) in word_tokens.iter().enumerate() {
                let w_start = MediaTimeUs(start_us.as_micros() + (w_idx as i64 * per_word));
                let w_end = if w_idx + 1 == total_words {
                    end_us
                } else {
                    MediaTimeUs(w_start.as_micros() + per_word)
                };

                let word_id = format!("w_{seg_idx}_{w_idx}");
                let mut aligned = AlignedWord::new(
                    word_id,
                    *w_text,
                    w_start,
                    w_end,
                    WordTimingProvenance::WhisperNativeEstimate,
                )?;
                aligned.source_segment_id = Some(seg_id.clone());
                if w_idx == 0 {
                    aligned.punctuation_prefix = prefix.clone();
                }
                if w_idx + 1 == total_words {
                    aligned.punctuation_suffix = suffix.clone();
                }
                words.push(aligned);
            }
        } else {
            let word_id = format!("w_{seg_idx}_0");
            let mut aligned = AlignedWord::new(
                word_id,
                clean_word,
                start_us,
                end_us,
                WordTimingProvenance::WhisperNativeEstimate,
            )?;
            aligned.source_segment_id = Some(seg_id.clone());
            aligned.punctuation_prefix = prefix;
            aligned.punctuation_suffix = suffix;
            words.push(aligned);
        }
    }

    if words.is_empty() {
        return Err(AlignmentError::NoWordsFound);
    }

    let quality = if !has_any_timing {
        TimingQuality::Unavailable
    } else if words.len() == total_tokens {
        TimingQuality::Aligned
    } else {
        TimingQuality::Estimated
    };

    Ok((words, quality))
}
