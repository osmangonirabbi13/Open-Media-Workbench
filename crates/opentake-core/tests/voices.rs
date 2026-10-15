use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

use opentake_core::ai::catalog::find_catalog_model;
use opentake_core::ai::models::AiModelInstallation;
use opentake_core::ai::storage::AiStorageManager;
use opentake_core::ai::tts::engine::MockTtsEngine;
use opentake_core::ai::voices::assignments::{
    assign_voice_to_track, get_project_voice_assignments, remove_track_voice_assignment,
    set_project_default_voice,
};
use opentake_core::ai::voices::catalog::{
    filter_voices_by_language, find_voice_by_id, get_supported_languages, get_voice_catalog,
};
use opentake_core::ai::voices::compatibility::validate_speaker_id;
use opentake_core::ai::voices::models::{
    VoiceAvailabilityStatus, VoiceComparisonRequest, VoicePreviewRequest,
};
use opentake_core::ai::voices::VoiceLibraryManager;
use opentake_core::localization::subtitles::model::{
    SubtitleDocument, SubtitleFormat, SubtitleTrack,
};

fn setup_test_voice_env(
    temp_dir: &tempfile::TempDir,
) -> (VoiceLibraryManager, Arc<AiStorageManager>) {
    let storage_dir = temp_dir.path().join("models");
    let cache_dir = temp_dir.path().join("cache");
    let prefs_file = temp_dir.path().join("voice_preferences.json");

    let storage = Arc::new(AiStorageManager::new(&storage_dir).unwrap());

    // Set up mock installation for piper-en-lessac-medium
    let manifest = find_catalog_model("piper-en-lessac-medium").unwrap();
    let model_dir = storage.resolve_model_dir(&manifest.id).unwrap();
    fs::create_dir_all(&model_dir).unwrap();

    let _model_file = model_dir.join(&manifest.artifact_filename);
    let config_file = model_dir.join("en_US-lessac-medium.onnx.json");

    let home = std::env::var("HOME").unwrap_or_default();
    let real_installed_dir =
        PathBuf::from(&home).join(".local/share/opentake/models/installed/piper-en-lessac-medium");
    let real_model = real_installed_dir.join("en_US-lessac-medium.onnx");
    let real_config = real_installed_dir.join("en_US-lessac-medium.onnx.json");
    let staging_model = temp_dir.path().join("staging_model.part");

    if real_model.exists() && real_config.exists() {
        if fs::hard_link(&real_model, &staging_model).is_err() {
            let _ = fs::copy(&real_model, &staging_model);
        }
        if fs::hard_link(&real_config, &config_file).is_err() {
            let _ = fs::copy(&real_config, &config_file);
        }
    } else {
        fs::write(&staging_model, b"dummy-onnx-model-binary").unwrap();
        fs::write(&config_file, b"{\"audio\": {\"sample_rate\": 22050}}").unwrap();
    }

    let actual_size = fs::metadata(&staging_model).map(|m| m.len()).unwrap_or(32);
    let installation = AiModelInstallation {
        model_id: manifest.id.clone(),
        installed_at: "2026-10-15T00:00:00Z".to_string(),
        installed_version: manifest.version.clone(),
        artifact_filename: manifest.artifact_filename.clone(),
        actual_size_bytes: actual_size,
        verified_sha256: manifest.expected_sha256.clone().unwrap_or_default(),
        installed_artifacts: vec![],
        in_use: false,
    };

    storage
        .commit_installation(&staging_model, installation)
        .unwrap();

    let engine = Arc::new(MockTtsEngine::new(22050));
    let manager = VoiceLibraryManager::new(storage.clone(), engine, cache_dir, prefs_file);

    (manager, storage)
}

#[test]
fn test_voice_catalog_loading_and_filtering() {
    let catalog = get_voice_catalog();
    assert!(!catalog.is_empty());
    assert!(catalog.iter().any(|v| v.id == "piper-en-lessac-medium"));

    let en_voices = filter_voices_by_language("en");
    assert!(!en_voices.is_empty());
    assert!(en_voices
        .iter()
        .all(|v| v.language == "en" || v.locale.starts_with("en-")));

    let es_voices = filter_voices_by_language("es");
    assert!(!es_voices.is_empty());
    assert!(es_voices.iter().all(|v| v.language == "es"));

    let languages = get_supported_languages();
    assert!(languages.contains(&"en".to_string()));
    assert!(languages.contains(&"es".to_string()));
    assert!(languages.contains(&"bn".to_string()));
}

#[test]
fn test_unknown_voice_rejection() {
    let voice = find_voice_by_id("non-existent-voice-id");
    assert!(voice.is_none());
}

#[test]
fn test_voice_compatibility_ready_vs_uninstalled() {
    let temp = tempdir().unwrap();
    let (manager, _storage) = setup_test_voice_env(&temp);

    // 1. Installed Lessac voice should be Ready
    let compat_lessac = manager
        .get_voice_compatibility("piper-en-lessac-medium", None)
        .unwrap();
    assert_eq!(compat_lessac.status, VoiceAvailabilityStatus::Ready);
    assert!(compat_lessac.is_ready);
    assert!(compat_lessac.is_installed);

    // 2. Uninstalled LibriTTS voice should be AvailableForInstall
    let compat_libri = manager
        .get_voice_compatibility("piper-en-libritts-high", None)
        .unwrap();
    assert_eq!(
        compat_libri.status,
        VoiceAvailabilityStatus::AvailableForInstall
    );
    assert!(!compat_libri.is_ready);
    assert!(!compat_libri.is_installed);

    // 3. Language mismatch should return UnsupportedLanguage
    let compat_lang_mismatch = manager
        .get_voice_compatibility("piper-en-lessac-medium", Some("es"))
        .unwrap();
    assert_eq!(
        compat_lang_mismatch.status,
        VoiceAvailabilityStatus::UnsupportedLanguage
    );
    assert!(!compat_lang_mismatch.is_ready);
}

#[test]
fn test_speaker_validation() {
    let single_speaker = find_voice_by_id("piper-en-lessac-medium").unwrap();
    assert_eq!(single_speaker.speaker_count, 1);
    assert_eq!(validate_speaker_id(&single_speaker, None).unwrap(), 0);
    assert_eq!(validate_speaker_id(&single_speaker, Some(0)).unwrap(), 0);
    assert!(validate_speaker_id(&single_speaker, Some(5)).is_err());

    let multi_speaker = find_voice_by_id("piper-en-libritts-high").unwrap();
    assert_eq!(multi_speaker.speaker_count, 904);
    assert_eq!(validate_speaker_id(&multi_speaker, Some(42)).unwrap(), 42);
    assert!(validate_speaker_id(&multi_speaker, Some(9999)).is_err());
}

#[test]
fn test_voice_preview_generation_and_caching() {
    let temp = tempdir().unwrap();
    let (manager, _storage) = setup_test_voice_env(&temp);

    let req = VoicePreviewRequest {
        voice_id: "piper-en-lessac-medium".to_string(),
        custom_text: Some("Preview sample text.".to_string()),
        speaker_id: None,
        settings: None,
    };

    // First call: fresh synthesis (cached = false)
    let res1 = manager.generate_preview(&req).unwrap();
    assert_eq!(res1.voice_id, "piper-en-lessac-medium");
    assert!(!res1.cached);
    assert!(res1.duration_us > 0);
    assert!(PathBuf::from(&res1.staged_audio_path).exists());

    // Second call with same text & settings: served from cache (cached = true)
    let res2 = manager.generate_preview(&req).unwrap();
    assert!(res2.cached);
    assert_eq!(res1.preview_id, res2.preview_id);
    assert_eq!(res1.duration_us, res2.duration_us);

    // Different text: fresh synthesis
    let req_diff = VoicePreviewRequest {
        voice_id: "piper-en-lessac-medium".to_string(),
        custom_text: Some("A completely different sentence.".to_string()),
        speaker_id: None,
        settings: None,
    };
    let res3 = manager.generate_preview(&req_diff).unwrap();
    assert!(!res3.cached);
    assert_ne!(res1.preview_id, res3.preview_id);

    // Clear cache
    let cleared_count = manager.clear_preview_cache().unwrap();
    assert_eq!(cleared_count, 2);

    // After clear, next call synthesizes fresh
    let res4 = manager.generate_preview(&req).unwrap();
    assert!(!res4.cached);
}

#[test]
fn test_voice_comparison() {
    let temp = tempdir().unwrap();
    let (manager, _storage) = setup_test_voice_env(&temp);

    // Test rejection of comparing identical voice
    let req_same = VoiceComparisonRequest {
        voice_id_a: "piper-en-lessac-medium".to_string(),
        voice_id_b: "piper-en-lessac-medium".to_string(),
        text: Some("Comparing identical voices should fail.".to_string()),
    };
    assert!(manager.compare_voices(&req_same).is_err());
}

#[test]
fn test_preferences_favorites_and_recents() {
    let temp = tempdir().unwrap();
    let (manager, _storage) = setup_test_voice_env(&temp);

    // Initial preferences
    let prefs = manager.get_preferences();
    assert!(prefs.favorite_voice_ids.is_empty());
    assert!(prefs.preferred_voice_id.is_none());

    // Toggle favorite
    let is_fav = manager.toggle_favorite("piper-en-lessac-medium").unwrap();
    assert!(is_fav);
    assert_eq!(
        manager.get_preferences().favorite_voice_ids,
        vec!["piper-en-lessac-medium"]
    );

    // Untoggle favorite
    let is_fav_again = manager.toggle_favorite("piper-en-lessac-medium").unwrap();
    assert!(!is_fav_again);
    assert!(manager.get_preferences().favorite_voice_ids.is_empty());

    // Set preferred voice
    manager
        .set_preferred_voice(Some("piper-en-lessac-medium".to_string()))
        .unwrap();
    assert_eq!(
        manager.get_preferences().preferred_voice_id.as_deref(),
        Some("piper-en-lessac-medium")
    );

    // Generate preview updates recents
    let req = VoicePreviewRequest {
        voice_id: "piper-en-lessac-medium".to_string(),
        custom_text: Some("Test recent tracking.".to_string()),
        speaker_id: None,
        settings: None,
    };
    manager.generate_preview(&req).unwrap();
    assert_eq!(
        manager.get_preferences().recent_voice_ids,
        vec!["piper-en-lessac-medium"]
    );
}

#[test]
fn test_project_voice_assignments() {
    let mut doc = SubtitleDocument::new(Some("asset_123".to_string()));
    let track = SubtitleTrack::new("tr_english", "en", "English", SubtitleFormat::Srt);
    doc.add_track(track);

    // Set project default voice
    set_project_default_voice(&mut doc, Some("piper-en-lessac-medium".to_string())).unwrap();
    let assignments = get_project_voice_assignments(&doc);
    assert_eq!(
        assignments.default_voice_id.as_deref(),
        Some("piper-en-lessac-medium")
    );

    // Assign voice to track
    let assignment =
        assign_voice_to_track(&mut doc, "tr_english", "piper-en-lessac-medium", None).unwrap();
    assert_eq!(assignment.track_id, "tr_english");
    assert_eq!(assignment.voice_id, "piper-en-lessac-medium");
    assert_eq!(assignment.language, "en");

    let updated = get_project_voice_assignments(&doc);
    assert_eq!(updated.track_assignments.len(), 1);
    assert_eq!(
        updated.track_assignments[0].voice_id,
        "piper-en-lessac-medium"
    );

    // Remove track assignment
    let removed = remove_track_voice_assignment(&mut doc, "tr_english").unwrap();
    assert!(removed);
    assert!(get_project_voice_assignments(&doc)
        .track_assignments
        .is_empty());

    // Subtitle track cues remain 100% untouched
    assert!(doc.tracks[0].cues.is_empty());
}
