use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const TRANSLATION_MEMORY_SCHEMA_VERSION: u32 = 1;

/// A single translation memory unit recording an approved human translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationMemoryEntry {
    pub id: String,
    pub source_text_normalized: String,
    pub target_text: String,
    pub source_language: String,
    pub target_language: String,
    pub domain: Option<String>,
    pub usage_count: u32,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}

/// In-memory repository of previously approved translation units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationMemory {
    pub schema_version: u32,
    pub entries: Vec<TranslationMemoryEntry>,
    pub updated_at_epoch_ms: u64,
}

impl TranslationMemory {
    pub fn new() -> Self {
        Self {
            schema_version: TRANSLATION_MEMORY_SCHEMA_VERSION,
            entries: Vec::new(),
            updated_at_epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Normalizes text for comparison by trimming, lowercasing, and collapsing whitespace.
    pub fn normalize_text(text: &str) -> String {
        text.split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
            .to_lowercase()
    }

    /// Queries the translation memory for an exact normalized match.
    pub fn find_exact_match(
        &self,
        source_text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Option<&TranslationMemoryEntry> {
        let norm = Self::normalize_text(source_text);
        if norm.is_empty() {
            return None;
        }

        self.entries.iter().find(|e| {
            e.source_text_normalized == norm
                && e.source_language.eq_ignore_ascii_case(source_lang)
                && e.target_language.eq_ignore_ascii_case(target_lang)
        })
    }

    /// Records or updates an approved translation in memory.
    pub fn record_approved(
        &mut self,
        source_text: &str,
        target_text: &str,
        source_lang: &str,
        target_lang: &str,
        domain: Option<&str>,
    ) -> TranslationMemoryEntry {
        let norm = Self::normalize_text(source_text);
        let trimmed_target = target_text.trim();
        if norm.is_empty() || trimmed_target.is_empty() {
            return TranslationMemoryEntry {
                id: String::new(),
                source_text_normalized: norm,
                target_text: trimmed_target.to_string(),
                source_language: source_lang.trim().to_lowercase(),
                target_language: target_lang.trim().to_lowercase(),
                domain: domain.map(|s| s.to_string()),
                usage_count: 0,
                created_at_epoch_ms: 0,
                updated_at_epoch_ms: 0,
            };
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let entry = if let Some(pos) = self.entries.iter().position(|e| {
            e.source_text_normalized == norm
                && e.source_language.eq_ignore_ascii_case(source_lang)
                && e.target_language.eq_ignore_ascii_case(target_lang)
        }) {
            let entry = &mut self.entries[pos];
            entry.target_text = trimmed_target.to_string();
            entry.usage_count += 1;
            entry.updated_at_epoch_ms = now;
            if domain.is_some() {
                entry.domain = domain.map(|s| s.to_string());
            }
            entry.clone()
        } else {
            let entry = TranslationMemoryEntry {
                id: format!("tm-{}", Uuid::new_v4()),
                source_text_normalized: norm,
                target_text: trimmed_target.to_string(),
                source_language: source_lang.trim().to_lowercase(),
                target_language: target_lang.trim().to_lowercase(),
                domain: domain.map(|s| s.to_string()),
                usage_count: 1,
                created_at_epoch_ms: now,
                updated_at_epoch_ms: now,
            };
            self.entries.push(entry.clone());
            entry
        };

        self.updated_at_epoch_ms = now;
        entry
    }

    /// Deletes a specific translation memory entry by ID.
    pub fn delete_entry(&mut self, entry_id: &str) -> bool {
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

    /// Clears all translation memory entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.updated_at_epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }
}

impl Default for TranslationMemory {
    fn default() -> Self {
        Self::new()
    }
}
