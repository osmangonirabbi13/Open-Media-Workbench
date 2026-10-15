use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::localization::subtitles::model::SubtitleDocument;

use super::errors::TranslationReviewError;
use super::glossary::Glossary;
use super::memory::TranslationMemory;
use super::models::TranslationReviewDocument;

pub const REVIEW_METADATA_KEY: &str = "translationReview";

/// Persistent storage manager for glossaries and translation memory.
#[derive(Clone)]
pub struct ReviewStorageManager {
    base_dir: PathBuf,
    glossaries_dir: PathBuf,
    memory_file: PathBuf,
    memory_lock: Arc<RwLock<TranslationMemory>>,
}

impl ReviewStorageManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self, TranslationReviewError> {
        let base_dir = base_dir.into();
        let glossaries_dir = base_dir.join("glossaries");
        let memory_file = base_dir.join("translation_memory.json");

        fs::create_dir_all(&glossaries_dir).map_err(|e| {
            TranslationReviewError::IoError(format!("Failed to create glossaries dir: {e}"))
        })?;

        let memory = if memory_file.exists() {
            let data = fs::read_to_string(&memory_file).map_err(|e| {
                TranslationReviewError::IoError(format!("Failed to read memory file: {e}"))
            })?;
            serde_json::from_str(&data).unwrap_or_else(|_| TranslationMemory::new())
        } else {
            TranslationMemory::new()
        };

        Ok(Self {
            base_dir,
            glossaries_dir,
            memory_file,
            memory_lock: Arc::new(RwLock::new(memory)),
        })
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    // --- GLOSSARY PERSISTENCE ---

    pub fn save_glossary(&self, glossary: &Glossary) -> Result<(), TranslationReviewError> {
        let filename = format!("{}.json", glossary.id);
        let path = self.glossaries_dir.join(&filename);
        let tmp_path = self.glossaries_dir.join(format!("{}.tmp", filename));

        let data = serde_json::to_string_pretty(glossary)
            .map_err(|e| TranslationReviewError::SerializationError(e.to_string()))?;

        {
            let mut file = File::create(&tmp_path).map_err(|e| {
                TranslationReviewError::IoError(format!("Failed creating temp file: {e}"))
            })?;
            file.write_all(data.as_bytes()).map_err(|e| {
                TranslationReviewError::IoError(format!("Failed writing temp file: {e}"))
            })?;
            file.sync_all().map_err(|e| {
                TranslationReviewError::IoError(format!("Failed syncing file: {e}"))
            })?;
        }

        fs::rename(&tmp_path, &path).map_err(|e| {
            TranslationReviewError::IoError(format!("Failed renaming temp file: {e}"))
        })?;

        Ok(())
    }

    pub fn load_glossary(&self, glossary_id: &str) -> Result<Glossary, TranslationReviewError> {
        let path = self.glossaries_dir.join(format!("{glossary_id}.json"));
        if !path.exists() {
            return Err(TranslationReviewError::GlossaryNotFound(
                glossary_id.to_string(),
            ));
        }

        let data = fs::read_to_string(&path).map_err(|e| {
            TranslationReviewError::IoError(format!("Failed reading glossary file: {e}"))
        })?;

        serde_json::from_str(&data)
            .map_err(|e| TranslationReviewError::SerializationError(e.to_string()))
    }

    pub fn list_glossaries(&self) -> Result<Vec<Glossary>, TranslationReviewError> {
        let mut list = Vec::new();
        if !self.glossaries_dir.exists() {
            return Ok(list);
        }

        let entries = fs::read_dir(&self.glossaries_dir).map_err(|e| {
            TranslationReviewError::IoError(format!("Failed reading glossaries dir: {e}"))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(glo) = serde_json::from_str::<Glossary>(&data) {
                        list.push(glo);
                    }
                }
            }
        }

        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    pub fn delete_glossary(&self, glossary_id: &str) -> Result<bool, TranslationReviewError> {
        let path = self.glossaries_dir.join(format!("{glossary_id}.json"));
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                TranslationReviewError::IoError(format!("Failed deleting glossary: {e}"))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // --- TRANSLATION MEMORY PERSISTENCE ---

    pub fn get_memory(&self) -> TranslationMemory {
        self.memory_lock.read().unwrap().clone()
    }

    pub fn update_memory<F, R>(&self, f: F) -> Result<R, TranslationReviewError>
    where
        F: FnOnce(&mut TranslationMemory) -> R,
    {
        let mut guard = self.memory_lock.write().unwrap();
        let res = f(&mut guard);

        // Atomic write memory file
        let data = serde_json::to_string_pretty(&*guard)
            .map_err(|e| TranslationReviewError::SerializationError(e.to_string()))?;

        let tmp_path = self.base_dir.join("translation_memory.json.tmp");
        {
            let mut file = File::create(&tmp_path).map_err(|e| {
                TranslationReviewError::IoError(format!("Failed creating memory tmp: {e}"))
            })?;
            file.write_all(data.as_bytes()).map_err(|e| {
                TranslationReviewError::IoError(format!("Failed writing memory tmp: {e}"))
            })?;
            file.sync_all().map_err(|e| {
                TranslationReviewError::IoError(format!("Failed syncing memory: {e}"))
            })?;
        }

        fs::rename(&tmp_path, &self.memory_file).map_err(|e| {
            TranslationReviewError::IoError(format!("Failed committing memory file: {e}"))
        })?;

        Ok(res)
    }
}

// --- SUBTITLE DOCUMENT EMBEDDING ---

/// Embeds a TranslationReviewDocument into a SubtitleDocument's metadata for project persistence.
pub fn embed_review_in_document(
    document: &mut SubtitleDocument,
    review_doc: &TranslationReviewDocument,
) -> Result<(), TranslationReviewError> {
    let json_str = serde_json::to_string(review_doc)
        .map_err(|e| TranslationReviewError::SerializationError(e.to_string()))?;

    document
        .metadata
        .insert(REVIEW_METADATA_KEY.to_string(), json_str);
    document.touch();
    Ok(())
}

/// Extracts an embedded TranslationReviewDocument from a SubtitleDocument's metadata.
pub fn extract_review_from_document(
    document: &SubtitleDocument,
) -> Option<TranslationReviewDocument> {
    document
        .metadata
        .get(REVIEW_METADATA_KEY)
        .and_then(|json_str| serde_json::from_str(json_str).ok())
}
