use std::fs;
use std::path::Path;
use tauri::State;

use serde::{Deserialize, Serialize};

use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::{
    export_subtitles_to_string, load_subtitle_document, parse_subtitles_from_str,
    save_subtitle_document, validate_document, validate_track, SubtitleDocument, SubtitleFinding,
    SubtitleFormat, SubtitleTrack, MAX_SUBTITLE_FILE_SIZE,
};
use opentake_core::media::authorization::SourceProvenance;

use super::media_import::MediaLibraryState;

/// Response payload when importing a subtitle file into the studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleImportResponse {
    pub document: SubtitleDocument,
    pub track: SubtitleTrack,
    pub findings: Vec<SubtitleFinding>,
    pub warnings: Vec<String>,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub file_path: Option<String>,
}

/// Response payload after exporting a subtitle track to a user-selected path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleExportResponse {
    pub exported_path: String,
    pub bytes_written: u64,
    pub format: SubtitleFormat,
    pub warnings: Vec<String>,
}

/// Response payload after saving a versioned subtitle project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleProjectSaveResponse {
    pub project_path: String,
    pub bytes_written: u64,
}

/// Response payload after opening an existing subtitle project from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleProjectOpenResponse {
    pub document: SubtitleDocument,
    pub findings: Vec<SubtitleFinding>,
    pub file_name: String,
    pub file_path: String,
}

/// Helper function to extract language code from filename (e.g. `video.en.srt` -> "en", `movie.bn.vtt` -> "bn").
fn infer_language_from_filename(filename: &str) -> String {
    let parts: Vec<&str> = filename.split('.').collect();
    if parts.len() >= 3 {
        let candidate = parts[parts.len() - 2].to_lowercase();
        // Check for common ISO 639-1 two-letter and three-letter codes
        if candidate.len() == 2 || candidate.len() == 3 {
            return candidate;
        }
    }
    "und".to_string()
}

/// Native user-directed subtitle file picker and parser (.srt / .vtt).
#[tauri::command]
pub async fn pick_and_import_subtitle_file(
    associated_asset_id: Option<String>,
    library: State<'_, MediaLibraryState>,
) -> Result<Option<SubtitleImportResponse>, String> {
    // 1. Open native OS file dialog with narrow subtitle file filters
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Import Subtitle File")
        .add_filter("Subtitle Files (*.srt, *.vtt)", &["srt", "vtt", "webvtt"])
        .add_filter("SubRip Subtitle (*.srt)", &["srt"])
        .add_filter("WebVTT Subtitle (*.vtt)", &["vtt", "webvtt"])
        .add_filter("All Files (*.*)", &["*"]);

    let handle = dialog.pick_file().await;
    let Some(file) = handle else {
        return Ok(None);
    };

    let path = file.path().to_path_buf();
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "subtitles.srt".to_string());

    // 2. Validate file size boundary
    let meta = fs::metadata(&path).map_err(|e| {
        format!(
            "Failed to read subtitle metadata from {}: {e}",
            path.display()
        )
    })?;
    if meta.len() > MAX_SUBTITLE_FILE_SIZE as u64 {
        return Err(format!(
            "Subtitle file size ({} bytes) exceeds the maximum allowed limit of 10 MB.",
            meta.len()
        ));
    }

    // 3. Register authorization in registry with SubtitleFileDialog provenance
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Path canonicalization failed: {e}"))?;
    library
        .session
        .authorization_registry()
        .authorize(
            canonical,
            SourceProvenance::SubtitleFileDialog,
            meta.len(),
            meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
        )
        .await;

    // 4. Read UTF-8 content
    let raw_bytes = fs::read(&path).map_err(|e| format!("Failed to read subtitle content: {e}"))?;
    let content = std::str::from_utf8(&raw_bytes)
        .map_err(|e| format!("Invalid UTF-8 encoding in subtitle file: {e}"))?;

    // 5. Detect format
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let detected_format = SubtitleFormat::from_extension(ext);

    let language = infer_language_from_filename(&file_name);
    let display_name = format!("{file_name} ({})", language.to_uppercase());

    let (mut track, warnings) = parse_subtitles_from_str(
        content,
        detected_format,
        None,
        Some(language),
        Some(display_name),
    )
    .map_err(|e| format!("Failed to parse subtitle file: {e}"))?;

    // 6. Inspect known media duration if associated asset is present
    let known_duration_us = if let Some(ref asset_id) = associated_asset_id {
        let assets = library.session.get_assets().await;
        assets.iter().find(|a| &a.id == asset_id).and_then(|a| {
            a.metadata
                .duration_micros()
                .map(|micros| MediaTimeUs::from_micros(micros as i64))
        })
    } else {
        None
    };

    let findings = validate_track(&track, known_duration_us);

    let mut doc = SubtitleDocument::new(associated_asset_id);
    track.id = uuid::Uuid::new_v4().to_string();
    doc.add_track(track.clone());

    Ok(Some(SubtitleImportResponse {
        document: doc,
        track,
        findings,
        warnings,
        file_name,
        file_size_bytes: meta.len(),
        file_path: Some(path.to_string_lossy().to_string()),
    }))
}

/// Direct parsing of subtitle string content without invoking native file pickers.
#[tauri::command]
pub async fn parse_subtitle_content(
    content: String,
    format: Option<SubtitleFormat>,
    language: Option<String>,
    display_name: Option<String>,
    associated_asset_id: Option<String>,
    library: State<'_, MediaLibraryState>,
) -> Result<SubtitleImportResponse, String> {
    if content.len() > MAX_SUBTITLE_FILE_SIZE {
        return Err("Subtitle content exceeds maximum 10 MB limit.".to_string());
    }

    let (mut track, warnings) = parse_subtitles_from_str(
        &content,
        format,
        None,
        language.clone(),
        display_name.clone(),
    )
    .map_err(|e| e.to_string())?;

    let known_duration_us = if let Some(ref asset_id) = associated_asset_id {
        let assets = library.session.get_assets().await;
        assets.iter().find(|a| &a.id == asset_id).and_then(|a| {
            a.metadata
                .duration_micros()
                .map(|micros| MediaTimeUs::from_micros(micros as i64))
        })
    } else {
        None
    };

    let findings = validate_track(&track, known_duration_us);

    let mut doc = SubtitleDocument::new(associated_asset_id);
    track.id = uuid::Uuid::new_v4().to_string();
    doc.add_track(track.clone());

    Ok(SubtitleImportResponse {
        document: doc,
        track,
        findings,
        warnings,
        file_name: "imported_subtitles".to_string(),
        file_size_bytes: content.len() as u64,
        file_path: None,
    })
}

/// Exports a subtitle track to a user-selected file destination with safe no-clobber protection.
#[tauri::command]
pub async fn export_subtitle_track(
    track: SubtitleTrack,
    format: SubtitleFormat,
    default_filename: Option<String>,
) -> Result<Option<SubtitleExportResponse>, String> {
    let mut warnings = Vec::new();

    // Check compatibility warnings
    if format == SubtitleFormat::Srt && track.cues.iter().any(|c| c.settings.is_some()) {
        warnings.push(
            "SRT format does not support WebVTT cue settings (positioning and alignments were omitted)."
                .to_string(),
        );
    }

    let ext = format.extension();
    let default_name = default_filename.unwrap_or_else(|| {
        format!(
            "{}.{}",
            track.display_name.to_lowercase().replace(' ', "_"),
            ext
        )
    });

    let (filter_name, filter_ext) = match format {
        SubtitleFormat::Srt => ("SubRip Subtitle (*.srt)", "srt"),
        SubtitleFormat::WebVtt => ("WebVTT Subtitle (*.vtt)", "vtt"),
        SubtitleFormat::NativeProject => ("OpenTake Subtitle Track (*.json)", "json"),
    };

    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Export Subtitle Track")
        .set_file_name(&default_name)
        .add_filter(filter_name, &[filter_ext]);

    let handle = dialog.save_file().await;
    let Some(dest) = handle else {
        return Ok(None);
    };

    let target_path = dest.path().to_path_buf();

    let output_str = export_subtitles_to_string(&track, format).map_err(|e| e.to_string())?;

    // Atomic temporary write + rename to guarantee crash-safety
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = parent.join(format!(
        ".{}.tmp.{}",
        target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("export"),
        uuid::Uuid::new_v4().as_simple()
    ));

    fs::write(&tmp_path, output_str.as_bytes())
        .map_err(|e| format!("Failed writing temporary export file: {e}"))?;

    fs::rename(&tmp_path, &target_path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed finalizing export file: {e}")
    })?;

    let bytes_written = output_str.len() as u64;

    Ok(Some(SubtitleExportResponse {
        exported_path: target_path.to_string_lossy().to_string(),
        bytes_written,
        format,
        warnings,
    }))
}

/// Saves an entire SubtitleDocument project atomically to user-chosen storage.
#[tauri::command]
pub async fn save_subtitle_project(
    document: SubtitleDocument,
    default_filename: Option<String>,
) -> Result<Option<SubtitleProjectSaveResponse>, String> {
    let default_name = default_filename.unwrap_or_else(|| "project_subtitles.omwsub".to_string());

    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Save Subtitle Project")
        .set_file_name(&default_name)
        .add_filter(
            "OpenTake Subtitle Project (*.omwsub, *.json)",
            &["omwsub", "json"],
        );

    let handle = dialog.save_file().await;
    let Some(dest) = handle else {
        return Ok(None);
    };

    let target_path = dest.path().to_path_buf();
    save_subtitle_document(&document, &target_path).map_err(|e| e.to_string())?;

    let meta = fs::metadata(&target_path)
        .map_err(|e| format!("Failed to read saved project metadata: {e}"))?;

    Ok(Some(SubtitleProjectSaveResponse {
        project_path: target_path.to_string_lossy().to_string(),
        bytes_written: meta.len(),
    }))
}

/// Reopens an existing SubtitleDocument project from disk.
#[tauri::command]
pub async fn open_subtitle_project(
    library: State<'_, MediaLibraryState>,
) -> Result<Option<SubtitleProjectOpenResponse>, String> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Open Subtitle Project")
        .add_filter(
            "OpenTake Subtitle Project (*.omwsub, *.json)",
            &["omwsub", "json"],
        )
        .add_filter("All Files (*.*)", &["*"]);

    let handle = dialog.pick_file().await;
    let Some(file) = handle else {
        return Ok(None);
    };

    let path = file.path().to_path_buf();
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "project.omwsub".to_string());

    let document = load_subtitle_document(&path).map_err(|e| e.to_string())?;

    // Validate document against associated media asset if registered
    let known_duration_us = if let Some(ref asset_id) = document.associated_media_asset_id {
        let assets = library.session.get_assets().await;
        assets.iter().find(|a| &a.id == asset_id).and_then(|a| {
            a.metadata
                .duration_micros()
                .map(|micros| MediaTimeUs::from_micros(micros as i64))
        })
    } else {
        None
    };

    let findings = validate_document(&document, known_duration_us);

    Ok(Some(SubtitleProjectOpenResponse {
        document,
        findings,
        file_name,
        file_path: path.to_string_lossy().to_string(),
    }))
}

/// Runs validation rules against a given SubtitleDocument.
#[tauri::command]
pub fn validate_subtitle_document_command(
    document: SubtitleDocument,
    media_duration_us: Option<i64>,
) -> Vec<SubtitleFinding> {
    let dur = media_duration_us.map(MediaTimeUs::from_micros);
    validate_document(&document, dur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentake_core::localization::subtitles::SubtitleCue;

    #[test]
    fn test_infer_language_from_filename() {
        assert_eq!(infer_language_from_filename("video.en.srt"), "en");
        assert_eq!(infer_language_from_filename("clip.bn.vtt"), "bn");
        assert_eq!(infer_language_from_filename("movie.spa.srt"), "spa");
        assert_eq!(infer_language_from_filename("plain_file.srt"), "und");
    }

    #[test]
    fn test_validate_subtitle_document_command() {
        let mut doc = SubtitleDocument::new(Some("test-asset".to_string()));
        let mut track = SubtitleTrack::new("tr-1", "en", "English", SubtitleFormat::Srt);
        track.cues.push(
            SubtitleCue::new(
                "c-1",
                MediaTimeUs::from_micros(1_000_000),
                MediaTimeUs::from_micros(5_000_000),
                "Valid cue",
            )
            .unwrap(),
        );
        doc.add_track(track);

        let findings = validate_subtitle_document_command(doc, Some(10_000_000));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_subtitle_import_response_serialization() {
        let doc = SubtitleDocument::new(None);
        let track = SubtitleTrack::new("tr-1", "bn", "Bengali", SubtitleFormat::Srt);
        let resp = SubtitleImportResponse {
            document: doc,
            track,
            findings: vec![],
            warnings: vec!["Notice: Cleaned".to_string()],
            file_name: "test.bn.srt".to_string(),
            file_size_bytes: 120,
            file_path: Some("/path/to/test.bn.srt".to_string()),
        };

        let json = serde_json::to_string(&resp).expect("Serialization must succeed");
        assert!(json.contains("\"fileName\":\"test.bn.srt\""));
        assert!(json.contains("\"fileSizeBytes\":120"));
        assert!(json.contains("\"filePath\":\"/path/to/test.bn.srt\""));
    }
}
