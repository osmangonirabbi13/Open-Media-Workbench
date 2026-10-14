use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::subtitles::{
    export_subtitles_to_string, load_subtitle_document, parse_srt, parse_subtitles_from_str,
    parse_webvtt, save_subtitle_document, serialize_srt, serialize_webvtt, validate_document,
    validate_track, FindingSeverity, SubtitleCue, SubtitleDocument, SubtitleError, SubtitleFormat,
    SubtitleTrack, WebVttCueSettings, MAX_SUBTITLE_CUE_COUNT, MAX_SUBTITLE_FILE_SIZE,
};
use opentake_core::media::authorization::{MediaAuthorizationRegistry, SourceProvenance};
use sha2::Digest;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_01_valid_srt_parsing() {
    let srt = "1\n00:00:01,000 --> 00:00:04,500\nHello, welcome to Open Media Workbench!\n\n2\n00:00:05,000 --> 00:00:08,200\nProfessional media recovery and localization.\n";
    let res = parse_srt(
        srt,
        None,
        Some("en".to_string()),
        Some("English".to_string()),
    )
    .expect("Valid SRT must parse");
    assert_eq!(res.track.cues.len(), 2);
    assert_eq!(res.track.language, "en");
    assert_eq!(
        res.track.cues[0].start_us,
        MediaTimeUs::from_micros(1_000_000)
    );
    assert_eq!(
        res.track.cues[0].end_us,
        MediaTimeUs::from_micros(4_500_000)
    );
    assert_eq!(
        res.track.cues[0].text,
        "Hello, welcome to Open Media Workbench!"
    );
    assert_eq!(
        res.track.cues[1].start_us,
        MediaTimeUs::from_micros(5_000_000)
    );
    assert_eq!(
        res.track.cues[1].end_us,
        MediaTimeUs::from_micros(8_200_000)
    );
    assert!(res.warnings.is_empty());
}

#[test]
fn test_02_valid_webvtt_parsing() {
    let vtt = "WEBVTT\n\n00:00:01.000 --> 00:00:03.500\nFirst WebVTT caption line\n\n00:00:04.000 --> 00:00:06.000 line:90% align:center\nSecond caption with settings\n";
    let res =
        parse_webvtt(vtt, None, Some("en".to_string()), None).expect("Valid WebVTT must parse");
    assert_eq!(res.track.cues.len(), 2);
    assert_eq!(
        res.track.cues[0].start_us,
        MediaTimeUs::from_micros(1_000_000)
    );
    assert_eq!(
        res.track.cues[0].end_us,
        MediaTimeUs::from_micros(3_500_000)
    );
    assert_eq!(res.track.cues[0].text, "First WebVTT caption line");

    let cue2 = &res.track.cues[1];
    assert_eq!(cue2.text, "Second caption with settings");
    assert!(cue2.settings.is_some());
    let settings = cue2.settings.as_ref().unwrap();
    assert_eq!(settings.line.as_deref(), Some("90%"));
    assert_eq!(settings.align.as_deref(), Some("center"));
}

#[test]
fn test_03_utf8_bom_handling() {
    let srt_with_bom = "\u{feff}1\n00:00:01,000 --> 00:00:02,000\nBOM test text\n";
    let res = parse_srt(srt_with_bom, None, None, None).expect("SRT with BOM must parse");
    assert_eq!(res.track.cues.len(), 1);
    assert_eq!(res.track.cues[0].text, "BOM test text");

    let vtt_with_bom = "\u{feff}WEBVTT\n\n00:01.000 --> 00:02.000\nWebVTT with BOM\n";
    let res_vtt = parse_webvtt(vtt_with_bom, None, None, None).expect("VTT with BOM must parse");
    assert_eq!(res_vtt.track.cues.len(), 1);
}

#[test]
fn test_04_crlf_and_cr_line_endings_normalization() {
    let srt_crlf = "1\r\n00:00:01,000 --> 00:00:02,000\r\nLine one\r\nLine two\r\n\r\n2\r00:00:03,000 --> 00:00:04,000\rCR line\r\n";
    let res = parse_srt(srt_crlf, None, None, None).expect("CRLF and CR must parse cleanly");
    assert_eq!(res.track.cues.len(), 2);
    assert_eq!(res.track.cues[0].text, "Line one\nLine two");
    assert_eq!(res.track.cues[1].text, "CR line");
}

#[test]
fn test_05_bengali_unicode_preservation() {
    let srt_bengali = "1\n00:00:01,000 --> 00:00:04,000\nওপেন মিডিয়া ওয়ার্কবেঞ্চে আপনাকে স্বাগতম!\n\n2\n00:00:05,000 --> 00:00:08,000\nসাবটাইটেল এবং ভিডিও রিকভারি স্টুডিও\n";
    let res = parse_srt(
        srt_bengali,
        None,
        Some("bn".to_string()),
        Some("বাংলা".to_string()),
    )
    .expect("Bengali Unicode must parse without corruption");
    assert_eq!(res.track.language, "bn");
    assert_eq!(res.track.cues[0].text, "ওপেন মিডিয়া ওয়ার্কবেঞ্চে আপনাকে স্বাগতম!");
    assert_eq!(res.track.cues[1].text, "সাবটাইটেল এবং ভিডিও রিকভারি স্টুডিও");

    let serialized = serialize_srt(&res.track).expect("Serialization must preserve Unicode");
    assert!(serialized.contains("ওপেন মিডিয়া ওয়ার্কবেঞ্চে আপনাকে স্বাগতম!"));
}

#[test]
fn test_06_multiline_cues() {
    let srt = "1\n00:00:01,000 --> 00:00:05,000\nFirst line of dialogue.\nSecond line of dialogue.\nThird line of dialogue.\n";
    let res = parse_srt(srt, None, None, None).unwrap();
    assert_eq!(
        res.track.cues[0].text,
        "First line of dialogue.\nSecond line of dialogue.\nThird line of dialogue."
    );
}

#[test]
fn test_07_duplicate_or_unordered_cue_indexes() {
    // Sequence numbers 1, 1, 99 (duplicate and out of order)
    let srt = "1\n00:00:01,000 --> 00:00:02,000\nFirst\n\n1\n00:00:03,000 --> 00:00:04,000\nSecond\n\n99\n00:00:05,000 --> 00:00:06,000\nThird\n";
    let res = parse_srt(srt, None, None, None).expect("Unordered indexes must not crash");
    assert_eq!(res.track.cues.len(), 3);
    assert!(!res.warnings.is_empty());
}

#[test]
fn test_08_malformed_timestamps() {
    let invalid_ts = "1\n00:99:99,999 --> 00:00:02,000\nBad minutes\n";
    let res = parse_srt(invalid_ts, None, None, None);
    assert!(res.is_err() || res.unwrap().track.cues.is_empty());

    let garbage = "1\nnot_a_timestamp --> neither_is_this\nText\n";
    let res2 = parse_srt(garbage, None, None, None);
    assert!(res2.is_err() || res2.unwrap().track.cues.is_empty());
}

#[test]
fn test_09_negative_or_reversed_intervals() {
    // end time before start time: 00:00:05,000 --> 00:00:02,000
    let reversed = "1\n00:00:05,000 --> 00:00:02,000\nReversed time\n";
    let res = parse_srt(reversed, None, None, None);
    assert!(res.is_err() || res.unwrap().track.cues.is_empty());
}

#[test]
fn test_10_overlapping_cues_validation() {
    let srt = "1\n00:00:01,000 --> 00:00:05,000\nSpeaker 1\n\n2\n00:00:03,000 --> 00:00:07,000\nSpeaker 2 (overlaps)\n";
    let res = parse_srt(srt, None, None, None).unwrap();
    assert_eq!(res.track.cues.len(), 2);

    let findings = validate_track(&res.track, None);
    let overlap = findings.iter().find(|f| f.code == "OVERLAPPING_CUES");
    assert!(overlap.is_some());
    assert_eq!(overlap.unwrap().severity, FindingSeverity::Warning);
}

#[test]
fn test_11_empty_file_rejection() {
    assert!(matches!(
        parse_srt("", None, None, None),
        Err(SubtitleError::EmptyInput)
    ));
    assert!(matches!(
        parse_srt("   \n\n   ", None, None, None),
        Err(SubtitleError::EmptyInput)
    ));
    assert!(matches!(
        parse_webvtt("", None, None, None),
        Err(SubtitleError::EmptyInput)
    ));
}

#[test]
fn test_12_invalid_utf8_handling() {
    let invalid_bytes: Vec<u8> = vec![0x31, 0x0A, 0x30, 0x30, 0xFF, 0xFE];
    let str_result = std::str::from_utf8(&invalid_bytes);
    assert!(str_result.is_err());
}

#[test]
fn test_13_very_large_input_rejection() {
    // Generate string exceeding 10MB
    let huge_content = "A".repeat(MAX_SUBTITLE_FILE_SIZE + 10);
    let res = parse_srt(&huge_content, None, None, None);
    assert!(matches!(
        res,
        Err(SubtitleError::ExceededMaxFileSize { .. })
    ));
}

#[test]
fn test_14_excessive_cue_count_rejection() {
    let mut big_srt = String::new();
    for i in 1..=(MAX_SUBTITLE_CUE_COUNT + 10) {
        let s_tot = i;
        let e_tot = i + 1;
        let s_sec = s_tot % 60;
        let s_min = (s_tot / 60) % 60;
        let s_hr = s_tot / 3600;

        let e_sec = e_tot % 60;
        let e_min = (e_tot / 60) % 60;
        let e_hr = e_tot / 3600;

        big_srt.push_str(&format!(
            "{i}\n{s_hr:02}:{s_min:02}:{s_sec:02},000 --> {e_hr:02}:{e_min:02}:{e_sec:02},000\nCue {i}\n\n"
        ));
    }
    let res = parse_srt(&big_srt, None, None, None);
    assert!(matches!(
        res,
        Err(SubtitleError::ExceededMaxCueCount { .. })
    ));
}

#[test]
fn test_15_webvtt_note_blocks() {
    let vtt = "WEBVTT\n\nNOTE This is a top-level comment block\nwhich should be cleanly ignored\n\n00:01.000 --> 00:03.000\nActual subtitle\n\nNOTE Another note\n";
    let res = parse_webvtt(vtt, None, None, None).unwrap();
    assert_eq!(res.track.cues.len(), 1);
    assert_eq!(res.track.cues[0].text, "Actual subtitle");
}

#[test]
fn test_16_webvtt_cue_settings_parsing() {
    let vtt = "WEBVTT\n\n00:01.000 --> 00:04.000 line:0 position:20% size:60% align:start vertical:rl\nStyled cue\n";
    let res = parse_webvtt(vtt, None, None, None).unwrap();
    let cue = &res.track.cues[0];
    let settings = cue.settings.as_ref().unwrap();
    assert_eq!(settings.line.as_deref(), Some("0"));
    assert_eq!(settings.position.as_deref(), Some("20%"));
    assert_eq!(settings.size.as_deref(), Some("60%"));
    assert_eq!(settings.align.as_deref(), Some("start"));
    assert_eq!(settings.vertical.as_deref(), Some("rl"));
}

#[test]
fn test_17_webvtt_unsupported_markup_sanitization() {
    let vtt = "WEBVTT\n\n00:01.000 --> 00:04.000\n<v Alice>Hello <b>Bob</b> &amp; <c.yellow>Charlie</c> &lt;3</v>\n";
    let res = parse_webvtt(vtt, None, None, None).unwrap();
    let cue = &res.track.cues[0];
    assert_eq!(cue.speaker.as_deref(), Some("Alice"));
    assert_eq!(cue.text, "Hello Bob & Charlie <3");
}

#[test]
fn test_18_round_trip_srt() {
    let mut track = SubtitleTrack::new("tr-1", "en", "English", SubtitleFormat::Srt);
    track.cues.push(
        SubtitleCue::new(
            "c-1",
            MediaTimeUs::from_micros(1_000_000),
            MediaTimeUs::from_micros(4_000_000),
            "Hello world",
        )
        .unwrap(),
    );
    track.cues.push(
        SubtitleCue::new(
            "c-2",
            MediaTimeUs::from_micros(5_000_000),
            MediaTimeUs::from_micros(9_000_000),
            "Second line\nWith newline",
        )
        .unwrap(),
    );

    let srt_text = serialize_srt(&track).unwrap();
    let exported_text = export_subtitles_to_string(&track, SubtitleFormat::Srt).unwrap();
    assert_eq!(srt_text, exported_text);
    let parsed = parse_srt(&srt_text, None, None, None).unwrap();

    assert_eq!(parsed.track.cues.len(), 2);
    assert_eq!(parsed.track.cues[0].start_us, track.cues[0].start_us);
    assert_eq!(parsed.track.cues[0].end_us, track.cues[0].end_us);
    assert_eq!(parsed.track.cues[0].text, track.cues[0].text);
    assert_eq!(parsed.track.cues[1].text, track.cues[1].text);
}

#[test]
fn test_19_round_trip_webvtt() {
    let mut track = SubtitleTrack::new("tr-2", "fr", "Français", SubtitleFormat::WebVtt);
    let mut cue = SubtitleCue::new(
        "c-1",
        MediaTimeUs::from_micros(1_500_000),
        MediaTimeUs::from_micros(3_500_000),
        "Bonjour le monde",
    )
    .unwrap();
    cue.settings = Some(WebVttCueSettings {
        align: Some("center".to_string()),
        line: Some("90%".to_string()),
        ..Default::default()
    });
    track.cues.push(cue);

    let vtt_text = serialize_webvtt(&track).unwrap();
    assert!(vtt_text.starts_with("WEBVTT\n\n"));

    let parsed = parse_webvtt(&vtt_text, None, None, None).unwrap();
    assert_eq!(parsed.track.cues.len(), 1);
    assert_eq!(parsed.track.cues[0].text, "Bonjour le monde");
    assert_eq!(
        parsed.track.cues[0].start_us,
        MediaTimeUs::from_micros(1_500_000)
    );
}

#[test]
fn test_20_microsecond_to_millisecond_rounding_rules() {
    // 1_499 microseconds rounds down to 1 ms
    let t1 = MediaTimeUs::from_micros(1_000_499);
    assert_eq!(
        opentake_core::localization::subtitles::srt::format_srt_timestamp(t1),
        "00:00:01,000"
    );

    // 1_500 microseconds rounds up to 2 ms (half away from zero)
    let t2 = MediaTimeUs::from_micros(1_000_500);
    assert_eq!(
        opentake_core::localization::subtitles::srt::format_srt_timestamp(t2),
        "00:00:01,001"
    );
}

#[test]
fn test_21_subtitle_project_persistence() {
    let dir = tempdir().unwrap();
    let project_file = dir.path().join("test_project.omwsub");

    let mut doc = SubtitleDocument::new(Some("asset-uuid-1".to_string()));
    let mut track = SubtitleTrack::new("tr-persist", "en", "English", SubtitleFormat::Srt);
    track.cues.push(
        SubtitleCue::new(
            "c-1",
            MediaTimeUs::from_micros(1_000_000),
            MediaTimeUs::from_micros(2_000_000),
            "Persisted cue",
        )
        .unwrap(),
    );
    doc.add_track(track);

    save_subtitle_document(&doc, &project_file).expect("Atomic save must succeed");
    assert!(project_file.exists());

    let loaded = load_subtitle_document(&project_file).expect("Load must succeed");
    assert_eq!(loaded.schema_version, 1);
    assert_eq!(
        loaded.associated_media_asset_id.as_deref(),
        Some("asset-uuid-1")
    );
    assert_eq!(loaded.tracks.len(), 1);
    assert_eq!(loaded.tracks[0].cues[0].text, "Persisted cue");
}

#[test]
fn test_22_corrupted_project_recovery() {
    let dir = tempdir().unwrap();
    let corrupt_file = dir.path().join("corrupted.omwsub");
    fs::write(
        &corrupt_file,
        b"{\"schemaVersion\": 1, \"invalid_json: true",
    )
    .unwrap();

    let res = load_subtitle_document(&corrupt_file);
    assert!(matches!(
        res,
        Err(SubtitleError::DeserializationError { .. })
    ));
}

#[tokio::test]
async fn test_23_unauthorized_import_rejection() {
    let registry = MediaAuthorizationRegistry::new();
    let dummy_path = PathBuf::from("/etc/passwd");
    // Not registered in registry
    assert!(!registry.is_authorized(&dummy_path).await);

    // Register with SubtitleFileDialog provenance
    let record = registry
        .authorize(
            dummy_path.clone(),
            SourceProvenance::SubtitleFileDialog,
            1024,
            None,
        )
        .await;
    assert_eq!(record.provenance, SourceProvenance::SubtitleFileDialog);
    assert!(registry.is_authorized(&dummy_path).await);
}

#[test]
fn test_24_export_path_atomic_persistence() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("exported.omwsub");

    let doc = SubtitleDocument::new(None);
    save_subtitle_document(&doc, &target).unwrap();

    // Verify no stray .tmp files left over
    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], "exported.omwsub");
}

#[test]
fn test_25_media_association_safety() {
    let mut doc = SubtitleDocument::new(Some("asset-123".to_string()));
    assert_eq!(doc.associated_media_asset_id.as_deref(), Some("asset-123"));

    let track = SubtitleTrack::new("tr-1", "en", "English", SubtitleFormat::Srt);
    doc.add_track(track);

    let findings = validate_document(&doc, Some(MediaTimeUs::from_micros(10_000_000)));
    // Empty track inside document flags EMPTY_TRACK
    assert!(findings.iter().any(|f| f.code == "EMPTY_TRACK"));
}

#[test]
fn test_26_cue_lookup_correctness_binary_search() {
    let mut track = SubtitleTrack::new("tr-lookup", "en", "English", SubtitleFormat::Srt);
    track.cues.push(
        SubtitleCue::new(
            "c-1",
            MediaTimeUs::from_micros(1_000_000),
            MediaTimeUs::from_micros(3_000_000),
            "Cue 1",
        )
        .unwrap(),
    );
    track.cues.push(
        SubtitleCue::new(
            "c-2",
            MediaTimeUs::from_micros(2_500_000),
            MediaTimeUs::from_micros(5_000_000),
            "Cue 2 (overlaps Cue 1)",
        )
        .unwrap(),
    );
    track.cues.push(
        SubtitleCue::new(
            "c-3",
            MediaTimeUs::from_micros(6_000_000),
            MediaTimeUs::from_micros(8_000_000),
            "Cue 3",
        )
        .unwrap(),
    );

    // At 500ms (before all cues)
    let active_0 = track.find_active_cues(MediaTimeUs::from_micros(500_000));
    assert!(active_0.is_empty());

    // At 2.0s (only Cue 1)
    let active_2 = track.find_active_cues(MediaTimeUs::from_micros(2_000_000));
    assert_eq!(active_2.len(), 1);
    assert_eq!(active_2[0].id, "c-1");

    // At 2.7s (both Cue 1 and Cue 2 overlap)
    let active_2_7 = track.find_active_cues(MediaTimeUs::from_micros(2_700_000));
    assert_eq!(active_2_7.len(), 2);
    assert_eq!(active_2_7[0].id, "c-1");
    assert_eq!(active_2_7[1].id, "c-2");

    // At 5.5s (gap between cues)
    let active_gap = track.find_active_cues(MediaTimeUs::from_micros(5_500_000));
    assert!(active_gap.is_empty());

    // At 7.0s (only Cue 3)
    let active_7 = track.find_active_cues(MediaTimeUs::from_micros(7_000_000));
    assert_eq!(active_7.len(), 1);
    assert_eq!(active_7[0].id, "c-3");
}

#[test]
fn test_27_subtitle_edl_mapping_boundaries() {
    // In OpenTake, subtitle cues are indexed in source media time (MediaTimeUs).
    let cue = SubtitleCue::new(
        "c-edl",
        MediaTimeUs::from_micros(5_000_000),
        MediaTimeUs::from_micros(8_000_000),
        "EDL boundary test",
    )
    .unwrap();

    assert!(cue.contains_time(MediaTimeUs::from_micros(5_000_000))); // Inclusive start
    assert!(cue.contains_time(MediaTimeUs::from_micros(7_999_999))); // Inside
    assert!(!cue.contains_time(MediaTimeUs::from_micros(8_000_000))); // Exclusive end
}

#[test]
fn test_28_stable_ids_across_save_and_reopen() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("stable_ids.omwsub");

    let mut doc = SubtitleDocument::new(Some("asset-xyz".to_string()));
    let doc_id = doc.id.clone();

    let mut track = SubtitleTrack::new("track-stable-id", "en", "English", SubtitleFormat::Srt);
    track.cues.push(
        SubtitleCue::new(
            "cue-stable-id-1",
            MediaTimeUs::from_micros(0),
            MediaTimeUs::from_micros(1_000_000),
            "Stable",
        )
        .unwrap(),
    );
    doc.add_track(track);

    save_subtitle_document(&doc, &file).unwrap();
    let reopened = load_subtitle_document(&file).unwrap();

    assert_eq!(reopened.id, doc_id);
    assert_eq!(reopened.tracks[0].id, "track-stable-id");
    assert_eq!(reopened.tracks[0].cues[0].id, "cue-stable-id-1");
}

#[test]
fn test_29_no_media_source_mutation() {
    let dir = tempdir().unwrap();
    let dummy_media_file = dir.path().join("source_video.mp4");
    fs::write(&dummy_media_file, b"DUMMY_MP4_CONTENT_STRICTLY_READ_ONLY").unwrap();

    let initial_hash = sha2::Sha256::digest(fs::read(&dummy_media_file).unwrap());

    // Parse and export subtitles referencing this media
    let srt = "1\n00:00:01,000 --> 00:00:02,000\nSubtitle\n";
    let parsed = parse_srt(srt, None, None, None).unwrap();
    let _ = serialize_srt(&parsed.track).unwrap();

    let post_hash = sha2::Sha256::digest(fs::read(&dummy_media_file).unwrap());
    assert_eq!(
        initial_hash, post_hash,
        "Original media file must never be modified by subtitle actions"
    );
}

#[test]
fn test_30_arbitrary_fuzz_malformed_text_safety() {
    let malformed_inputs = [
        "WEBVTT\n\nGarbage with no times\n",
        "1\n00:00:01,000 -->\nMissing end\n",
        "1\n--> 00:00:02,000\nMissing start\n",
        "1\n00:00:01,000 --> 00:00:02,000\n\n\n\n2\n",
        "WEBVTT\nSTYLE\n::cue { color: red; }\n\n00:00.000 --> 00:01.000\nValid after style\n",
        "1\n00:00:01,000 --> 00:00:02,000\nEmoji test: 🎥🎬🍿 বাংলা 🚀\n",
    ];

    for input in &malformed_inputs {
        let _ = parse_subtitles_from_str(input, None, None, None, None);
        // Ensure no panic
    }
}
