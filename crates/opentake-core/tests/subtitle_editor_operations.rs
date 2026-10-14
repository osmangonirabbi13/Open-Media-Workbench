use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::{
    load_subtitle_document, save_subtitle_document, SubtitleCue, SubtitleDocument, SubtitleError,
    SubtitleFormat, SubtitleOperation, SubtitleTrack,
};
use tempfile::tempdir;

fn sample_track() -> SubtitleTrack {
    let mut track = SubtitleTrack::new("track-1", "en", "English (Original)", SubtitleFormat::Srt);
    let cue1 = SubtitleCue::new(
        "cue-1",
        MediaTimeUs(1_000_000),
        MediaTimeUs(3_000_000),
        "Hello world",
    )
    .unwrap();
    let cue2 = SubtitleCue::new(
        "cue-2",
        MediaTimeUs(4_000_000),
        MediaTimeUs(7_000_000),
        "This is Open Media Workbench",
    )
    .unwrap();
    track.cues.push(cue1);
    track.cues.push(cue2);
    track
}

fn sample_doc() -> SubtitleDocument {
    let mut doc = SubtitleDocument::new(Some("asset-xyz".to_string()));
    doc.add_track(sample_track());
    doc
}

#[test]
fn test_01_add_cue() {
    let mut track = sample_track();
    let new_cue = SubtitleCue::new(
        "cue-3",
        MediaTimeUs(8_000_000),
        MediaTimeUs(10_000_000),
        "New Subtitle",
    )
    .unwrap();

    let op = SubtitleOperation::AddCue {
        track_id: track.id.clone(),
        cue: new_cue,
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues.len(), 3);
    assert_eq!(track.cues[2].id, "cue-3");
}

#[test]
fn test_02_delete_cue() {
    let mut track = sample_track();
    let op = SubtitleOperation::DeleteCue {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues.len(), 1);
    assert_eq!(track.cues[0].id, "cue-2");

    // Deleting nonexistent cue returns CueNotFound
    let err = track.apply_operation(&op).unwrap_err();
    assert!(matches!(err, SubtitleError::CueNotFound(_)));
}

#[test]
fn test_03_text_update() {
    let mut track = sample_track();
    let op = SubtitleOperation::UpdateCueText {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        text: "Updated subtitle dialogue line".to_string(),
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues[0].text, "Updated subtitle dialogue line");
}

#[test]
fn test_04_start_end_timing_edits() {
    let mut track = sample_track();
    let op = SubtitleOperation::SetCueTiming {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        start_us: MediaTimeUs(1_500_000),
        end_us: MediaTimeUs(3_500_000),
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues[0].start_us, MediaTimeUs(1_500_000));
    assert_eq!(track.cues[0].end_us, MediaTimeUs(3_500_000));
}

#[test]
fn test_05_move_cue() {
    let mut track = sample_track();
    // Move forward by 500ms (500_000 µs)
    let op = SubtitleOperation::MoveCue {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        delta_us: 500_000,
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues[0].start_us, MediaTimeUs(1_500_000));
    assert_eq!(track.cues[0].end_us, MediaTimeUs(3_500_000));

    // Move backward by 1,000,000 µs
    let op2 = SubtitleOperation::MoveCue {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        delta_us: -1_000_000,
    };
    assert!(track.apply_operation(&op2).is_ok());
    assert_eq!(track.cues[0].start_us, MediaTimeUs(500_000));
    assert_eq!(track.cues[0].end_us, MediaTimeUs(2_500_000));
}

#[test]
fn test_06_resize_boundaries() {
    let mut track = sample_track();
    // Resize left edge
    let op_start = SubtitleOperation::ResizeCueStart {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        new_start_us: MediaTimeUs(800_000),
    };
    assert!(track.apply_operation(&op_start).is_ok());
    assert_eq!(track.cues[0].start_us, MediaTimeUs(800_000));
    assert_eq!(track.cues[0].end_us, MediaTimeUs(3_000_000));

    // Resize right edge
    let op_end = SubtitleOperation::ResizeCueEnd {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        new_end_us: MediaTimeUs(3_800_000),
    };
    assert!(track.apply_operation(&op_end).is_ok());
    assert_eq!(track.cues[0].end_us, MediaTimeUs(3_800_000));
}

#[test]
fn test_07_split_cue() {
    let mut track = sample_track();
    // Split cue-2 (4s to 7s, "This is Open Media Workbench") at 5.5s
    let op = SubtitleOperation::SplitCue {
        track_id: track.id.clone(),
        cue_id: "cue-2".to_string(),
        split_time_us: MediaTimeUs(5_500_000),
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues.len(), 3);

    // First half ends at 5.5s
    assert_eq!(track.cues[1].id, "cue-2");
    assert_eq!(track.cues[1].start_us, MediaTimeUs(4_000_000));
    assert_eq!(track.cues[1].end_us, MediaTimeUs(5_500_000));

    // Second half starts at 5.5s and ends at 7.0s
    assert_eq!(track.cues[2].start_us, MediaTimeUs(5_500_000));
    assert_eq!(track.cues[2].end_us, MediaTimeUs(7_000_000));
    assert!(!track.cues[1].text.is_empty());
    assert!(!track.cues[2].text.is_empty());
}

#[test]
fn test_08_merge_cues() {
    let mut track = sample_track();
    let op = SubtitleOperation::MergeCues {
        track_id: track.id.clone(),
        first_cue_id: "cue-1".to_string(),
        second_cue_id: "cue-2".to_string(),
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues.len(), 1);
    assert_eq!(track.cues[0].start_us, MediaTimeUs(1_000_000));
    assert_eq!(track.cues[0].end_us, MediaTimeUs(7_000_000));
    assert!(track.cues[0].text.contains("Hello world"));
    assert!(track.cues[0].text.contains("This is Open Media Workbench"));
}

#[test]
fn test_09_duplicate_cue() {
    let mut track = sample_track();
    let op = SubtitleOperation::DuplicateCue {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues.len(), 3);
    // Duplicated cue placed right after cue-1 (starts at 3s, duration 2s -> ends at 5s)
    let dup = track
        .cues
        .iter()
        .find(|c| c.id != "cue-1" && c.id != "cue-2")
        .unwrap();
    assert_eq!(dup.start_us, MediaTimeUs(3_000_000));
    assert_eq!(dup.end_us, MediaTimeUs(5_000_000));
    assert_eq!(dup.text, "Hello world");
}

#[test]
fn test_10_batch_shift() {
    let mut track = sample_track();
    let op = SubtitleOperation::ShiftSelectedCues {
        track_id: track.id.clone(),
        cue_ids: vec!["cue-1".to_string(), "cue-2".to_string()],
        delta_us: 1_000_000,
    };
    assert!(track.apply_operation(&op).is_ok());
    assert_eq!(track.cues[0].start_us, MediaTimeUs(2_000_000));
    assert_eq!(track.cues[0].end_us, MediaTimeUs(4_000_000));
    assert_eq!(track.cues[1].start_us, MediaTimeUs(5_000_000));
    assert_eq!(track.cues[1].end_us, MediaTimeUs(8_000_000));
}

#[test]
fn test_11_negative_time_rejection() {
    let mut track = sample_track();
    // Shifting cue-1 (starts at 1s) by -1.5s (-1_500_000 µs) should be rejected
    let op = SubtitleOperation::MoveCue {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        delta_us: -1_500_000,
    };
    let err = track.apply_operation(&op).unwrap_err();
    assert!(matches!(err, SubtitleError::NegativeTime { .. }));

    // Batch shift where ONE cue becomes negative must atomically reject without mutating anything!
    let batch_op = SubtitleOperation::ShiftSelectedCues {
        track_id: track.id.clone(),
        cue_ids: vec!["cue-1".to_string(), "cue-2".to_string()],
        delta_us: -1_500_000,
    };
    let batch_err = track.apply_operation(&batch_op).unwrap_err();
    assert!(matches!(batch_err, SubtitleError::NegativeTime { .. }));
    // Verify cues were not mutated
    assert_eq!(track.cues[0].start_us, MediaTimeUs(1_000_000));
    assert_eq!(track.cues[1].start_us, MediaTimeUs(4_000_000));
}

#[test]
fn test_12_overflow_protection() {
    let mut track = sample_track();
    let op = SubtitleOperation::MoveCue {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        delta_us: i64::MAX,
    };
    let err = track.apply_operation(&op).unwrap_err();
    assert!(matches!(err, SubtitleError::OperationError(_)));
}

#[test]
fn test_13_stable_cue_ids() {
    let mut track = sample_track();
    let initial_cue1_id = track.cues[0].id.clone();
    let initial_cue2_id = track.cues[1].id.clone();

    // Perform timing edit and move
    track
        .apply_operation(&SubtitleOperation::SetCueTiming {
            track_id: track.id.clone(),
            cue_id: "cue-1".to_string(),
            start_us: MediaTimeUs(500_000),
            end_us: MediaTimeUs(2_500_000),
        })
        .unwrap();

    track
        .apply_operation(&SubtitleOperation::MoveCue {
            track_id: track.id.clone(),
            cue_id: "cue-2".to_string(),
            delta_us: 100_000,
        })
        .unwrap();

    assert_eq!(track.cues[0].id, initial_cue1_id);
    assert_eq!(track.cues[1].id, initial_cue2_id);
}

#[test]
fn test_14_multi_track_isolation() {
    let mut doc = sample_doc();
    let mut track2 = SubtitleTrack::new("track-2", "bn", "Bengali", SubtitleFormat::WebVtt);
    track2.cues.push(
        SubtitleCue::new(
            "cue-bn-1",
            MediaTimeUs(0),
            MediaTimeUs(2_000_000),
            "বাংলা সাবটাইটেল",
        )
        .unwrap(),
    );
    doc.add_track(track2);

    // Apply op to track-1
    doc.apply_operation(SubtitleOperation::UpdateCueText {
        track_id: "track-1".to_string(),
        cue_id: "cue-1".to_string(),
        text: "Changed english text".to_string(),
    })
    .unwrap();

    // Verify track-2 is completely unaffected
    assert_eq!(doc.tracks[1].cues[0].text, "বাংলা সাবটাইটেল");
    assert_eq!(doc.tracks[0].cues[0].text, "Changed english text");
}

#[test]
fn test_15_microsecond_precision() {
    let mut track = sample_track();
    let exact_us = 1_234_567;
    let end_exact_us = 3_456_789;

    track
        .apply_operation(&SubtitleOperation::SetCueTiming {
            track_id: track.id.clone(),
            cue_id: "cue-1".to_string(),
            start_us: MediaTimeUs(exact_us),
            end_us: MediaTimeUs(end_exact_us),
        })
        .unwrap();

    assert_eq!(track.cues[0].start_us.as_micros(), 1_234_567);
    assert_eq!(track.cues[0].end_us.as_micros(), 3_456_789);
    assert_eq!(
        track.cues[0].duration_us().as_micros(),
        end_exact_us - exact_us
    );
}

#[test]
fn test_16_edl_mapping_behavior() {
    let track = sample_track();
    // Subtitle interval [1s, 3s)
    let cue = &track.cues[0];
    assert!(cue.contains_time(MediaTimeUs(1_000_000)));
    assert!(cue.contains_time(MediaTimeUs(2_000_000)));
    assert!(!cue.contains_time(MediaTimeUs(3_000_000))); // Exclusive end
    assert!(!cue.contains_time(MediaTimeUs(999_999)));
}

#[test]
fn test_17_undo_redo_transaction_invariants() {
    let mut track = sample_track();
    let original_text = track.cues[0].text.clone();

    // Operation: text update
    let forward_op = SubtitleOperation::UpdateCueText {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        text: "Brand new line".to_string(),
    };
    // Inverse operation: restore original text
    let inverse_op = SubtitleOperation::UpdateCueText {
        track_id: track.id.clone(),
        cue_id: "cue-1".to_string(),
        text: original_text.clone(),
    };

    track.apply_operation(&forward_op).unwrap();
    assert_eq!(track.cues[0].text, "Brand new line");

    // Apply inverse (Undo)
    track.apply_operation(&inverse_op).unwrap();
    assert_eq!(track.cues[0].text, original_text);

    // Apply forward again (Redo)
    track.apply_operation(&forward_op).unwrap();
    assert_eq!(track.cues[0].text, "Brand new line");
}

#[test]
fn test_18_unicode_preservation() {
    let mut track = sample_track();
    let bengali_text = "ওপেন মিডিয়া ওয়ার্কবেঞ্চ — সাবটাইটেল এডিটর\nবাংলা ও মাল্টিলাইন সাপোর্ট";

    track
        .apply_operation(&SubtitleOperation::UpdateCueText {
            track_id: track.id.clone(),
            cue_id: "cue-1".to_string(),
            text: bengali_text.to_string(),
        })
        .unwrap();

    assert_eq!(track.cues[0].text, bengali_text);

    // Split Bengali cue safely
    track
        .apply_operation(&SubtitleOperation::SplitCue {
            track_id: track.id.clone(),
            cue_id: "cue-1".to_string(),
            split_time_us: MediaTimeUs(2_000_000),
        })
        .unwrap();

    assert_eq!(track.cues[0].text, "ওপেন মিডিয়া ওয়ার্কবেঞ্চ — সাবটাইটেল এডিটর");
    assert_eq!(track.cues[1].text, "বাংলা ও মাল্টিলাইন সাপোর্ট");
}

#[test]
fn test_19_overlapping_cue_queries() {
    let mut track = sample_track();
    // Add overlapping cue: 2s to 5s
    let overlap_cue = SubtitleCue::new(
        "cue-overlap",
        MediaTimeUs(2_000_000),
        MediaTimeUs(5_000_000),
        "Overlapping dialogue",
    )
    .unwrap();
    track
        .apply_operation(&SubtitleOperation::AddCue {
            track_id: track.id.clone(),
            cue: overlap_cue,
        })
        .unwrap();

    // At 2.5s: both cue-1 [1s, 3s) and cue-overlap [2s, 5s) are active concurrently!
    let active = track.find_active_cues(MediaTimeUs(2_500_000));
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].id, "cue-1");
    assert_eq!(active[1].id, "cue-overlap");
}

#[test]
fn test_20_save_reopen_correctness() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test_project.omwsub");

    let mut doc = sample_doc();
    doc.apply_operation(SubtitleOperation::UpdateCueText {
        track_id: "track-1".to_string(),
        cue_id: "cue-1".to_string(),
        text: "Persisted edit test".to_string(),
    })
    .unwrap();

    // Save
    assert!(save_subtitle_document(&doc, &path).is_ok());
    assert!(path.exists());

    // Load back
    let reopened = load_subtitle_document(&path).unwrap();
    assert_eq!(reopened.id, doc.id);
    assert_eq!(reopened.tracks[0].cues[0].text, "Persisted edit test");
    assert_eq!(reopened.tracks[0].cues[0].start_us, MediaTimeUs(1_000_000));
}
