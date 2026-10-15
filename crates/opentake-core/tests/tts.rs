use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

use opentake_core::ai::catalog::find_catalog_model;
use opentake_core::ai::models::AiModelInstallation;
use opentake_core::ai::storage::AiStorageManager;
use opentake_core::ai::tts::audio::{concatenate_wav_files, validate_wav_file};
use opentake_core::ai::tts::engine::{MockTtsEngine, TtsEngine};
use opentake_core::ai::tts::manager::TtsManager;
use opentake_core::ai::tts::models::{
    TtsProgressStage, TtsRequest, TtsSource, TtsSynthesisInput, TtsVoiceSettings,
};
use opentake_core::ai::tts::segments::{chunk_script_text, extract_cues_for_tts};
use opentake_core::editing::MediaTimeUs;
use opentake_core::localization::subtitles::model::{SubtitleCue, SubtitleFormat, SubtitleTrack};

fn setup_test_manager(temp_dir: &tempfile::TempDir) -> (TtsManager, Arc<AiStorageManager>) {
    let storage = Arc::new(AiStorageManager::new(temp_dir.path()).unwrap());

    // Install mock model for piper-en-lessac-medium in test storage
    let manifest = find_catalog_model("piper-en-lessac-medium").unwrap();
    let model_dir = storage.resolve_model_dir(&manifest.id).unwrap();
    fs::create_dir_all(&model_dir).unwrap();

    let model_file = model_dir.join(&manifest.artifact_filename);
    fs::write(&model_file, b"fake-onnx-model-weights-content").unwrap();

    let config_file = model_dir.join("en_US-lessac-medium.onnx.json");
    fs::write(&config_file, b"{\"audio\": {\"sample_rate\": 22050}}").unwrap();

    let installation = AiModelInstallation {
        model_id: manifest.id.clone(),
        installed_at: "2026-10-15T00:00:00Z".to_string(),
        installed_version: manifest.version.clone(),
        artifact_filename: manifest.artifact_filename.clone(),
        actual_size_bytes: 32,
        verified_sha256: "fake-sha".to_string(),
        installed_artifacts: vec![],
        in_use: false,
    };

    let fake_staging = temp_dir.path().join("fake.part");
    fs::write(&fake_staging, b"dummy").unwrap();
    storage
        .commit_installation(&fake_staging, installation)
        .unwrap();

    let engine = Arc::new(MockTtsEngine::new(22050));
    let manager = TtsManager::new(storage.clone(), engine);

    (manager, storage)
}

#[test]
fn test_mock_tts_engine_synthesizes_valid_wav() {
    let temp = tempdir().unwrap();
    let out_wav = temp.path().join("speech.wav");

    let engine = MockTtsEngine::new(22050);
    let input = TtsSynthesisInput {
        text: "Welcome to Open Media Workbench.".to_string(),
        voice_id: "piper-en-lessac-medium".to_string(),
        model_path: PathBuf::from("/mock/model.onnx"),
        config_path: PathBuf::from("/mock/model.onnx.json"),
        settings: TtsVoiceSettings::default(),
    };

    let meta = engine.synthesize(&input, &out_wav, None).unwrap();
    assert_eq!(meta.sample_rate, 22050);
    assert_eq!(meta.channel_count, 1);
    assert_eq!(meta.bits_per_sample, 16);
    assert!(meta.duration_us > 400_000);
    assert!(!meta.is_silent);

    let inspected = validate_wav_file(&out_wav).unwrap();
    assert_eq!(inspected.sample_rate, 22050);
    assert_eq!(inspected.channel_count, 1);
    assert!(!inspected.is_silent);
}

#[test]
fn test_script_chunking_and_bengali_preservation() {
    let script = "Open Media Workbench is fast. It runs locally! এটি কি কাজ করে? হ্যাঁ, চমৎকার!";
    let chunks = chunk_script_text(script);
    assert_eq!(chunks.len(), 4);
    assert_eq!(chunks[0], "Open Media Workbench is fast.");
    assert_eq!(chunks[1], "It runs locally!");
    assert_eq!(chunks[2], "এটি কি কাজ করে?");
    assert_eq!(chunks[3], "হ্যাঁ, চমৎকার!");
}

#[test]
fn test_subtitle_cue_extraction() {
    let mut track = SubtitleTrack::new("tr_test", "en", "English", SubtitleFormat::Srt);
    track.cues.push(
        SubtitleCue::new(
            "c1",
            MediaTimeUs(1_000_000),
            MediaTimeUs(3_000_000),
            "First cue",
        )
        .unwrap(),
    );
    track.cues.push(
        SubtitleCue::new(
            "c2",
            MediaTimeUs(3_500_000),
            MediaTimeUs(5_000_000),
            "Second cue",
        )
        .unwrap(),
    );
    track.cues.push(
        SubtitleCue::new("c3", MediaTimeUs(6_000_000), MediaTimeUs(8_000_000), "   ").unwrap(),
    ); // empty

    let all_cues = extract_cues_for_tts(&track, None);
    assert_eq!(all_cues.len(), 2);
    assert_eq!(all_cues[0].cue_id, "c1");
    assert_eq!(all_cues[1].cue_id, "c2");

    let filtered = extract_cues_for_tts(&track, Some(&["c2".to_string()]));
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].cue_id, "c2");
}

#[test]
fn test_tts_voice_settings_validation() {
    let mut settings = TtsVoiceSettings::default();
    assert!(settings.validate().is_ok());

    settings.speed_scale = 0.1; // out of range
    assert!(settings.validate().is_err());

    settings.speed_scale = 1.0;
    settings.noise_scale = 5.0; // out of range
    assert!(settings.validate().is_err());
}

#[tokio::test]
async fn test_script_synthesis_lifecycle_and_model_locking() {
    let temp = tempdir().unwrap();
    let (manager, storage) = setup_test_manager(&temp);

    let request = TtsRequest {
        source: TtsSource::Script {
            text: "Sentence one. Sentence two.".to_string(),
        },
        voice_id: "piper-en-lessac-medium".to_string(),
        language: "en-US".to_string(),
        settings: TtsVoiceSettings::default(),
    };

    let job_id = manager.start_synthesis(request, None).unwrap();
    assert!(job_id.starts_with("tts_"));

    // Poll until completed
    let mut finished = false;
    for _ in 0..50 {
        if let Some(status) = manager.get_status(&job_id) {
            if status.stage == TtsProgressStage::Completed {
                finished = true;
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    assert!(finished);

    // Verify lock was released
    assert!(!storage.is_locked("piper-en-lessac-medium"));

    let result = manager.get_result(&job_id).unwrap();
    assert_eq!(result.segments.len(), 2);
    assert_eq!(result.voice_id, "piper-en-lessac-medium");
    assert!(result.total_duration_us > 800_000);

    // Test export of single segment
    let export_single = temp.path().join("segment_0.wav");
    manager
        .export_audio(&job_id, Some(&result.segments[0].id), &export_single)
        .unwrap();
    assert!(export_single.is_file());
    let meta_single = validate_wav_file(&export_single).unwrap();
    assert_eq!(meta_single.sample_rate, 22050);

    // Test export of entire script concatenated
    let export_all = temp.path().join("full_speech.wav");
    manager.export_audio(&job_id, None, &export_all).unwrap();
    assert!(export_all.is_file());
    let meta_all = validate_wav_file(&export_all).unwrap();
    assert_eq!(meta_all.sample_rate, 22050);
    assert!(meta_all.duration_us >= meta_single.duration_us);
}

#[tokio::test]
async fn test_subtitle_synthesis_and_duration_difference() {
    let temp = tempdir().unwrap();
    let (manager, _storage) = setup_test_manager(&temp);

    let mut track = SubtitleTrack::new("tr_sub", "en", "English", SubtitleFormat::Srt);
    // 2.0s duration cue
    track.cues.push(
        SubtitleCue::new(
            "cue_short",
            MediaTimeUs(1_000_000),
            MediaTimeUs(3_000_000),
            "This is a longer sentence intended to produce audio longer than two seconds.",
        )
        .unwrap(),
    );

    let request = TtsRequest {
        source: TtsSource::SubtitleTrack {
            track_id: "tr_sub".to_string(),
            cue_ids: None,
        },
        voice_id: "piper-en-lessac-medium".to_string(),
        language: "en-US".to_string(),
        settings: TtsVoiceSettings::default(),
    };

    let job_id = manager.start_synthesis(request, Some(track)).unwrap();

    let mut finished = false;
    for _ in 0..50 {
        if let Some(status) = manager.get_status(&job_id) {
            if status.stage == TtsProgressStage::Completed {
                finished = true;
                break;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    assert!(finished);

    let result = manager.get_result(&job_id).unwrap();
    assert_eq!(result.segments.len(), 1);
    let seg = &result.segments[0];
    assert_eq!(seg.source_cue_id.as_deref(), Some("cue_short"));
    assert_eq!(seg.cue_start_us, Some(1_000_000));
    assert_eq!(seg.cue_end_us, Some(3_000_000));
    assert!(seg.timing_difference_us.is_some());
}

#[tokio::test]
async fn test_cancellation_releases_model_lock() {
    let temp = tempdir().unwrap();
    let (manager, storage) = setup_test_manager(&temp);

    let request = TtsRequest {
        source: TtsSource::Script {
            text: "Long long script text sentence one. Sentence two. Sentence three.".to_string(),
        },
        voice_id: "piper-en-lessac-medium".to_string(),
        language: "en-US".to_string(),
        settings: TtsVoiceSettings::default(),
    };

    let job_id = manager.start_synthesis(request, None).unwrap();
    // Cancel immediately
    let _ = manager.cancel_job(&job_id);

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify lock released
    assert!(!storage.is_locked("piper-en-lessac-medium"));
}

#[test]
fn test_wav_validation_rejects_corrupted_header() {
    let temp = tempdir().unwrap();
    let bad_file = temp.path().join("corrupted.wav");
    fs::write(&bad_file, b"NOT_A_WAV_HEADER_DATA_CONTENT_HERE_AND_MORE").unwrap();

    let res = validate_wav_file(&bad_file);
    assert!(res.is_err());
}

#[test]
fn test_wav_concatenation_mismatched_sample_rates() {
    let temp = tempdir().unwrap();
    let engine_22k = MockTtsEngine::new(22050);
    let engine_16k = MockTtsEngine::new(16000);

    let f1 = temp.path().join("f1.wav");
    let f2 = temp.path().join("f2.wav");
    let out = temp.path().join("out.wav");

    engine_22k
        .generate_synthetic_wav("Sample one", &f1)
        .unwrap();
    engine_16k
        .generate_synthetic_wav("Sample two", &f2)
        .unwrap();

    let res = concatenate_wav_files(&[&f1, &f2], &out);
    assert!(res.is_err());
}

#[test]
fn test_real_piper_tts_if_installed() {
    let home = std::env::var("HOME").unwrap_or_default();
    let piper_path = PathBuf::from(&home).join(".local/share/opentake/bin/piper");
    let model_path = PathBuf::from(&home).join(
        ".local/share/opentake/models/installed/piper-en-lessac-medium/en_US-lessac-medium.onnx",
    );
    let config_path = PathBuf::from(&home)
        .join(".local/share/opentake/models/installed/piper-en-lessac-medium/en_US-lessac-medium.onnx.json");

    if !piper_path.exists() || !model_path.exists() || !config_path.exists() {
        eprintln!(
            "Piper or model not found in ~/.local/share/opentake, skipping real inference test"
        );
        return;
    }

    use opentake_core::ai::tts::engine::SystemPiperCliEngine;
    let engine = SystemPiperCliEngine::new(Some(piper_path));
    let temp = tempdir().unwrap();
    let out_wav = temp.path().join("real_welcome.wav");

    let input = TtsSynthesisInput {
        text: "Welcome to Open Media Workbench.".to_string(),
        voice_id: "piper-en-lessac-medium".to_string(),
        model_path,
        config_path,
        settings: TtsVoiceSettings::default(),
    };

    let meta = engine
        .synthesize(&input, &out_wav, None)
        .expect("Real Piper synthesis failed");
    assert_eq!(meta.sample_rate, 22050);
    assert_eq!(meta.channel_count, 1);
    assert_eq!(meta.bits_per_sample, 16);
    assert!(meta.duration_us > 1_500_000 && meta.duration_us < 3_000_000);
    assert!(!meta.is_silent);
    assert!(meta.sample_count > 30_000);

    let inspected = validate_wav_file(&out_wav).expect("Validation of real WAV failed");
    assert_eq!(inspected.sample_rate, 22050);
    assert_eq!(inspected.channel_count, 1);
    assert!(!inspected.is_silent);
}
