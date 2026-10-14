pub mod commands;
pub mod protocol;

pub use commands::{
    cancel_preview_proxy, cancel_waveform_analysis, clear_media_library, get_app_info,
    get_media_engine_status, get_media_library_assets, get_media_playback_url, get_proxy_status,
    get_waveform_segment, get_waveform_status, import_files_by_path, pick_and_import_files,
    remove_media_library_asset, request_preview_proxy, request_waveform_analysis, AppInfo,
    MediaEngineState, MediaLibraryState, PlaybackState, WaveformState,
};

/// Desktop application entry point.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::MediaEngineState::default())
        .manage(commands::MediaLibraryState::default())
        .manage(commands::PlaybackState::default())
        .manage(commands::WaveformState::default())
        .register_uri_scheme_protocol("opentake-media", |app, req| {
            protocol::media::handle_media_protocol(app, req)
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info::get_app_info,
            commands::media_engine::get_media_engine_status,
            commands::media_import::pick_and_import_files,
            commands::media_import::import_files_by_path,
            commands::media_import::get_media_library_assets,
            commands::media_import::remove_media_library_asset,
            commands::media_import::clear_media_library,
            commands::playback::get_media_playback_url,
            commands::playback::request_preview_proxy,
            commands::playback::cancel_preview_proxy,
            commands::playback::get_proxy_status,
            commands::waveform::get_waveform_status,
            commands::waveform::request_waveform_analysis,
            commands::waveform::cancel_waveform_analysis,
            commands::waveform::get_waveform_segment,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenTake desktop application");
}
