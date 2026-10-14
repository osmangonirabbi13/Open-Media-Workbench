use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::recovery_report::{RecoveryStatus, ValidationLevel};
use super::recovery_review::ValidationOutcome;

pub const RECOVERY_HISTORY_SCHEMA_VERSION: u32 = 1;
pub const MAX_HISTORY_ENTRIES: usize = 50;

/// An individual persistent audit record of a past recovery attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryHistoryEntry {
    pub job_id: String,
    pub source_asset_id: String,
    pub source_file_name: String,
    pub recovery_method: String,
    pub attempt_date_time_epoch_ms: u64,
    pub final_status: RecoveryStatus,
    pub validation_level: ValidationLevel,
    pub validation_outcome: ValidationOutcome,
    pub output_available: bool,
    pub output_file_name: Option<String>,
    pub report_exported: bool,
    pub source_duration_seconds: Option<f64>,
    pub recovered_duration_seconds: Option<f64>,
    pub limitations: Vec<String>,
}

/// Versioned local store for recovery history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryHistoryStore {
    pub schema_version: u32,
    pub entries: Vec<RecoveryHistoryEntry>,
}

impl Default for RecoveryHistoryStore {
    fn default() -> Self {
        Self {
            schema_version: RECOVERY_HISTORY_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

impl RecoveryHistoryStore {
    /// Loads the history store from a JSON file.
    /// If the file does not exist, returns an empty default store.
    /// If the file is corrupted or an older/unrecognized schema, falls back gracefully.
    pub fn load_from_file(file_path: &Path) -> Self {
        if !file_path.exists() {
            return Self::default();
        }

        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        match serde_json::from_str::<RecoveryHistoryStore>(&content) {
            Ok(store) => {
                if store.schema_version == RECOVERY_HISTORY_SCHEMA_VERSION {
                    store
                } else {
                    // Graceful schema migration or fallback
                    Self::default()
                }
            }
            Err(_) => Self::default(),
        }
    }

    /// Atomically persists the history store to disk using a temporary file and rename.
    pub fn save_to_file(&self, file_path: &Path) -> io::Result<()> {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let temp_path: PathBuf = file_path.with_extension("tmp");
        fs::write(&temp_path, json.as_bytes())?;

        // Atomic rename guarantees crash-safety
        fs::rename(&temp_path, file_path)?;
        Ok(())
    }

    /// Records a new recovery attempt, bounding the total list size to `MAX_HISTORY_ENTRIES`.
    pub fn add_entry(&mut self, entry: RecoveryHistoryEntry) {
        // Remove duplicate if job_id already exists (update existing)
        self.entries.retain(|e| e.job_id != entry.job_id);

        // Prepend new entry so most recent is first
        self.entries.insert(0, entry);

        // Bound to maximum entries
        if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries.truncate(MAX_HISTORY_ENTRIES);
        }
    }

    /// Clears all history entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_round_trip_and_atomic_write() {
        let temp_dir = tempfile::tempdir().unwrap();
        let history_file = temp_dir.path().join("recovery_history.json");

        let mut store = RecoveryHistoryStore::load_from_file(&history_file);
        assert_eq!(store.entries.len(), 0);

        let entry = RecoveryHistoryEntry {
            job_id: "job-123".to_string(),
            source_asset_id: "asset-1".to_string(),
            source_file_name: "broken.mp4".to_string(),
            recovery_method: "Lossless Remux".to_string(),
            attempt_date_time_epoch_ms: 1700000000,
            final_status: RecoveryStatus::Completed,
            validation_level: ValidationLevel::SampledDecode,
            validation_outcome: ValidationOutcome::Passed,
            output_available: true,
            output_file_name: Some("recovered_job-123.mp4".to_string()),
            report_exported: false,
            source_duration_seconds: Some(10.0),
            recovered_duration_seconds: Some(10.0),
            limitations: vec!["Sampled decode only".to_string()],
        };

        store.add_entry(entry.clone());
        store.save_to_file(&history_file).unwrap();

        // Reload from file
        let loaded = RecoveryHistoryStore::load_from_file(&history_file);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].job_id, "job-123");
        assert_eq!(loaded.entries[0].source_file_name, "broken.mp4");

        // Test clear
        let mut clear_store = loaded;
        clear_store.clear();
        assert_eq!(clear_store.entries.len(), 0);
    }

    #[test]
    fn test_history_bounding() {
        let mut store = RecoveryHistoryStore::default();
        for i in 0..60 {
            store.add_entry(RecoveryHistoryEntry {
                job_id: format!("job-{i}"),
                source_asset_id: format!("asset-{i}"),
                source_file_name: format!("file_{i}.mp4"),
                recovery_method: "Stream Salvage".to_string(),
                attempt_date_time_epoch_ms: 1700000000 + i,
                final_status: RecoveryStatus::Completed,
                validation_level: ValidationLevel::MetadataOnly,
                validation_outcome: ValidationOutcome::Passed,
                output_available: true,
                output_file_name: Some(format!("out_{i}.mp4")),
                report_exported: false,
                source_duration_seconds: None,
                recovered_duration_seconds: Some(5.0),
                limitations: Vec::new(),
            });
        }

        assert_eq!(store.entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(store.entries[0].job_id, "job-59");
    }
}
