use super::glossary::GlossaryEntry;
use super::models::{QualityFinding, QualityFindingCode, QualitySeverity, TranslationReviewEntry};
use super::terminology::validate_terminology;

/// Maximum acceptable reading speed threshold in characters per second.
pub const DEFAULT_MAX_CPS: f64 = 25.0;

/// Maximum characters per line before generating a readability overflow warning.
pub const DEFAULT_MAX_CHARS_PER_LINE: usize = 84;

/// Performs a comprehensive rule-based quality evaluation on a single review entry.
pub fn evaluate_entry_quality(
    entry: &TranslationReviewEntry,
    glossary_entries: &[GlossaryEntry],
    is_language_pair_verified: bool,
) -> Vec<QualityFinding> {
    let mut findings = Vec::new();

    let trimmed_src = entry.source_text.trim();
    let trimmed_tgt = entry.target_text.trim();

    // 1. Empty target text check
    if trimmed_tgt.is_empty() && !trimmed_src.is_empty() {
        findings.push(QualityFinding {
            code: QualityFindingCode::EmptyTargetText,
            severity: QualitySeverity::Error,
            message: "Target subtitle cue is empty.".to_string(),
            details: Some(format!(
                "Source dialogue has {} characters.",
                trimmed_src.chars().count()
            )),
        });
    }

    // 2. Untranslated text check (identical source and target)
    if !trimmed_src.is_empty() && trimmed_src.eq_ignore_ascii_case(trimmed_tgt) {
        // Exclude purely numeric or short acronyms
        let is_pure_number = trimmed_src
            .chars()
            .all(|c| c.is_numeric() || c.is_ascii_punctuation());
        if !is_pure_number && trimmed_src.len() > 3 {
            findings.push(QualityFinding {
                code: QualityFindingCode::PossibleUntranslatedText,
                severity: QualitySeverity::Warning,
                message: "Target text is identical to source text; translation may be missing."
                    .to_string(),
                details: None,
            });
        }
    }

    // 3. Timing mismatch check
    if entry.target_start_us != entry.source_start_us || entry.target_end_us != entry.source_end_us
    {
        findings.push(QualityFinding {
            code: QualityFindingCode::SourceTargetTimingMismatch,
            severity: QualitySeverity::Info,
            message: "Target cue timing differs from source cue timing.".to_string(),
            details: Some(format!(
                "Source: [{}, {}], Target: [{}, {}]",
                entry.source_start_us.0,
                entry.source_end_us.0,
                entry.target_start_us.0,
                entry.target_end_us.0
            )),
        });
    }

    // 4. Reading speed (CPS) check
    let dur_us = entry.target_end_us.0 - entry.target_start_us.0;
    if dur_us > 0 {
        let dur_secs = dur_us as f64 / 1_000_000.0;
        let char_count = trimmed_tgt.chars().count();
        let cps = char_count as f64 / dur_secs;
        if cps > DEFAULT_MAX_CPS {
            findings.push(QualityFinding {
                code: QualityFindingCode::ExcessiveReadingSpeed,
                severity: QualitySeverity::Warning,
                message: format!(
                    "Reading speed of {:.1} chars/sec exceeds comfortable threshold ({:.0} CPS).",
                    cps, DEFAULT_MAX_CPS
                ),
                details: Some(format!("{char_count} chars over {dur_secs:.2} seconds")),
            });
        }
    }

    // 5. Excessive line length check
    for (line_idx, line) in trimmed_tgt.lines().enumerate() {
        let char_count = line.chars().count();
        if char_count > DEFAULT_MAX_CHARS_PER_LINE {
            findings.push(QualityFinding {
                code: QualityFindingCode::ExcessiveLineLength,
                severity: QualitySeverity::Warning,
                message: format!(
                    "Line {} contains {} characters (exceeds standard limit of {}).",
                    line_idx + 1,
                    char_count,
                    DEFAULT_MAX_CHARS_PER_LINE
                ),
                details: Some(
                    "Consider splitting the line or breaking into multiple cues.".to_string(),
                ),
            });
        }
    }

    // 6. Stale approval check
    if entry.is_approval_stale() {
        findings.push(QualityFinding {
            code: QualityFindingCode::StaleReviewApproval,
            severity: QualitySeverity::Warning,
            message: "Approval is stale: target text or timing was modified since approval."
                .to_string(),
            details: Some(
                "Re-review and approve the cue to update the approval certificate.".to_string(),
            ),
        });
    }

    // 7. Language pair verification status check
    if !is_language_pair_verified {
        findings.push(QualityFinding {
            code: QualityFindingCode::LanguagePairUnverified,
            severity: QualitySeverity::Info,
            message: "This language pair has not been marked production-verified; human review recommended.".to_string(),
            details: None,
        });
    }

    // 8. Terminology and Protected Brand checks
    let term_findings = validate_terminology(trimmed_src, trimmed_tgt, glossary_entries);
    findings.extend(term_findings);

    findings
}
