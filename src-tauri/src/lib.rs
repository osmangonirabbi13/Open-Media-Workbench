pub mod commands;

pub use commands::{get_app_info, get_media_engine_status, AppInfo, MediaEngineState};

/// Desktop application entry point.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::MediaEngineState::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_info::get_app_info,
            commands::media_engine::get_media_engine_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenTake desktop application");
}
