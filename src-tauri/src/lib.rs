pub mod commands;
pub mod protocol;

pub use commands::{
    cancel_media_diagnostics, cancel_media_recovery, cancel_preview_proxy,
    cancel_waveform_analysis, clear_media_library, export_diagnostic_report,
    export_recovery_report, finalize_recovered_asset, get_app_info, get_diagnostic_report,
    get_media_engine_status, get_media_library_assets, get_media_playback_url, get_proxy_status,
    get_recovery_status, get_waveform_segment, get_waveform_status, import_files_by_path,
    pick_and_import_files, pick_and_import_repair_asset, remove_media_library_asset,
    request_preview_proxy, request_waveform_analysis, run_media_diagnostics, start_media_recovery,
    AppInfo, MediaEngineState, MediaLibraryState, PlaybackState, RecoveryJobStatus,
    RepairDiagnosticsState, WaveformState,
};

/// Desktop application entry point.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::MediaEngineState::default())
        .manage(commands::MediaLibraryState::default())
        .manage(commands::PlaybackState::default())
        .manage(commands::WaveformState::default())
        .manage(commands::RepairDiagnosticsState::default())
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
            commands::repair::run_media_diagnostics,
            commands::repair::cancel_media_diagnostics,
            commands::repair::get_diagnostic_report,
            commands::repair::export_diagnostic_report,
            commands::repair::pick_and_import_repair_asset,
            commands::repair::start_media_recovery,
            commands::repair::get_recovery_status,
            commands::repair::cancel_media_recovery,
            commands::repair::finalize_recovered_asset,
            commands::repair::export_recovery_report,
            commands::repair::pick_and_import_reference_asset,
            commands::repair::analyze_reference_compatibility,
            commands::repair::start_mp4_reconstruction,
            commands::repair::get_reconstruction_status,
            commands::repair::cancel_mp4_reconstruction,
            commands::repair::get_reconstruction_capabilities,
            commands::repair::export_reconstruction_report,
            commands::repair::register_recovery_candidate_preview,
            commands::repair::get_recovery_review,
            commands::repair::run_advanced_output_validation,
            commands::repair::export_recovery_review_report,
            commands::repair::get_recovery_history,
            commands::repair::clear_recovery_history,
            commands::subtitles::pick_and_import_subtitle_file,
            commands::subtitles::parse_subtitle_content,
            commands::subtitles::export_subtitle_track,
            commands::subtitles::save_subtitle_project,
            commands::subtitles::open_subtitle_project,
            commands::subtitles::validate_subtitle_document_command,
            commands::subtitles::apply_subtitle_operation_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenTake desktop application");
}
