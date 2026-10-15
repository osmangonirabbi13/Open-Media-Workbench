use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::TranslationReviewError;

pub const GLOSSARY_SCHEMA_VERSION: u32 = 1;

/// Single terminology entry mapping a source term to a preferred/protected target term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntry {
    pub id: String,
    pub source_term: String,
    pub preferred_term: String,
    pub source_language: String,
    pub target_language: String,
    pub description: Option<String>,
    pub is_protected: bool,
    pub case_sensitive: bool,
    pub domain: Option<String>,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}

impl GlossaryEntry {
    pub fn new(
        source_term: impl Into<String>,
        preferred_term: impl Into<String>,
        source_language: impl Into<String>,
        target_language: impl Into<String>,
        is_protected: bool,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: format!("ge-{}", Uuid::new_v4()),
            source_term: source_term.into().trim().to_string(),
            preferred_term: preferred_term.into().trim().to_string(),
            source_language: source_language.into().trim().to_lowercase(),
            target_language: target_language.into().trim().to_lowercase(),
            description: None,
            is_protected,
            case_sensitive: false,
            domain: None,
            created_at_epoch_ms: now,
            updated_at_epoch_ms: now,
        }
    }
}

/// Named collection of terminology entries for a language pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Glossary {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub source_language: String,
    pub target_language: String,
    pub project_id: Option<String>,
    pub entries: Vec<GlossaryEntry>,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}

impl Glossary {
    pub fn new(
        name: impl Into<String>,
        source_language: impl Into<String>,
        target_language: impl Into<String>,
        project_id: Option<String>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            schema_version: GLOSSARY_SCHEMA_VERSION,
            id: format!("glo-{}", Uuid::new_v4()),
            name: name.into().trim().to_string(),
            source_language: source_language.into().trim().to_lowercase(),
            target_language: target_language.into().trim().to_lowercase(),
            project_id,
            entries: Vec::new(),
            created_at_epoch_ms: now,
            updated_at_epoch_ms: now,
        }
    }

    /// Adds or updates an entry in the glossary with conflict checking.
    pub fn upsert_entry(&mut self, mut entry: GlossaryEntry) -> Result<(), TranslationReviewError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        entry.source_term = entry.source_term.trim().to_string();
        entry.preferred_term = entry.preferred_term.trim().to_string();
        entry.updated_at_epoch_ms = now;

        // Check for conflicting term: same source term, different target term (unless same entry ID)
        for existing in &self.entries {
            let matches_source = if entry.case_sensitive || existing.case_sensitive {
                existing.source_term == entry.source_term
            } else {
                existing
                    .source_term
                    .eq_ignore_ascii_case(&entry.source_term)
            };

            if matches_source
                && existing.id != entry.id
                && existing.preferred_term != entry.preferred_term
            {
                return Err(TranslationReviewError::ConflictingGlossaryTerm {
                    term: entry.source_term,
                    existing: existing.preferred_term.clone(),
                    new_target: entry.preferred_term,
                });
            }
        }

        if let Some(pos) = self.entries.iter().position(|e| e.id == entry.id) {
            self.entries[pos] = entry;
        } else {
            self.entries.push(entry);
        }

        self.updated_at_epoch_ms = now;
        Ok(())
    }

    /// Deletes an entry by its ID.
    pub fn remove_entry(&mut self, entry_id: &str) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|e| e.id != entry_id);
        if self.entries.len() != initial_len {
            self.updated_at_epoch_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Finds all terms matching a given source term (case-insensitive unless specified).
    pub fn find_terms_for_source(&self, source_word: &str) -> Vec<&GlossaryEntry> {
        self.entries
            .iter()
            .filter(|e| {
                if e.case_sensitive {
                    e.source_term == source_word
                } else {
                    e.source_term.eq_ignore_ascii_case(source_word)
                }
            })
            .collect()
    }
}
