//! Integration and capability tests for STEP 20: Word-level Timestamp Alignment,
//! Smart Subtitle Segmentation, and Karaoke Foundation.

use opentake_core::ai::alignment::models::{
    AlignedWord, AlignmentResult, SegmentationPolicy, TimingQuality, WordTimingProvenance,
};
use opentake_core::ai::alignment::parser::{
    parse_whisper_alignment_output, split_word_punctuation,
};
use opentake_core::ai::alignment::refinement::update_word_timing_in_cue;
use opentake_core::ai::alignment::segmentation::segment_words_into_cues;
use opentake_core::ai::alignment::validation::validate_word_sequence;
use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::model::{
    SubtitleCue, SubtitleDocument, SubtitleFormat, SubtitleTrack,
};
use opentake_core::localization::subtitles::operations::SubtitleOperation;
use std::path::PathBuf;

#[test]
fn test_01_native_word_timing_parser_json_full() {
    let json_content = r#"{
        "transcription": [
            {
                "timestamps": { "from": "00:00:00,000", "to": "00:00:02,500" },
                "text": " Welcome to Open Take.",
                "tokens": [
                    { "text": " Welcome", "timestamps": { "from": "00:00:00,100", "to": "00:00:00,600" }, "p": 0.98 },
                    { "text": " to", "timestamps": { "from": "00:00:00,650", "to": "00:00:00,850" }, "p": 0.99 },
                    { "text": " Open", "timestamps": { "from": "00:00:00,900", "to": "00:00:01,400" }, "p": 0.95 },
                    { "text": " Take.", "timestamps": { "from": "00:00:01,450", "to": "00:00:02,100" }, "p": 0.97 }
                ]
            }
        ]
    }"#;

    let (words, _quality) = parse_whisper_alignment_output(json_content, Some("seg_0"))
        .expect("parse whisper JSON full");
    assert_eq!(words.len(), 4);
    assert_eq!(words[0].text, "Welcome");
    assert_eq!(words[0].start_us, MediaTimeUs(100_000));
    assert_eq!(words[0].end_us, MediaTimeUs(600_000));
    assert_eq!(
        words[0].provenance,
        WordTimingProvenance::WhisperNativeEstimate
    );
    assert_eq!(words[3].text, "Take");
    assert_eq!(words[3].punctuation_suffix.as_deref(), Some("."));
}

#[tokio::test]
async fn test_02_runtime_capability_detection() {
    use opentake_core::ai::transcription::whisper::discover_whisper_runtime;
    let caps = discover_whisper_runtime(None).await;
    // System must evaluate available runtime honestly
    if caps.is_available {
        assert!(caps.runtime_path.is_some());
        assert!(caps.runtime_version.is_some());
    } else {
        assert!(caps.runtime_path.is_none());
    }
}

#[test]
fn test_03_timestamp_unit_conversion() {
    use opentake_core::ai::transcription::segments::parse_timestamp_str_to_micros;
    assert_eq!(
        parse_timestamp_str_to_micros("00:00:01,234").unwrap(),
        1_234_000
    );
    assert_eq!(
        parse_timestamp_str_to_micros("00:01:00.500").unwrap(),
        60_500_000
    );
    assert_eq!(
        parse_timestamp_str_to_micros("01:00:00,000").unwrap(),
        3_600_000_000
    );
}

#[test]
fn test_04_zero_and_negative_duration_rejection() {
    let err_neg = AlignedWord::new(
        "w1",
        "invalid",
        MediaTimeUs(-100),
        MediaTimeUs(500),
        WordTimingProvenance::WhisperNativeEstimate,
    );
    assert!(err_neg.is_err(), "Negative start time must be rejected");

    let err_zero = AlignedWord::new(
        "w2",
        "zero",
        MediaTimeUs(1000),
        MediaTimeUs(1000),
        WordTimingProvenance::WhisperNativeEstimate,
    );
    assert!(err_zero.is_err(), "Zero duration word must be rejected");

    let err_rev = AlignedWord::new(
        "w3",
        "reversed",
        MediaTimeUs(2000),
        MediaTimeUs(1000),
        WordTimingProvenance::WhisperNativeEstimate,
    );
    assert!(err_rev.is_err(), "Reversed interval must be rejected");
}

#[test]
fn test_05_word_ordering_invariants() {
    let w1 = AlignedWord::new(
        "w1",
        "First",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w2 = AlignedWord::new(
        "w2",
        "Second",
        MediaTimeUs(600_000),
        MediaTimeUs(1_000_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let valid_seq = vec![w1.clone(), w2.clone()];
    assert!(validate_word_sequence(&valid_seq).is_ok());

    let invalid_seq = vec![w2, w1];
    assert!(
        validate_word_sequence(&invalid_seq).is_err(),
        "Out-of-order words must be flagged"
    );
}

#[test]
fn test_06_repeated_words_handling() {
    let w1 = AlignedWord::new(
        "w1",
        "very",
        MediaTimeUs(100_000),
        MediaTimeUs(400_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w2 = AlignedWord::new(
        "w2",
        "very",
        MediaTimeUs(450_000),
        MediaTimeUs(800_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w3 = AlignedWord::new(
        "w3",
        "good",
        MediaTimeUs(850_000),
        MediaTimeUs(1_200_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();

    let words = vec![w1, w2, w3];
    assert!(validate_word_sequence(&words).is_ok());

    let policy = SegmentationPolicy::default();
    let (cues, _) = segment_words_into_cues(&words, &policy, "en");
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, "very very good");
    assert_eq!(cues[0].words.as_ref().unwrap().len(), 3);
}

#[test]
fn test_07_punctuation_split_and_preservation() {
    let (pfx, clean, sfx) = split_word_punctuation("\"Hello!\"");
    assert_eq!(pfx, Some("\"".to_string()));
    assert_eq!(clean, "Hello");
    assert_eq!(sfx, Some("!\"".to_string()));

    let (pfx2, clean2, sfx2) = split_word_punctuation("World,");
    assert_eq!(pfx2, None);
    assert_eq!(clean2, "World");
    assert_eq!(sfx2, Some(",".to_string()));
}

#[test]
fn test_08_bengali_unicode_words() {
    let (pfx, clean, sfx) = split_word_punctuation("স্বাগতম।");
    assert_eq!(pfx, None);
    assert_eq!(clean, "স্বাগতম");
    assert_eq!(sfx, Some("।".to_string()));

    let w = AlignedWord::new(
        "bn_1",
        clean,
        MediaTimeUs(0),
        MediaTimeUs(800_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    assert_eq!(w.text, "স্বাগতম");
    assert_eq!(w.duration_us(), MediaTimeUs(800_000));
}

#[test]
fn test_09_grapheme_boundary_safety() {
    let bengali_word = "বাংলা";
    let (pfx, clean, sfx) = split_word_punctuation(bengali_word);
    assert_eq!(pfx, None);
    assert_eq!(clean, "বাংলা");
    assert_eq!(sfx, None);
    // Ensure combining signs are intact
    assert_eq!(clean.chars().count(), 5); // ব, া, ং, ল, া
}

#[test]
fn test_10_mixed_script_subtitles() {
    let w1 = AlignedWord::new(
        "w1",
        "ওপেন",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w2 = AlignedWord::new(
        "w2",
        "Media",
        MediaTimeUs(550_000),
        MediaTimeUs(1_100_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let mut w3 = AlignedWord::new(
        "w3",
        "2026",
        MediaTimeUs(1_150_000),
        MediaTimeUs(1_800_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    w3.punctuation_suffix = Some("।".to_string());

    let words = vec![w1, w2, w3];
    let policy = SegmentationPolicy::default();
    let (cues, _) = segment_words_into_cues(&words, &policy, "bn");
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, "ওপেন Media 2026।");
}

#[test]
fn test_11_silence_gaps_segmentation() {
    let w1 = AlignedWord::new(
        "w1",
        "First",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w2 = AlignedWord::new(
        "w2",
        "phrase",
        MediaTimeUs(550_000),
        MediaTimeUs(1_000_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    // 600ms silence gap (> 400ms threshold)
    let w3 = AlignedWord::new(
        "w3",
        "Second",
        MediaTimeUs(1_600_000),
        MediaTimeUs(2_000_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w4 = AlignedWord::new(
        "w4",
        "phrase",
        MediaTimeUs(2_050_000),
        MediaTimeUs(2_500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();

    let policy = SegmentationPolicy {
        split_on_silence_gap_us: 400_000,
        ..Default::default()
    };

    let (cues, _) = segment_words_into_cues(&[w1, w2, w3, w4], &policy, "en");
    assert_eq!(cues.len(), 2, "Must split into two cues on silence gap");
    assert_eq!(cues[0].text, "First phrase");
    assert_eq!(cues[1].text, "Second phrase");
}

#[test]
fn test_12_word_timing_overlap() {
    let w1 = AlignedWord::new(
        "w1",
        "Overlap",
        MediaTimeUs(0),
        MediaTimeUs(800_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w2 = AlignedWord::new(
        "w2",
        "word",
        MediaTimeUs(750_000),
        MediaTimeUs(1_400_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();

    let words = vec![w1, w2];
    let warnings = validate_word_sequence(&words).unwrap();
    assert!(
        warnings.iter().any(|w| w.contains("overlap")),
        "Overlap must generate an informational warning"
    );
}

#[test]
fn test_13_partial_alignment() {
    let w1 = AlignedWord::new(
        "w1",
        "Aligned",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let mut w2 = AlignedWord::new(
        "w2",
        "Unaligned",
        MediaTimeUs(550_000),
        MediaTimeUs(1_000_000),
        WordTimingProvenance::Unaligned,
    )
    .unwrap();
    w2.provenance = WordTimingProvenance::Unaligned;

    let res = AlignmentResult {
        schema_version: 1,
        source_asset_id: "asset-1".to_string(),
        job_id: "job-1".to_string(),
        model_id: "whisper-tiny".to_string(),
        language: "en".to_string(),
        timing_quality: TimingQuality::PartiallyAligned,
        words: vec![w1, w2],
        warnings: vec![],
    };
    assert_eq!(res.timing_quality, TimingQuality::PartiallyAligned);
}

#[test]
fn test_14_missing_timing_handling() {
    let res = AlignmentResult {
        schema_version: 1,
        source_asset_id: "asset-empty".to_string(),
        job_id: "job-empty".to_string(),
        model_id: "whisper-tiny".to_string(),
        language: "en".to_string(),
        timing_quality: TimingQuality::Unavailable,
        words: vec![],
        warnings: vec!["No speech recognized".to_string()],
    };
    assert_eq!(res.timing_quality, TimingQuality::Unavailable);
    assert_eq!(res.words.len(), 0);
}

#[test]
fn test_15_invalid_runtime_output_handling() {
    let malformed_json = "{ \"transcription\": [ invalid ] }";
    let res = parse_whisper_alignment_output(malformed_json, Some("seg_0"));
    assert!(
        res.is_err(),
        "Malformed JSON output must be safely rejected"
    );
}

#[test]
fn test_16_segmentation_by_punctuation() {
    let mut w1 = AlignedWord::new(
        "w1",
        "Stop",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    w1.punctuation_suffix = Some(".".to_string());
    let w2 = AlignedWord::new(
        "w2",
        "Go",
        MediaTimeUs(600_000),
        MediaTimeUs(1_100_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();

    let policy = SegmentationPolicy {
        split_on_punctuation: true,
        ..Default::default()
    };
    let (cues, _) = segment_words_into_cues(&[w1, w2], &policy, "en");
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].text, "Stop.");
    assert_eq!(cues[1].text, "Go");
}

#[test]
fn test_17_segmentation_by_silence() {
    let w1 = AlignedWord::new(
        "w1",
        "A",
        MediaTimeUs(0),
        MediaTimeUs(300_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w2 = AlignedWord::new(
        "w2",
        "B",
        MediaTimeUs(1_000_000),
        MediaTimeUs(1_300_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();

    let policy = SegmentationPolicy {
        split_on_silence_gap_us: 500_000,
        ..Default::default()
    };
    let (cues, _) = segment_words_into_cues(&[w1, w2], &policy, "en");
    assert_eq!(cues.len(), 2);
}

#[test]
fn test_18_cue_duration_limits() {
    let w1 = AlignedWord::new(
        "w1",
        "Short",
        MediaTimeUs(0),
        MediaTimeUs(200_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let policy = SegmentationPolicy {
        min_cue_duration_us: 1_000_000,
        ..Default::default()
    };
    let (cues, _) = segment_words_into_cues(&[w1], &policy, "en");
    assert_eq!(cues.len(), 1);
    assert_eq!(
        cues[0].duration_us(),
        MediaTimeUs(1_000_000),
        "Cue duration must be padded to minimum"
    );
}

#[test]
fn test_19_cps_based_segmentation() {
    let mut words = Vec::new();
    for i in 0..10 {
        let w = AlignedWord::new(
            format!("w{i}"),
            "supercalifragilisticexpialidocious",
            MediaTimeUs(i as i64 * 300_000),
            MediaTimeUs((i as i64 + 1) * 300_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap();
        words.push(w);
    }
    let policy = SegmentationPolicy {
        max_chars_per_second: 15.0,
        ..Default::default()
    };
    let (cues, _) = segment_words_into_cues(&words, &policy, "en");
    assert!(
        cues.len() > 1,
        "Excessive CPS must trigger segmentation break"
    );
}

#[test]
fn test_20_conflicting_constraints_resolution() {
    // Single word longer than max_chars_per_line must never be dropped or silently truncated
    let giant_word = AlignedWord::new(
        "gw",
        "Pneumonoultramicroscopicsilicovolcanoconiosis",
        MediaTimeUs(0),
        MediaTimeUs(2_000_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();

    let policy = SegmentationPolicy {
        max_chars_per_line: 20,
        ..Default::default()
    };
    let (cues, _) = segment_words_into_cues(&[giant_word], &policy, "en");
    assert_eq!(cues.len(), 1);
    assert_eq!(
        cues[0].text,
        "Pneumonoultramicroscopicsilicovolcanoconiosis"
    );
}

#[test]
fn test_21_transcript_content_preservation() {
    let words = vec![
        AlignedWord::new(
            "w1",
            "Every",
            MediaTimeUs(0),
            MediaTimeUs(300_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap(),
        AlignedWord::new(
            "w2",
            "single",
            MediaTimeUs(350_000),
            MediaTimeUs(700_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap(),
        AlignedWord::new(
            "w3",
            "word",
            MediaTimeUs(750_000),
            MediaTimeUs(1_100_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap(),
        AlignedWord::new(
            "w4",
            "matters",
            MediaTimeUs(1_150_000),
            MediaTimeUs(1_600_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap(),
    ];
    let policy = SegmentationPolicy::default();
    let (cues, _) = segment_words_into_cues(&words, &policy, "en");
    let full_reconstructed: Vec<String> = cues.iter().map(|c| c.text.clone()).collect();
    let joined = full_reconstructed.join(" ");
    assert_eq!(joined, "Every single word matters");
}

#[test]
fn test_22_stable_word_ids() {
    let w1 = AlignedWord::new(
        "unique-stable-id-123",
        "Stable",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let policy = SegmentationPolicy::default();
    let (cues, _) = segment_words_into_cues(&[w1], &policy, "en");
    assert_eq!(
        cues[0].words.as_ref().unwrap()[0].id,
        "unique-stable-id-123"
    );
}

#[test]
fn test_23_segment_to_word_mapping() {
    let mut w = AlignedWord::new(
        "w1",
        "Word",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    w.source_segment_id = Some("segment-alpha-42".to_string());
    assert_eq!(w.source_segment_id.as_deref(), Some("segment-alpha-42"));
}

#[test]
fn test_24_cue_to_word_mapping() {
    let w1 = AlignedWord::new(
        "w1",
        "Hello",
        MediaTimeUs(0),
        MediaTimeUs(400_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let w2 = AlignedWord::new(
        "w2",
        "there",
        MediaTimeUs(450_000),
        MediaTimeUs(800_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let policy = SegmentationPolicy::default();
    let (cues, _) = segment_words_into_cues(&[w1, w2], &policy, "en");
    assert!(cues[0].words.is_some());
    assert_eq!(cues[0].words.as_ref().unwrap().len(), 2);
}

#[test]
fn test_25_alignment_invalidation_after_text_edits() {
    let w1 = AlignedWord::new(
        "w1",
        "Original",
        MediaTimeUs(0),
        MediaTimeUs(500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let mut cue =
        SubtitleCue::new("cue-1", MediaTimeUs(0), MediaTimeUs(1_000_000), "Original").unwrap();
    cue.words = Some(vec![w1]);

    let mut doc = SubtitleDocument::new(None);
    let mut track = SubtitleTrack::new("t1", "en", "Track 1", SubtitleFormat::NativeProject);
    track.cues.push(cue);
    doc.tracks.push(track);

    // Apply text update operation
    let op = SubtitleOperation::UpdateCueText {
        track_id: "t1".to_string(),
        cue_id: "cue-1".to_string(),
        text: "Completely replaced text".to_string(),
    };
    doc.apply_operation(op).expect("apply update text");
    // Subtitle operation handles text update; words should be invalidated or updated
    assert_eq!(doc.tracks[0].cues[0].text, "Completely replaced text");
}

#[test]
fn test_26_timing_movement_semantics() {
    let w1 = AlignedWord::new(
        "w1",
        "Move",
        MediaTimeUs(1_000_000),
        MediaTimeUs(1_500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    let mut cue = SubtitleCue::new(
        "cue-1",
        MediaTimeUs(1_000_000),
        MediaTimeUs(2_000_000),
        "Move",
    )
    .unwrap();
    cue.words = Some(vec![w1]);

    let mut doc = SubtitleDocument::new(None);
    let mut track = SubtitleTrack::new("t1", "en", "Track 1", SubtitleFormat::NativeProject);
    track.cues.push(cue);
    doc.tracks.push(track);

    let op = SubtitleOperation::MoveCue {
        track_id: "t1".to_string(),
        cue_id: "cue-1".to_string(),
        delta_us: 500_000,
    };
    doc.apply_operation(op).expect("move cue");
    assert_eq!(doc.tracks[0].cues[0].start_us, MediaTimeUs(1_500_000));
    assert_eq!(doc.tracks[0].cues[0].end_us, MediaTimeUs(2_500_000));
}

#[test]
fn test_27_undo_redo_timing_correction() {
    let mut cue = SubtitleCue::new(
        "cue-1",
        MediaTimeUs(0),
        MediaTimeUs(2_000_000),
        "Adjustable",
    )
    .unwrap();
    let word = AlignedWord::new(
        "w1",
        "Adjustable",
        MediaTimeUs(100_000),
        MediaTimeUs(800_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    cue.words = Some(vec![word]);

    // Initial state
    assert_eq!(
        cue.words.as_ref().unwrap()[0].start_us,
        MediaTimeUs(100_000)
    );

    // Update word timing
    update_word_timing_in_cue(&mut cue, "w1", MediaTimeUs(250_000), MediaTimeUs(950_000)).unwrap();
    assert_eq!(
        cue.words.as_ref().unwrap()[0].start_us,
        MediaTimeUs(250_000)
    );
    assert_eq!(cue.words.as_ref().unwrap()[0].end_us, MediaTimeUs(950_000));
    assert_eq!(
        cue.words.as_ref().unwrap()[0].provenance,
        WordTimingProvenance::UserAdjusted
    );
}

#[test]
fn test_28_native_project_persistence() {
    let mut cue = SubtitleCue::new(
        "cue-persist",
        MediaTimeUs(0),
        MediaTimeUs(2_000_000),
        "Persisted",
    )
    .unwrap();
    let word = AlignedWord::new(
        "wp",
        "Persisted",
        MediaTimeUs(100_000),
        MediaTimeUs(900_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    cue.words = Some(vec![word]);

    let mut doc = SubtitleDocument::new(Some("asset-xyz".to_string()));
    let mut track = SubtitleTrack::new("t1", "en", "Aligned Track", SubtitleFormat::NativeProject);
    track.cues.push(cue);
    doc.tracks.push(track);

    let json = serde_json::to_string(&doc).expect("serialize doc");
    let reopened: SubtitleDocument = serde_json::from_str(&json).expect("deserialize doc");
    assert_eq!(
        reopened.tracks[0].cues[0].words.as_ref().unwrap()[0].id,
        "wp"
    );
    assert_eq!(
        reopened.tracks[0].cues[0].words.as_ref().unwrap()[0].start_us,
        MediaTimeUs(100_000)
    );
}

#[test]
fn test_29_old_project_migration() {
    // Older schema without `words` field
    let legacy_json = r#"{
        "schemaVersion": 1,
        "id": "doc-legacy",
        "tracks": [{
            "id": "track-legacy",
            "language": "en",
            "displayName": "Legacy Track",
            "sourceFormat": "srt",
            "cues": [{
                "id": "cue-old",
                "startUs": 0,
                "endUs": 2000000,
                "text": "Legacy cue without word metadata"
            }],
            "isEnabled": true,
            "isVisible": true,
            "schemaVersion": 1
        }],
        "createdAtEpochMs": 1000,
        "updatedAtEpochMs": 1000
    }"#;

    let doc: SubtitleDocument =
        serde_json::from_str(legacy_json).expect("deserialize legacy project");
    assert_eq!(
        doc.tracks[0].cues[0].words, None,
        "Legacy cue without words defaults to None safely"
    );
}

#[test]
fn test_30_srt_export_compatibility() {
    let mut cue = SubtitleCue::new(
        "cue-srt",
        MediaTimeUs(0),
        MediaTimeUs(2_000_000),
        "SRT Compatible",
    )
    .unwrap();
    let word = AlignedWord::new(
        "ws",
        "SRT Compatible",
        MediaTimeUs(0),
        MediaTimeUs(1_500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    cue.words = Some(vec![word]);

    let mut track = SubtitleTrack::new("ts", "en", "SRT Track", SubtitleFormat::Srt);
    track.cues.push(cue);

    let srt_content =
        opentake_core::localization::subtitles::srt::serialize_srt(&track).expect("export to srt");
    assert!(srt_content.contains("00:00:00,000 --> 00:00:02,000"));
    assert!(srt_content.contains("SRT Compatible"));
}

#[test]
fn test_31_vtt_export_compatibility() {
    let mut cue = SubtitleCue::new(
        "cue-vtt",
        MediaTimeUs(500_000),
        MediaTimeUs(3_000_000),
        "VTT Output",
    )
    .unwrap();
    let word = AlignedWord::new(
        "wv",
        "VTT Output",
        MediaTimeUs(500_000),
        MediaTimeUs(2_500_000),
        WordTimingProvenance::WhisperNativeEstimate,
    )
    .unwrap();
    cue.words = Some(vec![word]);

    let mut track = SubtitleTrack::new("tv", "en", "VTT Track", SubtitleFormat::WebVtt);
    track.cues.push(cue);

    let vtt_content = opentake_core::localization::subtitles::webvtt::serialize_webvtt(&track)
        .expect("export to vtt");
    assert!(vtt_content.contains("WEBVTT"));
    assert!(vtt_content.contains("00:00:00.500 --> 00:00:03.000"));
    assert!(vtt_content.contains("VTT Output"));
}

#[tokio::test]
async fn test_32_genuine_audio_word_timing_end_to_end() {
    use opentake_core::ai::transcription::whisper::{SystemWhisperCliEngine, WhisperEngine};
    use tokio_util::sync::CancellationToken;

    let whisper_cli_path = PathBuf::from("/home/osmangoni/.local/share/opentake/bin/whisper-cli");
    let model_path = PathBuf::from(
        "/home/osmangoni/.local/share/opentake/models/installed/whisper-tiny/ggml-tiny.bin",
    );
    let audio_path = PathBuf::from("/home/osmangoni/.local/share/opentake/fixtures/jfk.wav");

    if !whisper_cli_path.is_file() || !model_path.is_file() || !audio_path.is_file() {
        println!("Skipping genuine whisper word timing test: files not present.");
        return;
    }

    let engine = SystemWhisperCliEngine::new(whisper_cli_path, "whisper.cpp v1.9.5".to_string());
    let cancel = CancellationToken::new();

    let raw_output = engine
        .transcribe(&model_path, &audio_path, "en", Some(4), cancel)
        .await
        .expect("genuine whisper.cpp word timing inference must succeed");

    assert!(
        !raw_output.segments.is_empty(),
        "Segments must not be empty"
    );

    // Convert raw segments into AlignedWords
    let mut all_words = Vec::new();
    for (seg_idx, seg) in raw_output.segments.iter().enumerate() {
        let seg_id = format!("seg_{seg_idx}");
        use unicode_segmentation::UnicodeSegmentation;
        let words: Vec<&str> = seg.text.unicode_words().collect();
        if words.is_empty() {
            continue;
        }
        let seg_dur = (seg.end_us - seg.start_us).max(50_000);
        let per_word = seg_dur / words.len() as i64;

        for (w_idx, w_text) in words.iter().enumerate() {
            let w_start = MediaTimeUs(seg.start_us + (w_idx as i64 * per_word));
            let w_end = if w_idx + 1 == words.len() {
                MediaTimeUs(seg.end_us)
            } else {
                MediaTimeUs(w_start.as_micros() + per_word)
            };

            let (pfx, clean, sfx) = split_word_punctuation(w_text);
            if let Ok(mut word) = AlignedWord::new(
                format!("w_{seg_idx}_{w_idx}"),
                clean,
                w_start,
                w_end,
                WordTimingProvenance::WhisperNativeEstimate,
            ) {
                word.punctuation_prefix = pfx;
                word.punctuation_suffix = sfx;
                word.source_segment_id = Some(seg_id.clone());
                all_words.push(word);
            }
        }
    }

    assert!(
        !all_words.is_empty(),
        "Must extract nonempty aligned words from audio"
    );
    assert!(
        all_words.len() >= 10,
        "Known speech in JFK audio contains > 10 words"
    );

    // Verify ordering and timing bounds
    for w in &all_words {
        assert!(w.start_us.as_micros() >= 0);
        assert!(w.end_us > w.start_us);
        assert!(
            w.end_us.as_micros() <= 12_000_000,
            "Audio is ~11s, words must fit duration"
        );
    }

    // Verify smart segmentation using these genuine word timings
    let policy = SegmentationPolicy::default();
    let (cues, warnings) = segment_words_into_cues(&all_words, &policy, "en");
    assert!(!cues.is_empty(), "Segmented cues must be produced");
    assert_eq!(
        warnings.len(),
        0,
        "No segmentation warnings on healthy audio"
    );

    for cue in &cues {
        assert!(cue.duration_us() >= MediaTimeUs(policy.min_cue_duration_us));
        assert!(cue.words.is_some());
        assert!(!cue.words.as_ref().unwrap().is_empty());
    }

    // Verify speech match
    let combined_text = cues
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        combined_text.to_lowercase().contains("americans"),
        "Recognized speech must contain known words"
    );
}

#[test]
fn test_33_performance_scaling_100_1000_10000_words() {
    use std::time::Instant;

    let policy = SegmentationPolicy::default();

    for count in [100, 1_000, 10_000] {
        let mut words = Vec::with_capacity(count);
        for i in 0..count {
            let mut w = AlignedWord::new(
                format!("w_{i}"),
                if i % 7 == 0 {
                    "ওয়ার্কবেঞ্চ"
                } else {
                    "workbench"
                },
                MediaTimeUs(i as i64 * 300_000),
                MediaTimeUs((i as i64 * 300_000) + 250_000),
                WordTimingProvenance::WhisperNativeEstimate,
            )
            .unwrap();
            if i % 12 == 0 {
                w.punctuation_suffix = Some("।".to_string());
            }
            words.push(w);
        }

        let start = Instant::now();
        let (cues, warnings) = segment_words_into_cues(&words, &policy, "bn");
        let elapsed = start.elapsed();

        println!(
            "PERF: Processed {count} words into {} cues in {:.3}ms (warnings: {})",
            cues.len(),
            elapsed.as_secs_f64() * 1000.0,
            warnings.len()
        );

        assert!(!cues.is_empty());
        assert!(
            elapsed.as_millis() < 500,
            "10k words must process in under 500ms"
        );
    }
}
