pub mod ai;
pub mod alignment;
pub mod app_info;
pub mod media_engine;
pub mod media_import;
pub mod playback;
pub mod repair;
pub mod styling;
pub mod subtitles;
pub mod transcription;
pub mod translation;
pub mod waveform;

pub use ai::{
    cancel_ai_model_download, get_ai_hardware_info, get_ai_model_download_status,
    get_ai_model_status, get_ai_storage_usage, list_ai_models, remove_ai_model,
    start_ai_model_download, verify_ai_model, AiState,
};

pub use alignment::{
    apply_smart_segmented_subtitles, get_word_alignment_result, refine_subtitle_cues,
    segment_words_to_cues, update_word_timing_in_subtitle_cue,
};

pub use app_info::{get_app_info, AppInfo};
pub use media_engine::{get_media_engine_status, MediaEngineState};
pub use media_import::{
    clear_media_library, get_media_library_assets, import_files_by_path, pick_and_import_files,
    remove_media_library_asset, ImportBatchResult, ImportFailure, MediaLibraryState,
};
pub use playback::{
    cancel_preview_proxy, get_media_playback_url, get_proxy_status, request_preview_proxy,
    PlaybackInfo, PlaybackState, ProxyStateInfo, ProxyStatusResponse,
};
pub use repair::{
    analyze_reference_compatibility, cancel_media_diagnostics, cancel_media_recovery,
    cancel_mp4_reconstruction, clear_recovery_history, export_diagnostic_report,
    export_reconstruction_report, export_recovery_report, export_recovery_review_report,
    finalize_recovered_asset, get_diagnostic_report, get_reconstruction_capabilities,
    get_reconstruction_status, get_recovery_history, get_recovery_review, get_recovery_status,
    pick_and_import_reference_asset, pick_and_import_repair_asset,
    register_recovery_candidate_preview, run_advanced_output_validation, run_media_diagnostics,
    start_media_recovery, start_mp4_reconstruction, ReconstructionJobStatus, RecoveryJobStatus,
    RepairDiagnosticsState,
};
pub use subtitles::{
    apply_subtitle_operation_command, export_subtitle_track, open_subtitle_project,
    parse_subtitle_content, pick_and_import_subtitle_file, save_subtitle_project,
    validate_subtitle_document_command, SubtitleExportResponse, SubtitleImportResponse,
    SubtitleOperationResponse, SubtitleProjectOpenResponse, SubtitleProjectSaveResponse,
};

pub use transcription::{
    apply_transcription_result_to_subtitles, cancel_offline_transcription,
    get_offline_transcription_result, get_offline_transcription_status,
    get_whisper_runtime_capabilities, select_whisper_runtime, TranscriptionState,
};

pub use styling::{
    cancel_subtitle_burn_in_export, delete_custom_subtitle_style_preset, finalize_burn_in_export,
    generate_ass_script_preview, get_subtitle_burn_in_status, get_subtitle_style_presets,
    get_system_fonts, save_custom_subtitle_style_preset, start_subtitle_burn_in_export,
    StylingState,
};

pub use translation::{
    apply_translation_result_to_subtitles, cancel_offline_translation,
    get_offline_translation_result, get_offline_translation_status, get_translation_capabilities,
    list_translation_language_pairs, select_translation_runtime, start_offline_translation,
    TranslationRuntimeCapabilities, TranslationState,
};

pub use waveform::{
    cancel_waveform_analysis, get_waveform_segment, get_waveform_status, request_waveform_analysis,
    WaveformSegmentResponse, WaveformState, WaveformStatusInfo, WaveformStatusResponse,
};
