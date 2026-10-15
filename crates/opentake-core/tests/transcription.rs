use std::path::{Path, PathBuf};
use std::sync::Arc;

use opentake_core::ai::catalog::get_curated_model_catalog;
use opentake_core::ai::models::AiModelInstallation;
use opentake_core::ai::storage::AiStorageManager;
use opentake_core::ai::transcription::audio::build_audio_extraction_args;
use opentake_core::ai::transcription::errors::TranscriptionError;
use opentake_core::ai::transcription::models::{
    get_supported_whisper_languages, TranscriptionRequest, TranscriptionResult,
    TranscriptionSegment,
};
use opentake_core::ai::transcription::segments::{
    parse_timestamp_str_to_micros, process_raw_segments, RawTranscriptionSegment,
};
use opentake_core::ai::transcription::subtitle_conversion::{
    append_track_to_document, convert_transcription_result_to_track,
};
use opentake_core::ai::transcription::whisper::{
    discover_whisper_runtime, parse_whisper_json_output, MockWhisperEngine,
};
use opentake_core::ai::transcription::TranscriptionManager;
use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::model::SubtitleDocument;

/// Helper to set up an isolated test storage manager with a pre-installed mock model.
fn setup_test_environment(model_id: &str) -> (tempfile::TempDir, AiStorageManager, PathBuf) {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let storage = AiStorageManager::new(temp_dir.path()).expect("create storage");

    let staging_file = storage
        .create_staging_path(model_id, "init")
        .expect("staging path");
    let mock_bytes = b"MOCK_GGML_WHISPER_WEIGHTS_CONTENT";
    std::fs::write(&staging_file, mock_bytes).expect("write mock staging model");

    let mut hasher = sha2::Sha256::default();
    use sha2::Digest;
    hasher.update(mock_bytes);
    let checksum = format!("{:x}", hasher.finalize());

    let catalog = get_curated_model_catalog();
    let manifest = catalog
        .iter()
        .find(|m| m.id == model_id)
        .expect("model in catalog");

    let installation = AiModelInstallation {
        model_id: model_id.to_string(),
        installed_at: "2026-10-15T00:00:00Z".to_string(),
        installed_version: manifest.version.clone(),
        artifact_filename: manifest.artifact_filename.clone(),
        actual_size_bytes: mock_bytes.len() as u64,
        verified_sha256: checksum,
        in_use: false,
    };

    storage
        .commit_installation(&staging_file, installation)
        .expect("commit installation");

    let artifact_path = storage
        .get_artifact_path(model_id, &manifest.artifact_filename)
        .expect("get artifact path");

    (temp_dir, storage, artifact_path)
}

#[tokio::test]
async fn test_whisper_runtime_discovery_missing() {
    let non_existent = Path::new("/non/existent/whisper-cli");
    let caps = discover_whisper_runtime(Some(non_existent)).await;
    assert!(!caps.is_available);
    assert!(caps.errors.iter().any(|e| e.contains("is invalid")));
}

#[test]
fn test_timestamp_to_micros_conversion_precision() {
    assert_eq!(parse_timestamp_str_to_micros("00:00:00,000").unwrap(), 0);
    assert_eq!(
        parse_timestamp_str_to_micros("00:01:23,456").unwrap(),
        (60 + 23) * 1_000_000 + 456_000
    );
    assert_eq!(
        parse_timestamp_str_to_micros("01:30:15.123").unwrap(),
        (3600 + 1800 + 15) * 1_000_000 + 123_000
    );
    assert!(parse_timestamp_str_to_micros("invalid-timestamp").is_err());
}

#[test]
fn test_segment_sorting_and_duration_invariants() {
    let raw = vec![
        RawTranscriptionSegment {
            start_us: 4_000_000,
            end_us: 6_000_000,
            text: "Second Segment".to_string(),
            detected_language: Some("en".to_string()),
            confidence: Some(0.95),
        },
        RawTranscriptionSegment {
            start_us: 1_000_000,
            end_us: 3_000_000,
            text: "First Segment".to_string(),
            detected_language: Some("en".to_string()),
            confidence: Some(0.98),
        },
        RawTranscriptionSegment {
            start_us: 7_000_000,
            end_us: 7_000_000, // zero duration
            text: "Zero duration segment".to_string(),
            detected_language: Some("en".to_string()),
            confidence: Some(0.90),
        },
    ];

    let processed = process_raw_segments(raw, 0);
    assert_eq!(processed.len(), 3);
    assert_eq!(processed[0].text, "First Segment");
    assert_eq!(processed[0].start_us.as_micros(), 1_000_000);
    assert_eq!(processed[1].text, "Second Segment");
    assert_eq!(processed[1].start_us.as_micros(), 4_000_000);
    assert_eq!(processed[2].text, "Zero duration segment");
    assert!(processed[2].end_us.as_micros() > processed[2].start_us.as_micros());
}

#[test]
fn test_duplicate_segment_deduplication() {
    let raw = vec![
        RawTranscriptionSegment {
            start_us: 1_000_000,
            end_us: 2_000_000,
            text: "Repeated line".to_string(),
            detected_language: Some("en".to_string()),
            confidence: Some(0.99),
        },
        RawTranscriptionSegment {
            start_us: 1_000_000,
            end_us: 2_000_000,
            text: "Repeated line".to_string(),
            detected_language: Some("en".to_string()),
            confidence: Some(0.99),
        },
    ];

    let processed = process_raw_segments(raw, 0);
    assert_eq!(processed.len(), 1);
    assert_eq!(processed[0].text, "Repeated line");
}

#[test]
fn test_chunk_offset_mapping() {
    let raw = vec![RawTranscriptionSegment {
        start_us: 500_000,
        end_us: 2_000_000,
        text: "Chunk text".to_string(),
        detected_language: Some("en".to_string()),
        confidence: Some(0.95),
    }];

    // Offset of 10 seconds (10,000,000 µs)
    let processed = process_raw_segments(raw, 10_000_000);
    assert_eq!(processed[0].start_us.as_micros(), 10_500_000);
    assert_eq!(processed[0].end_us.as_micros(), 12_000_000);
}

#[test]
fn test_bengali_unicode_preservation() {
    let bengali_text = "ওপেন মিডিয়া ওয়ার্কবেঞ্চ — অফলাইন ট্রান্সক্রিপশন ইঞ্জিন।";
    let raw = vec![RawTranscriptionSegment {
        start_us: 0,
        end_us: 3_000_000,
        text: bengali_text.to_string(),
        detected_language: Some("bn".to_string()),
        confidence: Some(0.97),
    }];

    let processed = process_raw_segments(raw, 0);
    assert_eq!(processed[0].text, bengali_text);
}

#[test]
fn test_parse_whisper_json_output() {
    let json_sample = r#"{
        "result": { "language": "en" },
        "transcription": [
            {
                "timestamps": { "from": "00:00:01,000", "to": "00:00:04,500" },
                "offsets": { "from": 1000, "to": 4500 },
                "text": " Hello world from whisper."
            }
        ]
    }"#;

    let parsed = parse_whisper_json_output(json_sample, "auto").unwrap();
    assert_eq!(parsed.detected_language, Some("en".to_string()));
    assert_eq!(parsed.segments.len(), 1);
    assert_eq!(parsed.segments[0].start_us, 1_000_000);
    assert_eq!(parsed.segments[0].end_us, 4_500_000);
    assert_eq!(parsed.segments[0].text.trim(), "Hello world from whisper.");
}

#[test]
fn test_audio_extraction_argument_safety() {
    let input = Path::new("/authorized/media/video.mp4");
    let output = Path::new("/staging/audio.wav");

    let args = build_audio_extraction_args(input, output, Some(1_000_000), Some(5_000_000));
    assert_eq!(args[0], "-y");
    assert!(args.contains(&"-vn".to_string()));
    assert!(args.contains(&"-ac".to_string()));
    assert!(args.contains(&"1".to_string()));
    assert!(args.contains(&"-ar".to_string()));
    assert!(args.contains(&"16000".to_string()));
    assert!(args.contains(&"-c:a".to_string()));
    assert!(args.contains(&"pcm_s16le".to_string()));
    assert!(args.contains(&"/staging/audio.wav".to_string()));
}

#[test]
fn test_convert_result_to_subtitle_track_and_document() {
    let result = TranscriptionResult {
        schema_version: 1,
        job_id: "job-100".to_string(),
        asset_id: "asset-100".to_string(),
        model_id: "whisper-tiny".to_string(),
        runtime_version: "whisper.cpp v1.7".to_string(),
        requested_language: "bn".to_string(),
        detected_language: Some("bn".to_string()),
        segments: vec![TranscriptionSegment {
            id: "seg-1".to_string(),
            start_us: MediaTimeUs(500_000),
            end_us: MediaTimeUs(3_500_000),
            text: "বাংলা সাবটাইটেল পরীক্ষা".to_string(),
            detected_language: Some("bn".to_string()),
            confidence: Some(0.99),
        }],
        processing_duration_ms: 850,
        audio_duration_us: Some(4_000_000),
        warnings: vec![],
    };

    let track = convert_transcription_result_to_track(&result, Some("My AI Subtitles")).unwrap();
    assert_eq!(track.display_name, "My AI Subtitles");
    assert_eq!(track.language, "bn");
    assert_eq!(track.cues.len(), 1);
    assert_eq!(track.cues[0].text, "বাংলা সাবটাইটেল পরীক্ষা");

    let existing_doc = SubtitleDocument::new(Some("asset-100".to_string()));
    let updated_doc = append_track_to_document(Some(existing_doc), track);
    assert_eq!(updated_doc.tracks.len(), 1);
    assert_eq!(updated_doc.tracks[0].cues.len(), 1);
}

#[tokio::test]
async fn test_transcription_job_lifecycle_with_mock_engine() {
    let (_dir, storage, _model_file) = setup_test_environment("whisper-tiny");

    // Create a mock engine that reports available
    let mock_engine = Arc::new(MockWhisperEngine::new(true));
    let manager = TranscriptionManager::new(Arc::new(storage.clone()), mock_engine);

    // Create a temporary media file
    let media_temp = tempfile::NamedTempFile::new().expect("create media temp");

    let req = TranscriptionRequest {
        asset_id: "asset-test".to_string(),
        model_id: "whisper-tiny".to_string(),
        language: "en".to_string(),
        output_track_name: None,
        custom_runtime_path: None,
        threads: Some(2),
        start_time_us: None,
        end_time_us: None,
    };

    // Note: Since FFmpeg will attempt to run on `media_temp` (an empty file),
    // start_transcription should submit and update stage, or fail gracefully
    let job_id = manager
        .start_transcription(req, media_temp.path().to_path_buf(), Some(5_000_000))
        .await
        .expect("start transcription");

    assert!(!job_id.is_empty());

    // Give brief time for pipeline execution
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let status = manager.get_job_status(&job_id).await;
    assert!(status.is_some());
}

#[tokio::test]
async fn test_english_only_model_rejects_foreign_language() {
    let (_dir, storage, _) = setup_test_environment("whisper-tiny");
    let mock_engine = Arc::new(MockWhisperEngine::new(true));
    let manager = TranscriptionManager::new(Arc::new(storage), mock_engine);

    // Multilingual model accepts "bn"
    let catalog = get_curated_model_catalog();
    let manifest = catalog.iter().find(|m| m.id == "whisper-tiny").unwrap();
    assert!(manifest.supported_languages.contains(&"bn".to_string()));

    // English-only check
    let req = TranscriptionRequest {
        asset_id: "asset-1".to_string(),
        model_id: "whisper-tiny.en".to_string(), // English-only convention
        language: "bn".to_string(),
        output_track_name: None,
        custom_runtime_path: None,
        threads: None,
        start_time_us: None,
        end_time_us: None,
    };

    let res = manager
        .start_transcription(req, PathBuf::from("/any/path.mp4"), None)
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_model_integrity_failure_rejected() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let storage = AiStorageManager::new(temp_dir.path()).expect("create storage");

    // Create a model file with corrupted bytes (doesn't start with MOCK_GGML)
    let staging_file = storage
        .create_staging_path("whisper-tiny", "corrupt")
        .expect("staging");
    std::fs::write(&staging_file, b"CORRUPTED_MODEL_PAYLOAD").expect("write corrupt");

    let installation = AiModelInstallation {
        model_id: "whisper-tiny".to_string(),
        installed_at: "2026-10-15T00:00:00Z".to_string(),
        installed_version: "v1".to_string(),
        artifact_filename: "ggml-tiny.bin".to_string(),
        actual_size_bytes: 23,
        verified_sha256: "invalid_hash".to_string(),
        in_use: false,
    };

    storage
        .commit_installation(&staging_file, installation)
        .expect("commit installation");

    let mock_engine = Arc::new(MockWhisperEngine::new(true));
    let manager = TranscriptionManager::new(Arc::new(storage), mock_engine);

    let req = TranscriptionRequest {
        asset_id: "asset-1".to_string(),
        model_id: "whisper-tiny".to_string(),
        language: "en".to_string(),
        output_track_name: None,
        custom_runtime_path: None,
        threads: None,
        start_time_us: None,
        end_time_us: None,
    };

    let res = manager
        .start_transcription(req, PathBuf::from("/any/path.mp4"), None)
        .await;

    assert!(matches!(
        res,
        Err(TranscriptionError::ModelIntegrityFailed { .. })
    ));
}

#[tokio::test]
async fn test_uninstalled_model_rejected() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let storage = AiStorageManager::new(temp_dir.path()).expect("create storage");
    let mock_engine = Arc::new(MockWhisperEngine::new(true));
    let manager = TranscriptionManager::new(Arc::new(storage), mock_engine);

    let req = TranscriptionRequest {
        asset_id: "asset-1".to_string(),
        model_id: "whisper-base".to_string(), // not installed
        language: "en".to_string(),
        output_track_name: None,
        custom_runtime_path: None,
        threads: None,
        start_time_us: None,
        end_time_us: None,
    };

    let res = manager
        .start_transcription(req, PathBuf::from("/any/path.mp4"), None)
        .await;
    assert!(matches!(
        res,
        Err(TranscriptionError::ModelNotInstalled { .. })
    ));
}

#[tokio::test]
async fn test_cancellation_cleans_up_and_releases_lock() {
    let (_dir, storage, _) = setup_test_environment("whisper-tiny");
    let mock_engine = Arc::new(MockWhisperEngine::new(true));
    let manager = TranscriptionManager::new(Arc::new(storage.clone()), mock_engine);

    let media_temp = tempfile::NamedTempFile::new().expect("create media temp");
    let req = TranscriptionRequest {
        asset_id: "asset-cancel".to_string(),
        model_id: "whisper-tiny".to_string(),
        language: "en".to_string(),
        output_track_name: None,
        custom_runtime_path: None,
        threads: None,
        start_time_us: None,
        end_time_us: None,
    };

    let job_id = manager
        .start_transcription(req, media_temp.path().to_path_buf(), None)
        .await
        .expect("start job");

    // Cancel immediately
    let cancel_res = manager.cancel_job(&job_id).await;
    assert!(cancel_res.is_ok());

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify model lock was released
    assert!(!storage.is_locked("whisper-tiny"));
}

#[test]
fn test_supported_languages_list() {
    let langs = get_supported_whisper_languages();
    assert!(langs.iter().any(|l| l.code == "auto"));
    assert!(langs.iter().any(|l| l.code == "en"));
    assert!(langs
        .iter()
        .any(|l| l.code == "bn" && l.label.contains("বাংলা")));
}

#[tokio::test]
async fn test_genuine_whisper_cpp_inference_end_to_end() {
    use opentake_core::ai::transcription::whisper::{SystemWhisperCliEngine, WhisperEngine};
    use opentake_core::localization::subtitles::model::SubtitleFormat;
    use tokio_util::sync::CancellationToken;

    let whisper_cli_path = PathBuf::from("/home/osmangoni/.local/share/opentake/bin/whisper-cli");
    let model_path = PathBuf::from(
        "/home/osmangoni/.local/share/opentake/models/installed/whisper-tiny/ggml-tiny.bin",
    );
    let audio_path = PathBuf::from("/home/osmangoni/.local/share/opentake/fixtures/jfk.wav");

    if !whisper_cli_path.is_file() || !model_path.is_file() || !audio_path.is_file() {
        println!("Skipping genuine whisper test: binary, model, or audio fixture not present.");
        return;
    }

    let engine = SystemWhisperCliEngine::new(whisper_cli_path, "whisper.cpp v1.9.5".to_string());
    let cancel = CancellationToken::new();

    let raw_output = engine
        .transcribe(&model_path, &audio_path, "en", Some(4), cancel)
        .await
        .expect("genuine whisper.cpp inference must succeed");

    assert!(
        !raw_output.segments.is_empty(),
        "Transcribed segments must not be empty"
    );
    let full_text: String = raw_output
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert!(
        full_text.to_lowercase().contains("fellow americans"),
        "Recognized text must contain known speech from audio"
    );

    let processed_segments = process_raw_segments(raw_output.segments, 0);

    // Convert into canonical SubtitleTrack
    let result = opentake_core::ai::transcription::models::TranscriptionResult {
        schema_version: 1,
        job_id: "test-genuine-job".to_string(),
        asset_id: "asset-jfk".to_string(),
        model_id: "whisper-tiny".to_string(),
        runtime_version: "whisper.cpp v1.9.5".to_string(),
        requested_language: "en".to_string(),
        detected_language: Some("en".to_string()),
        segments: processed_segments,
        audio_duration_us: Some(11_000_000),
        processing_duration_ms: 800,
        warnings: vec![],
    };

    let track = convert_transcription_result_to_track(&result, Some("JFK Historic Speech"))
        .expect("convert to SubtitleTrack");
    assert_eq!(track.display_name, "JFK Historic Speech");
    assert!(!track.cues.is_empty());

    // Export to SRT
    let srt_export = opentake_core::localization::subtitles::export_subtitles_to_string(
        &track,
        SubtitleFormat::Srt,
    )
    .expect("export to SRT");
    assert!(srt_export.contains("fellow Americans") || srt_export.contains("country"));

    // Export to WebVTT
    let vtt_export = opentake_core::localization::subtitles::export_subtitles_to_string(
        &track,
        SubtitleFormat::WebVtt,
    )
    .expect("export to WebVTT");
    assert!(vtt_export.starts_with("WEBVTT"));
    assert!(vtt_export.contains("fellow Americans") || vtt_export.contains("country"));
}
