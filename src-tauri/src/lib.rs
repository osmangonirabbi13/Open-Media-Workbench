pub mod commands;

pub use commands::{
    clear_media_library, get_app_info, get_media_engine_status, get_media_library_assets,
    import_files_by_path, pick_and_import_files, remove_media_library_asset, AppInfo,
    MediaEngineState, MediaLibraryState,
};

/// Desktop application entry point.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::MediaEngineState::default())
        .manage(commands::MediaLibraryState::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_info::get_app_info,
            commands::media_engine::get_media_engine_status,
            commands::media_import::pick_and_import_files,
            commands::media_import::import_files_by_path,
            commands::media_import::get_media_library_assets,
            commands::media_import::remove_media_library_asset,
            commands::media_import::clear_media_library,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenTake desktop application");
}
