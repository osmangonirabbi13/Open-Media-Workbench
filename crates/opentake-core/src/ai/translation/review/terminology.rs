use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;

use super::errors::TranslationReviewError;
use super::glossary::GlossaryEntry;
use super::models::{QualityFinding, QualityFindingCode, QualitySeverity};

/// Checks whether a term appears in `text` at Unicode word boundaries.
pub fn text_contains_word(text: &str, term: &str, case_sensitive: bool) -> bool {
    let term = term.trim();
    if term.is_empty() || text.is_empty() {
        return false;
    }

    // For multi-word terms (e.g. "Open Media Workbench"), perform boundary-aware slice search
    if term.contains(' ') {
        let hay = if case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };
        let needle = if case_sensitive {
            term.to_string()
        } else {
            term.to_lowercase()
        };

        let mut start_idx = 0;
        while let Some(pos) = hay[start_idx..].find(&needle) {
            let actual_pos = start_idx + pos;
            let end_pos = actual_pos + needle.len();

            // Check left boundary
            let left_ok = if actual_pos == 0 {
                true
            } else {
                let prev_char = text[..actual_pos].chars().last().unwrap_or(' ');
                !prev_char.is_alphanumeric()
            };

            // Check right boundary
            let right_ok = if end_pos >= text.len() {
                true
            } else {
                let next_char = text[end_pos..].chars().next().unwrap_or(' ');
                !next_char.is_alphanumeric()
            };

            if left_ok && right_ok {
                return true;
            }

            start_idx = actual_pos + 1;
            if start_idx >= hay.len() {
                break;
            }
        }
        return false;
    }

    // For single-word terms, segment by Unicode words
    for word in text.unicode_words() {
        let matches = if case_sensitive {
            word == term
        } else {
            word.eq_ignore_ascii_case(term)
        };
        if matches {
            return true;
        }
    }

    false
}

/// Evaluates source and target text against a set of glossary entries.
/// Returns quality findings for any protected term changes or missing preferred terms.
pub fn validate_terminology(
    source_text: &str,
    target_text: &str,
    glossary_entries: &[GlossaryEntry],
) -> Vec<QualityFinding> {
    let mut findings = Vec::new();

    for entry in glossary_entries {
        // Check if the source text contains this glossary entry's source term
        if text_contains_word(source_text, &entry.source_term, entry.case_sensitive) {
            let expected_target = if entry.preferred_term.is_empty() {
                &entry.source_term
            } else {
                &entry.preferred_term
            };

            // Check if expected term exists in target text
            let found_in_target =
                text_contains_word(target_text, expected_target, entry.case_sensitive);

            if !found_in_target {
                if entry.is_protected {
                    findings.push(QualityFinding {
                        code: QualityFindingCode::ProtectedTermChanged,
                        severity: QualitySeverity::Error,
                        message: format!(
                            "Protected brand/proper noun '{}' is missing or modified in translation.",
                            expected_target
                        ),
                        details: Some(format!(
                            "Source term '{}' requires protected preservation of '{}'.",
                            entry.source_term, expected_target
                        )),
                    });
                } else {
                    findings.push(QualityFinding {
                        code: QualityFindingCode::PreferredTermMissing,
                        severity: QualitySeverity::Warning,
                        message: format!(
                            "Preferred terminology '{}' was not found in target subtitle.",
                            expected_target
                        ),
                        details: Some(format!(
                            "Source term '{}' is mapped to preferred translation '{}'.",
                            entry.source_term, expected_target
                        )),
                    });
                }
            }
        }
    }

    findings
}

/// Placeholder protection system for preserving protected terms during AI translation.
pub struct ProtectedTermMasker {
    placeholders: HashMap<String, String>,
}

impl ProtectedTermMasker {
    pub fn new() -> Self {
        Self {
            placeholders: HashMap::new(),
        }
    }

    /// Masks protected terms in `text` with unique non-colliding tokens like `__OMW_TERM_0__`.
    pub fn mask(&mut self, text: &str, protected_terms: &[&str]) -> String {
        let mut result = text.to_string();
        for (idx, term) in protected_terms.iter().enumerate() {
            let term = term.trim();
            if term.is_empty() {
                continue;
            }

            // Create a unique token unlikely to collide with regular subtitle text
            let token = format!("__OMW_TERM_{idx}__");

            if text_contains_word(&result, term, false) {
                self.placeholders.insert(token.clone(), term.to_string());
                // Replace case-insensitively preserving whole word boundaries
                result = replace_whole_word_ci(&result, term, &token);
            }
        }
        result
    }

    /// Unmasks and restores protected terms in translated text.
    /// Returns error if any placeholder was dropped or duplicated.
    pub fn unmask(&self, translated_text: &str) -> Result<String, TranslationReviewError> {
        let mut restored = translated_text.to_string();

        for (token, original_term) in &self.placeholders {
            let count = restored.matches(token).count();
            if count == 0 {
                return Err(TranslationReviewError::PlaceholderRestorationFailed(format!(
                    "Placeholder token '{token}' for protected term '{original_term}' was dropped by translation model."
                )));
            }
            if count > 1 {
                return Err(TranslationReviewError::PlaceholderRestorationFailed(format!(
                    "Placeholder token '{token}' for protected term '{original_term}' was duplicated {count} times by translation model."
                )));
            }

            restored = restored.replace(token, original_term);
        }

        Ok(restored)
    }
}

impl Default for ProtectedTermMasker {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to replace whole words case-insensitively.
fn replace_whole_word_ci(text: &str, word: &str, replacement: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_word = word.to_lowercase();

    let mut out = String::with_capacity(text.len());
    let mut last_idx = 0;

    while let Some(pos) = lower_text[last_idx..].find(&lower_word) {
        let actual_pos = last_idx + pos;
        let end_pos = actual_pos + word.len();

        let left_ok = actual_pos == 0
            || !text[..actual_pos]
                .chars()
                .last()
                .unwrap_or(' ')
                .is_alphanumeric();
        let right_ok = end_pos >= text.len()
            || !text[end_pos..]
                .chars()
                .next()
                .unwrap_or(' ')
                .is_alphanumeric();

        if left_ok && right_ok {
            out.push_str(&text[last_idx..actual_pos]);
            out.push_str(replacement);
            last_idx = end_pos;
        } else {
            out.push_str(&text[last_idx..actual_pos + 1]);
            last_idx = actual_pos + 1;
        }

        if last_idx >= text.len() {
            break;
        }
    }

    if last_idx < text.len() {
        out.push_str(&text[last_idx..]);
    }

    out
}
