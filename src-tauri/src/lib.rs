pub mod commands;

pub use commands::{get_app_info, AppInfo};

/// Desktop application entry point.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running OpenTake desktop application");
}
