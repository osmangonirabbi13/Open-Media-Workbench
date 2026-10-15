use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use opentake_core::ai::catalog::get_curated_model_catalog;
use opentake_core::ai::models::AiModelInstallation;
use opentake_core::ai::storage::AiStorageManager;
use opentake_core::ai::translation::engine::{
    MockTranslationEngine, SystemLlamaCliTranslationEngine, TranslationEngine,
};
use opentake_core::ai::translation::errors::TranslationError;
use opentake_core::ai::translation::manager::TranslationManager;
use opentake_core::ai::translation::models::{
    get_supported_language_pairs, TranslationProgressStage, TranslationRequest,
};
use opentake_core::ai::translation::subtitle_conversion::{
    append_translated_track_to_document, convert_translation_to_track,
};
use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::model::{
    SubtitleCue, SubtitleDocument, SubtitleFormat, SubtitleTrack,
};

fn setup_test_storage(model_id: &str) -> (tempfile::TempDir, AiStorageManager, PathBuf) {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let storage = AiStorageManager::new(temp_dir.path()).expect("create storage");

    let staging_file = storage
        .create_staging_path(model_id, "init")
        .expect("staging path");
    let mock_bytes = b"MOCK_GGUF_QWEN_MODEL_WEIGHTS";
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

    let installed_file = storage
        .get_artifact_path(model_id, &manifest.artifact_filename)
        .expect("installed path");
    (temp_dir, storage, installed_file)
}

#[test]
fn test_language_pair_catalog() {
    let pairs = get_supported_language_pairs();
    assert!(pairs.len() >= 2);

    let en_es = pairs
        .iter()
        .find(|p| p.source_code == "en" && p.target_code == "es")
        .expect("en -> es pair exists");
    assert!(en_es.is_verified);
    assert!(en_es
        .compatible_model_ids
        .contains(&"qwen2.5-0.5b-instruct".to_string()));

    let en_bn = pairs
        .iter()
        .find(|p| p.source_code == "en" && p.target_code == "bn")
        .expect("en -> bn pair exists");
    assert!(en_bn
        .compatible_model_ids
        .contains(&"qwen2.5-0.5b-instruct".to_string()));
}

#[tokio::test]
async fn test_translation_request_validation() {
    let (_temp, storage, _model_path) = setup_test_storage("qwen2.5-0.5b-instruct");
    let engine = Arc::new(MockTranslationEngine::new(true));
    let manager = TranslationManager::new(Arc::new(storage), engine);

    // Empty cues should be rejected
    let req = TranslationRequest {
        document_id: Some("doc-1".to_string()),
        track_id: "track-1".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        model_id: "qwen2.5-0.5b-instruct".to_string(),
        custom_runtime_path: None,
        batch_size: None,
    };

    let result = manager.start_translation(req, vec![]).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TranslationError::EmptyTrack(track) => assert_eq!(track, "track-1"),
        other => panic!("expected EmptyTrack error, got: {:?}", other),
    }

    // Unsupported language pair should be rejected
    let req_invalid_pair = TranslationRequest {
        document_id: Some("doc-1".to_string()),
        track_id: "track-1".to_string(),
        source_language: "klingon".to_string(),
        target_language: "elvish".to_string(),
        model_id: "qwen2.5-0.5b-instruct".to_string(),
        custom_runtime_path: None,
        batch_size: None,
    };

    let cue = SubtitleCue::new("c1", MediaTimeUs(1000), MediaTimeUs(2000), "Hello").expect("cue");
    let result_pair = manager.start_translation(req_invalid_pair, vec![cue]).await;
    assert!(result_pair.is_err());
    match result_pair.unwrap_err() {
        TranslationError::LanguagePairNotSupported { .. } => {}
        other => panic!("expected LanguagePairNotSupported, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_missing_model_rejection() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let storage = AiStorageManager::new(temp_dir.path()).expect("storage");
    let engine = Arc::new(MockTranslationEngine::new(true));
    let manager = TranslationManager::new(Arc::new(storage), engine);

    let req = TranslationRequest {
        document_id: Some("doc-1".to_string()),
        track_id: "track-1".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        model_id: "qwen2.5-0.5b-instruct".to_string(), // Not installed in this empty storage
        custom_runtime_path: None,
        batch_size: None,
    };

    let cue = SubtitleCue::new("c1", MediaTimeUs(1000), MediaTimeUs(2000), "Hello").expect("cue");
    let result = manager.start_translation(req, vec![cue]).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TranslationError::ModelNotInstalled { model_id, .. } => {
            assert_eq!(model_id, "qwen2.5-0.5b-instruct")
        }
        other => panic!("expected ModelNotInstalled, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_timing_preservation_and_cue_order_invariants() {
    let (_temp, storage, _model_path) = setup_test_storage("qwen2.5-0.5b-instruct");
    let engine = Arc::new(MockTranslationEngine::new(true));
    let manager = TranslationManager::new(Arc::new(storage), engine);

    let cues = vec![
        SubtitleCue::new(
            "c1",
            MediaTimeUs(1_000_000),
            MediaTimeUs(3_500_000),
            "The first line of speech.",
        )
        .expect("c1"),
        SubtitleCue::new(
            "c2",
            MediaTimeUs(4_000_000),
            MediaTimeUs(7_000_000),
            "The second line across multiple cues.",
        )
        .expect("c2"),
        SubtitleCue::new(
            "c3",
            MediaTimeUs(7_500_000),
            MediaTimeUs(9_000_000),
            "Final concluding sentence.",
        )
        .expect("c3"),
    ];

    let req = TranslationRequest {
        document_id: Some("doc-inv".to_string()),
        track_id: "track-inv".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        model_id: "qwen2.5-0.5b-instruct".to_string(),
        custom_runtime_path: None,
        batch_size: None,
    };

    let job_id = manager
        .start_translation(req, cues.clone())
        .await
        .expect("start job");

    // Wait for mock translation completion
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let status = manager.get_status(&job_id).await.expect("status exists");
    assert_eq!(status.stage, TranslationProgressStage::Completed);

    let result = manager.get_result(&job_id).await.expect("result ready");
    assert_eq!(result.translated_cues.len(), 3);
    assert_eq!(result.original_track_id, "track-inv");

    for (orig, trans) in cues.iter().zip(result.translated_cues.iter()) {
        assert_eq!(trans.source_cue_id, orig.id);
        // Timing preservation invariants
        assert_eq!(trans.target_start_us, orig.start_us);
        assert_eq!(trans.target_end_us, orig.end_us);
        assert_eq!(trans.source_start_us, orig.start_us);
        assert_eq!(trans.source_end_us, orig.end_us);
        assert!(!trans.translated_text.is_empty());
    }
}

#[tokio::test]
async fn test_unicode_and_bengali_preservation() {
    let (_temp, storage, _model_path) = setup_test_storage("qwen2.5-0.5b-instruct");
    let engine = Arc::new(MockTranslationEngine::new(true));
    let manager = TranslationManager::new(Arc::new(storage), engine);

    let cues = vec![SubtitleCue::new(
        "bn-1",
        MediaTimeUs(1_000_000),
        MediaTimeUs(3_000_000),
        "বাংলা ভাষা ও সংস্কৃতি অত্যন্ত সমৃদ্ধ।",
    )
    .expect("bn-1")];

    let req = TranslationRequest {
        document_id: Some("doc-bn".to_string()),
        track_id: "track-bn".to_string(),
        source_language: "bn".to_string(),
        target_language: "en".to_string(),
        model_id: "qwen2.5-0.5b-instruct".to_string(),
        custom_runtime_path: None,
        batch_size: None,
    };

    let job_id = manager
        .start_translation(req, cues)
        .await
        .expect("start bn job");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let result = manager
        .get_result(&job_id)
        .await
        .expect("bn translation result");

    assert_eq!(result.translated_cues.len(), 1);
    assert_eq!(result.translated_cues[0].source_cue_id, "bn-1");
    assert!(result.translated_cues[0].translated_text.contains("বাংলা"));
}

#[tokio::test]
async fn test_subtitle_conversion_and_non_destructive_document_update() {
    let cues = vec![
        SubtitleCue::new(
            "cue-orig-1",
            MediaTimeUs(1_000_000),
            MediaTimeUs(2_500_000),
            "Hello everyone.",
        )
        .expect("cue1"),
        SubtitleCue::new(
            "cue-orig-2",
            MediaTimeUs(3_000_000),
            MediaTimeUs(5_000_000),
            "Welcome to Open Media Workbench.",
        )
        .expect("cue2"),
    ];

    let original_track = SubtitleTrack {
        id: "track-orig".to_string(),
        language: "en".to_string(),
        display_name: "English (Source)".to_string(),
        source_format: SubtitleFormat::NativeProject,
        cues: cues.clone(),
        is_enabled: true,
        is_visible: true,
        schema_version: 1,
    };

    let mut doc = SubtitleDocument {
        schema_version: 1,
        id: "doc-orig".to_string(),
        associated_media_asset_id: None,
        tracks: vec![original_track],
        active_track_id: Some("track-orig".to_string()),
        created_at_epoch_ms: 1700000000000,
        updated_at_epoch_ms: 1700000000000,
        metadata: HashMap::new(),
    };

    let (_temp, storage, _model_path) = setup_test_storage("qwen2.5-0.5b-instruct");
    let engine = Arc::new(MockTranslationEngine::new(true));
    let manager = TranslationManager::new(Arc::new(storage), engine);

    let req = TranslationRequest {
        document_id: Some("doc-orig".to_string()),
        track_id: "track-orig".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        model_id: "qwen2.5-0.5b-instruct".to_string(),
        custom_runtime_path: None,
        batch_size: None,
    };

    let job_id = manager
        .start_translation(req, cues)
        .await
        .expect("start job");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let result = manager.get_result(&job_id).await.expect("result");

    let new_track =
        convert_translation_to_track(&result, Some("Spanish (Dubbing)")).expect("convert track");
    assert_eq!(new_track.language, "es");
    assert_eq!(new_track.display_name, "Spanish (Dubbing)");
    assert_eq!(new_track.cues.len(), 2);

    // Invariant: Word timings must NOT be inherited from source
    assert!(new_track.cues[0].words.is_none());
    assert!(new_track.cues[1].words.is_none());

    // Non-destructive document update
    let new_track_id =
        append_translated_track_to_document(&mut doc, &result, Some("Spanish (Dubbing)"))
            .expect("append track");

    assert_eq!(doc.tracks.len(), 2);
    assert_eq!(doc.tracks[0].id, "track-orig");
    assert_eq!(doc.tracks[0].cues.len(), 2);
    assert_eq!(doc.tracks[0].cues[0].text, "Hello everyone.");
    assert_eq!(doc.active_track_id, Some(new_track_id));
}

#[tokio::test]
async fn test_genuine_llama_cli_translation_end_to_end() {
    let user_home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return, // Skip if no HOME
    };

    let llama_cli_path = user_home.join(".local/share/opentake/bin/llama-cli");
    let model_path = user_home.join(
        ".local/share/opentake/models/installed/qwen2.5-0.5b-instruct/qwen2.5-0.5b-instruct-q4_k_m.gguf",
    );

    if !llama_cli_path.exists() || !model_path.exists() {
        println!("Skipping genuine E2E: llama-cli or qwen2.5-0.5b-instruct model not installed");
        return;
    }

    let engine = SystemLlamaCliTranslationEngine::new(
        llama_cli_path.clone(),
        "llama.cpp b11658".to_string(),
    );

    let texts = vec![
        "Hello and welcome to Open Media Workbench!".to_string(),
        "We provide high quality offline localization and video repair.".to_string(),
    ];

    println!(
        "Executing genuine offline translation with llama-cli at: {:?}",
        llama_cli_path
    );
    let start_instant = std::time::Instant::now();
    let cancel_token = CancellationToken::new();
    let translated = engine
        .translate(&model_path, &texts, "en", "es", cancel_token)
        .await
        .expect("genuine translation should succeed");
    let elapsed = start_instant.elapsed();

    println!("Genuine inference finished in {:?}", elapsed);
    assert_eq!(translated.len(), 2);

    for (orig, trans) in texts.iter().zip(translated.iter()) {
        println!("Original:   [{}]", orig);
        println!("Translated: [{}]", trans);
        assert!(!trans.trim().is_empty());
        // Verify genuine Spanish output
        let lower = trans.to_lowercase();
        assert!(
            lower.contains("hola")
                || lower.contains("bienvenid")
                || lower.contains("medio")
                || lower.contains("calidad")
                || lower.contains("reparaci")
                || lower.contains("video"),
            "Expected authentic Spanish vocabulary in: {}",
            trans
        );
    }
}
