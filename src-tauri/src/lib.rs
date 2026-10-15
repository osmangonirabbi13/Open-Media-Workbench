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
    let ai_state = commands::AiState::default();
    let transcription_state = commands::TranscriptionState::new(ai_state.manager.storage().clone());
    let translation_state = commands::TranslationState::new(ai_state.manager.storage().clone());

    tauri::Builder::default()
        .manage(commands::MediaEngineState::default())
        .manage(commands::MediaLibraryState::default())
        .manage(commands::PlaybackState::default())
        .manage(commands::WaveformState::default())
        .manage(commands::RepairDiagnosticsState::default())
        .manage(commands::StylingState::default())
        .manage(ai_state)
        .manage(transcription_state)
        .manage(translation_state)
        .register_uri_scheme_protocol("opentake-media", |app, req| {
            protocol::media::handle_media_protocol(app, req)
        })
        .invoke_handler(tauri::generate_handler![
            commands::ai::list_ai_models,
            commands::ai::get_ai_model_status,
            commands::ai::get_ai_hardware_info,
            commands::ai::start_ai_model_download,
            commands::ai::get_ai_model_download_status,
            commands::ai::cancel_ai_model_download,
            commands::ai::verify_ai_model,
            commands::ai::remove_ai_model,
            commands::ai::get_ai_storage_usage,
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
            commands::transcription::get_whisper_runtime_capabilities,
            commands::transcription::select_whisper_runtime,
            commands::transcription::start_offline_transcription,
            commands::transcription::get_offline_transcription_status,
            commands::transcription::cancel_offline_transcription,
            commands::transcription::get_offline_transcription_result,
            commands::transcription::apply_transcription_result_to_subtitles,
            commands::alignment::get_word_alignment_result,
            commands::alignment::segment_words_to_cues,
            commands::alignment::refine_subtitle_cues,
            commands::alignment::update_word_timing_in_subtitle_cue,
            commands::alignment::apply_smart_segmented_subtitles,
            commands::styling::get_subtitle_style_presets,
            commands::styling::save_custom_subtitle_style_preset,
            commands::styling::delete_custom_subtitle_style_preset,
            commands::styling::get_system_fonts,
            commands::styling::generate_ass_script_preview,
            commands::styling::start_subtitle_burn_in_export,
            commands::styling::get_subtitle_burn_in_status,
            commands::styling::cancel_subtitle_burn_in_export,
            commands::styling::finalize_burn_in_export,
            commands::translation::get_translation_capabilities,
            commands::translation::list_translation_language_pairs,
            commands::translation::select_translation_runtime,
            commands::translation::start_offline_translation,
            commands::translation::get_offline_translation_status,
            commands::translation::cancel_offline_translation,
            commands::translation::get_offline_translation_result,
            commands::translation::apply_translation_result_to_subtitles,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenTake desktop application");
}
