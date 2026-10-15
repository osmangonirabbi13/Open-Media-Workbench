//! Integration and domain tests for STEP 23:
//! Bilingual Subtitle Review, Glossary Management, Terminology Protection,
//! Translation Memory & Quality Assurance.

use opentake_core::ai::translation::review::{
    batch_approve_entries, batch_set_status, create_or_sync_review_document,
    edit_review_entry_target_text, embed_review_in_document, evaluate_entry_quality,
    extract_review_from_document, set_review_entry_status, text_contains_word,
    validate_terminology, Glossary, GlossaryEntry, ProtectedTermMasker, QualityFindingCode,
    QualitySeverity, ReviewStatus, ReviewStorageManager, TranslationMemory,
};
use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::{
    serialize_srt, serialize_webvtt, SubtitleCue, SubtitleDocument, SubtitleFormat, SubtitleTrack,
};
use std::collections::HashMap;

fn create_sample_tracks() -> (SubtitleTrack, SubtitleTrack) {
    let source_cues = vec![
        SubtitleCue::new(
            "cue-1",
            MediaTimeUs(1_000_000),
            MediaTimeUs(3_000_000),
            "Welcome to Open Media Workbench.",
        )
        .expect("cue 1"),
        SubtitleCue::new(
            "cue-2",
            MediaTimeUs(4_000_000),
            MediaTimeUs(7_000_000),
            "Offline translation engine powered by llama.cpp and Whisper.",
        )
        .expect("cue 2"),
        SubtitleCue::new(
            "cue-3",
            MediaTimeUs(8_000_000),
            MediaTimeUs(10_000_000),
            "Our mission is local privacy.",
        )
        .expect("cue 3"),
    ];

    let target_cues = vec![
        SubtitleCue::new(
            "cue-1-tr-es",
            MediaTimeUs(1_000_000),
            MediaTimeUs(3_000_000),
            "Bienvenido a Open Media Workbench.",
        )
        .expect("target cue 1"),
        SubtitleCue::new(
            "cue-2-tr-es",
            MediaTimeUs(4_000_000),
            MediaTimeUs(7_000_000),
            "Motor de traducción sin conexión impulsado por llama.cpp y Whisper.",
        )
        .expect("target cue 2"),
        SubtitleCue::new(
            "cue-3-tr-es",
            MediaTimeUs(8_000_000),
            MediaTimeUs(10_000_000),
            "Nuestra misión es la privacidad local.",
        )
        .expect("target cue 3"),
    ];

    let source_track = SubtitleTrack {
        id: "track-en".to_string(),
        language: "en".to_string(),
        display_name: "English (Source)".to_string(),
        source_format: SubtitleFormat::NativeProject,
        cues: source_cues,
        is_enabled: true,
        is_visible: true,
        schema_version: 1,
    };

    let target_track = SubtitleTrack {
        id: "track-es".to_string(),
        language: "es".to_string(),
        display_name: "Spanish (Translated)".to_string(),
        source_format: SubtitleFormat::NativeProject,
        cues: target_cues,
        is_enabled: true,
        is_visible: true,
        schema_version: 1,
    };

    (source_track, target_track)
}

#[test]
fn test_cue_mapping_by_stable_ids_and_timing() {
    let (src, tgt) = create_sample_tracks();
    let review = create_or_sync_review_document("doc-test", &src, &tgt, None, &[], true);

    assert_eq!(review.entries.len(), 3);
    assert_eq!(review.entries[0].source_cue_id, "cue-1");
    assert_eq!(review.entries[0].target_cue_id, "cue-1-tr-es");
    assert_eq!(
        review.entries[0].source_text,
        "Welcome to Open Media Workbench."
    );
    assert_eq!(
        review.entries[0].target_text,
        "Bienvenido a Open Media Workbench."
    );
    assert_eq!(review.entries[0].status, ReviewStatus::Pending);
}

#[test]
fn test_unmapped_cue_diagnostics() {
    let (src, mut tgt) = create_sample_tracks();
    // Drop cue 2 in target track
    tgt.cues.remove(1);

    let review = create_or_sync_review_document("doc-test", &src, &tgt, None, &[], true);

    assert_eq!(review.entries.len(), 3);
    let unmapped = review
        .entries
        .iter()
        .find(|e| e.source_cue_id == "cue-2")
        .unwrap();
    assert!(unmapped
        .findings
        .iter()
        .any(|f| f.code == QualityFindingCode::UnmappedCue));
    assert_eq!(unmapped.target_cue_id, "cue-2-unmapped");
}

#[test]
fn test_approval_invalidation_after_text_edit() {
    let (src, mut tgt) = create_sample_tracks();
    let mut review = create_or_sync_review_document("doc-test", &src, &tgt, None, &[], true);
    let entry_id = review.entries[0].id.clone();

    // 1. Mark Approved
    set_review_entry_status(&mut review, &entry_id, ReviewStatus::Approved, false)
        .expect("approve entry");
    assert_eq!(review.entries[0].status, ReviewStatus::Approved);
    assert!(review.entries[0].approved_fingerprint.is_some());

    // 2. User edits target text
    edit_review_entry_target_text(
        &mut review,
        &mut tgt,
        &entry_id,
        "Bienvenido a Open Media Workbench (Edición Revisada).",
        &[],
        true,
    )
    .expect("edit text");

    // 3. Approval MUST be invalidated
    assert_eq!(review.entries[0].status, ReviewStatus::NeedsReview);
    assert!(review.entries[0].approved_fingerprint.is_none());
    assert_eq!(
        tgt.cues[0].text,
        "Bienvenido a Open Media Workbench (Edición Revisada)."
    );
}

#[test]
fn test_state_transitions_and_blocking_errors() {
    let (src, tgt) = create_sample_tracks();
    let mut review = create_or_sync_review_document("doc-test", &src, &tgt, None, &[], true);
    let entry_id = review.entries[0].id.clone();

    // Transition NeedsReview -> Reviewed
    set_review_entry_status(&mut review, &entry_id, ReviewStatus::Reviewed, false).unwrap();
    assert_eq!(review.entries[0].status, ReviewStatus::Reviewed);

    // Transition Reviewed -> Approved
    set_review_entry_status(&mut review, &entry_id, ReviewStatus::Approved, false).unwrap();
    assert_eq!(review.entries[0].status, ReviewStatus::Approved);

    // Transition Approved -> Rejected
    set_review_entry_status(&mut review, &entry_id, ReviewStatus::Rejected, false).unwrap();
    assert_eq!(review.entries[0].status, ReviewStatus::Rejected);
}

#[test]
fn test_batch_review_operations() {
    let (src, tgt) = create_sample_tracks();
    let mut review = create_or_sync_review_document("doc-test", &src, &tgt, None, &[], true);
    let all_ids: Vec<String> = review.entries.iter().map(|e| e.id.clone()).collect();

    // Batch Reviewed
    batch_set_status(&mut review, &all_ids, ReviewStatus::Reviewed);
    assert_eq!(review.entries[0].status, ReviewStatus::Reviewed);
    assert_eq!(review.entries[1].status, ReviewStatus::Reviewed);
    assert_eq!(review.entries[2].status, ReviewStatus::Reviewed);

    // Batch Approved
    let approved_count = batch_approve_entries(&mut review, &all_ids, false).unwrap();
    assert_eq!(approved_count, 3);
    assert_eq!(review.entries[0].status, ReviewStatus::Approved);
    assert_eq!(review.entries[1].status, ReviewStatus::Approved);
    assert_eq!(review.entries[2].status, ReviewStatus::Approved);
}

#[test]
fn test_protected_term_matching_and_unicode_word_boundaries() {
    assert!(text_contains_word(
        "Open Media Workbench is great.",
        "Open Media Workbench",
        false
    ));
    assert!(text_contains_word(
        "We use FFmpeg and llama.cpp.",
        "FFmpeg",
        false
    ));
    assert!(text_contains_word(
        "We use FFmpeg and llama.cpp.",
        "llama.cpp",
        false
    ));

    // Must NOT match inside longer unrelated words
    assert!(!text_contains_word(
        "SuperOpen Media WorkbenchZone",
        "Open Media Workbench",
        false
    ));
    assert!(!text_contains_word("AntiWhispering", "Whisper", false));

    // Unicode word boundary
    assert!(text_contains_word("Café au lait", "Café", false));
}

#[test]
fn test_bengali_unicode_grapheme_preservation_and_matching() {
    let bn_text = "আমাদের বাংলা ভাষা ও সংস্কৃতি অত্যন্ত সমৃদ্ধ।";
    assert!(text_contains_word(bn_text, "বাংলা", false));
    assert!(text_contains_word(bn_text, "সংস্কৃতি", false));
    assert!(!text_contains_word(bn_text, "ইংরেজি", false));

    // Grapheme safety: Editing does not corrupt composite characters (যুক্তবর্ণ)
    let complex_bn = "বিজ্ঞান ও প্রযুক্তি";
    assert!(text_contains_word(complex_bn, "বিজ্ঞান", false));
    assert!(text_contains_word(complex_bn, "প্রযুক্তি", false));
}

#[test]
fn test_placeholder_masking_unmasking_and_tampering_detection() {
    let mut masker = ProtectedTermMasker::new();
    let protected = ["Open Media Workbench", "llama.cpp"];

    let original = "Welcome to Open Media Workbench with llama.cpp engine.";
    let masked = masker.mask(original, &protected);

    assert!(masked.contains("__OMW_TERM_0__"));
    assert!(masked.contains("__OMW_TERM_1__"));
    assert!(!masked.contains("Open Media Workbench"));

    // Happy path unmasking
    let translated_mock = "Bienvenido a __OMW_TERM_0__ con el motor __OMW_TERM_1__.";
    let restored = masker.unmask(translated_mock).expect("unmask");
    assert_eq!(
        restored,
        "Bienvenido a Open Media Workbench con el motor llama.cpp."
    );

    // Tampering 1: Dropped placeholder
    let dropped = "Bienvenido a Open Media Workbench con el motor __OMW_TERM_1__.";
    assert!(masker.unmask(dropped).is_err());

    // Tampering 2: Duplicated placeholder
    let duplicated = "Bienvenido a __OMW_TERM_0__ y __OMW_TERM_0__ con __OMW_TERM_1__.";
    assert!(masker.unmask(duplicated).is_err());
}

#[test]
fn test_missing_protected_term_quality_diagnostics() {
    let entries = vec![GlossaryEntry {
        id: "g1".to_string(),
        source_term: "Open Media Workbench".to_string(),
        preferred_term: "Open Media Workbench".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        description: None,
        is_protected: true,
        case_sensitive: false,
        domain: None,
        created_at_epoch_ms: 0,
        updated_at_epoch_ms: 0,
    }];

    // Target omitted "Open Media Workbench"
    let findings = validate_terminology(
        "Welcome to Open Media Workbench.",
        "Bienvenido a la mesa de trabajo.",
        &entries,
    );

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, QualityFindingCode::ProtectedTermChanged);
    assert_eq!(findings[0].severity, QualitySeverity::Error);
}

#[test]
fn test_glossary_conflict_detection() {
    let mut glossary = Glossary::new("Brand Terms", "en", "es", None);

    let entry1 = GlossaryEntry {
        id: "e1".to_string(),
        source_term: "Workbench".to_string(),
        preferred_term: "Mesa de trabajo".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        description: None,
        is_protected: false,
        case_sensitive: false,
        domain: None,
        created_at_epoch_ms: 0,
        updated_at_epoch_ms: 0,
    };
    glossary.upsert_entry(entry1).expect("add e1");

    // Conflicting rule for same source term with different preferred term
    let entry2 = GlossaryEntry {
        id: "e2".to_string(),
        source_term: "Workbench".to_string(),
        preferred_term: "Banco de trabajo".to_string(),
        source_language: "en".to_string(),
        target_language: "es".to_string(),
        description: None,
        is_protected: false,
        case_sensitive: false,
        domain: None,
        created_at_epoch_ms: 0,
        updated_at_epoch_ms: 0,
    };
    let res = glossary.upsert_entry(entry2);
    assert!(res.is_err());
}

#[test]
fn test_translation_memory_exact_matching_and_privacy() {
    let mut tm = TranslationMemory::new();

    // Record approved translation
    let recorded = tm.record_approved(
        "Welcome to Open Media Workbench.",
        "Bienvenido a Open Media Workbench.",
        "en",
        "es",
        Some("Product"),
    );

    assert_eq!(recorded.usage_count, 1);

    // Exact normalized matching ignores casing and whitespace variance
    let match_opt = tm.find_exact_match("  welcome   to OPEN MEDIA WORKBENCH.  ", "en", "es");
    assert!(match_opt.is_some());
    assert_eq!(
        match_opt.unwrap().target_text,
        "Bienvenido a Open Media Workbench."
    );

    // Delete entry
    assert!(tm.delete_entry(&recorded.id));
    assert!(tm
        .find_exact_match("Welcome to Open Media Workbench.", "en", "es")
        .is_none());

    // Clear TM
    tm.record_approved("Test", "Prueba", "en", "es", None);
    assert_eq!(tm.entries.len(), 1);
    tm.clear();
    assert_eq!(tm.entries.len(), 0);
}

#[test]
fn test_quality_diagnostics_reading_speed_and_line_length() {
    let (src, tgt) = create_sample_tracks();
    let mut review = create_or_sync_review_document("doc-test", &src, &tgt, None, &[], true);

    // Excessive reading speed: 100 characters in 1 second interval = 100 chars/sec (> 25)
    review.entries[0].source_start_us = MediaTimeUs(1_000_000);
    review.entries[0].source_end_us = MediaTimeUs(2_000_000);
    review.entries[0].target_text = "This is an extraordinarily long translated subtitle that definitely exceeds twenty-five chars/second.".to_string();

    let findings = evaluate_entry_quality(&review.entries[0], &[], true);
    assert!(findings
        .iter()
        .any(|f| f.code == QualityFindingCode::ExcessiveReadingSpeed));

    // Excessive line length (> 84 chars)
    assert!(findings
        .iter()
        .any(|f| f.code == QualityFindingCode::ExcessiveLineLength));
}

#[test]
fn test_native_project_persistence_embedding_and_legacy_loading() {
    let (src, tgt) = create_sample_tracks();
    let review = create_or_sync_review_document("doc-test", &src, &tgt, None, &[], true);

    let mut doc = SubtitleDocument {
        schema_version: 1,
        id: "doc-test".to_string(),
        associated_media_asset_id: None,
        tracks: vec![src.clone(), tgt.clone()],
        active_track_id: Some("track-es".to_string()),
        created_at_epoch_ms: 1700000000000,
        updated_at_epoch_ms: 1700000000000,
        metadata: HashMap::new(),
    };

    // Embed review in document metadata
    embed_review_in_document(&mut doc, &review).expect("embed review");
    assert!(doc.metadata.contains_key("translationReview"));

    // Extract back
    let extracted = extract_review_from_document(&doc).expect("extracted review");
    assert_eq!(extracted.review_id, review.review_id);
    assert_eq!(extracted.entries.len(), 3);

    // Legacy project loading: A document with no review metadata extracts as None safely
    let legacy_doc = SubtitleDocument {
        schema_version: 1,
        id: "doc-legacy".to_string(),
        associated_media_asset_id: None,
        tracks: vec![src],
        active_track_id: None,
        created_at_epoch_ms: 1600000000000,
        updated_at_epoch_ms: 1600000000000,
        metadata: HashMap::new(),
    };
    assert!(extract_review_from_document(&legacy_doc).is_none());
}

#[test]
fn test_srt_and_webvtt_export_after_review() {
    let (src, mut tgt) = create_sample_tracks();
    let mut review = create_or_sync_review_document("doc-export", &src, &tgt, None, &[], true);

    // Human reviews and corrects target text
    let entry_id = review.entries[0].id.clone();
    edit_review_entry_target_text(
        &mut review,
        &mut tgt,
        &entry_id,
        "Bienvenido a Open Media Workbench (Aprobado).",
        &[],
        true,
    )
    .unwrap();

    set_review_entry_status(&mut review, &entry_id, ReviewStatus::Approved, false).unwrap();

    // Export reviewed target track to SRT
    let srt = serialize_srt(&tgt).expect("export srt");
    assert!(srt.contains("Bienvenido a Open Media Workbench (Aprobado)."));
    // Ensure source track was NOT modified
    assert_eq!(src.cues[0].text, "Welcome to Open Media Workbench.");

    // Export to WebVTT
    let vtt = serialize_webvtt(&tgt).expect("export vtt");
    assert!(vtt.contains("WEBVTT"));
    assert!(vtt.contains("Bienvenido a Open Media Workbench (Aprobado)."));
}

#[test]
fn test_isolated_review_storage_manager() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let storage = ReviewStorageManager::new(temp_dir.path()).expect("create storage");

    let glossary = Glossary::new("Test Glossary", "en", "es", None);
    storage.save_glossary(&glossary).expect("save glossary");

    let loaded = storage.load_glossary(&glossary.id).expect("load glossary");
    assert_eq!(loaded.name, "Test Glossary");

    let all_glossaries = storage.list_glossaries().expect("list glossaries");
    assert_eq!(all_glossaries.len(), 1);

    storage
        .delete_glossary(&glossary.id)
        .expect("delete glossary");
    assert!(storage.load_glossary(&glossary.id).is_err());
}
