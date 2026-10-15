//! Tauri IPC commands for Subtitle Styling, ASS Generation, and Burn-In Export.

use opentake_core::localization::styling::{
    discover_system_fonts, generate_ass_script, BurnInExportManager, FontFamilyInfo,
    StylePresetManager, SubtitleBurnInRequest, SubtitleBurnInStatus, SubtitleStyle,
    SubtitleStylePreset,
};
use opentake_core::localization::subtitles::model::SubtitleTrack;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct StylingState {
    pub preset_manager: Arc<StylePresetManager>,
    pub export_manager: Arc<BurnInExportManager>,
}

impl Default for StylingState {
    fn default() -> Self {
        let storage_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local/share/open-media-workbench/presets"))
            .unwrap_or_else(|_| std::env::temp_dir().join("open-media-workbench/presets"));

        let _ = std::fs::create_dir_all(&storage_dir);

        Self {
            preset_manager: Arc::new(StylePresetManager::new(storage_dir)),
            export_manager: Arc::new(BurnInExportManager::new()),
        }
    }
}

impl StylingState {
    pub fn new_with_dir(dir: PathBuf) -> Self {
        Self {
            preset_manager: Arc::new(StylePresetManager::new(dir)),
            export_manager: Arc::new(BurnInExportManager::new()),
        }
    }
}

#[tauri::command]
pub async fn get_subtitle_style_presets(
    state: tauri::State<'_, StylingState>,
) -> Result<Vec<SubtitleStylePreset>, String> {
    Ok(state.preset_manager.list_presets())
}

#[tauri::command]
pub async fn save_custom_subtitle_style_preset(
    preset: SubtitleStylePreset,
    state: tauri::State<'_, StylingState>,
) -> Result<(), String> {
    state
        .preset_manager
        .save_custom_preset(preset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_custom_subtitle_style_preset(
    preset_id: String,
    state: tauri::State<'_, StylingState>,
) -> Result<(), String> {
    state
        .preset_manager
        .delete_custom_preset(&preset_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_system_fonts() -> Result<Vec<FontFamilyInfo>, String> {
    Ok(discover_system_fonts())
}

#[tauri::command]
pub async fn generate_ass_script_preview(
    track: SubtitleTrack,
    style: SubtitleStyle,
) -> Result<String, String> {
    generate_ass_script(&track, &style).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_subtitle_burn_in_export(
    request: SubtitleBurnInRequest,
    state: tauri::State<'_, StylingState>,
) -> Result<String, String> {
    state
        .export_manager
        .start_export(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_subtitle_burn_in_status(
    state: tauri::State<'_, StylingState>,
) -> Result<SubtitleBurnInStatus, String> {
    Ok(state.export_manager.get_status().await)
}

#[tauri::command]
pub async fn cancel_subtitle_burn_in_export(
    state: tauri::State<'_, StylingState>,
) -> Result<(), String> {
    state.export_manager.cancel().await;
    Ok(())
}

#[tauri::command]
pub async fn finalize_burn_in_export(
    destination_path: String,
    source_original_path: String,
    state: tauri::State<'_, StylingState>,
) -> Result<String, String> {
    let dest = Path::new(&destination_path);
    let orig = Path::new(&source_original_path);

    let final_path = state
        .export_manager
        .finalize_export(dest, orig)
        .await
        .map_err(|e| e.to_string())?;

    Ok(final_path.to_string_lossy().to_string())
}
