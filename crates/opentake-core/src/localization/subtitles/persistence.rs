use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use super::error::SubtitleError;
use super::model::{SubtitleDocument, MAX_SUBTITLE_FILE_SIZE, SUBTITLE_DOCUMENT_SCHEMA_VERSION};

/// Persists a `SubtitleDocument` atomically to the filesystem using a temporary swap file.
///
/// # Atomic Guarantee
/// Writes content to a adjacent temporary file (`.tmp`), calls `sync_all` on the underlying
/// file descriptor, and atomically renames the temporary file to the destination path.
/// This prevents partial file corruption in the event of abrupt application crash or power cut.
pub fn save_subtitle_document(doc: &SubtitleDocument, path: &Path) -> Result<(), SubtitleError> {
    let parent = path.parent().ok_or_else(|| {
        SubtitleError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Path has no parent directory",
        ))
    })?;

    // Ensure parent directory exists
    fs::create_dir_all(parent)?;

    let json_bytes =
        serde_json::to_vec_pretty(doc).map_err(|e| SubtitleError::SerializationError {
            message: format!("Failed to serialize SubtitleDocument to JSON: {e}"),
        })?;

    let tmp_path = parent.join(format!(
        ".{}.tmp.{}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("subdoc"),
        uuid::Uuid::new_v4().as_simple()
    ));

    {
        let mut tmp_file = File::create(&tmp_path)?;
        tmp_file.write_all(&json_bytes)?;
        tmp_file.sync_all()?;
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        SubtitleError::Io(e)
    })?;

    Ok(())
}

/// Loads and validates a `SubtitleDocument` from the filesystem.
///
/// # Safety Guarantees
/// - Rejects oversized project files exceeding `MAX_SUBTITLE_FILE_SIZE` (10 MB).
/// - Enforces maximum supported schema version.
/// - Recovers safely from malformed JSON by returning descriptive errors rather than panicking.
pub fn load_subtitle_document(path: &Path) -> Result<SubtitleDocument, SubtitleError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_SUBTITLE_FILE_SIZE as u64 {
        return Err(SubtitleError::ExceededMaxFileSize {
            size_bytes: metadata.len() as usize,
            max_bytes: MAX_SUBTITLE_FILE_SIZE,
        });
    }

    let bytes = fs::read(path)?;
    let content = std::str::from_utf8(&bytes)?;

    let doc: SubtitleDocument =
        serde_json::from_str(content).map_err(|e| SubtitleError::DeserializationError {
            message: format!("Failed to parse SubtitleDocument JSON: {e}"),
        })?;

    if doc.schema_version > SUBTITLE_DOCUMENT_SCHEMA_VERSION {
        return Err(SubtitleError::UnsupportedSchemaVersion {
            version: doc.schema_version,
            supported: SUBTITLE_DOCUMENT_SCHEMA_VERSION,
        });
    }

    Ok(doc)
}
