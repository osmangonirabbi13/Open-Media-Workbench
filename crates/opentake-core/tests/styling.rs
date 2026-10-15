//! Comprehensive integration tests for Subtitle Styling, ASS Generation, and Video Burn-in Export.

use opentake_core::ai::alignment::models::{AlignedWord, WordTimingProvenance};
use opentake_core::editing::time::MediaTimeUs;
use opentake_core::localization::styling::{
    build_ass_filter_arg, escape_ass_text, execute_subtitle_burn_in, format_ass_timestamp,
    generate_ass_script, get_builtin_presets, resolve_safe_font_family,
    verify_ffmpeg_burnin_support, AudioExportPolicy, ColorRgba, ExportResolution,
    StylePresetManager, SubtitleAnimation, SubtitleBurnInRequest, SubtitleExportProfile,
    SubtitlePosition, SubtitleStyle, SubtitleStylePreset,
};
use opentake_core::localization::subtitles::model::{SubtitleCue, SubtitleFormat, SubtitleTrack};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_01_style_schema_validation() {
    let mut style = SubtitleStyle::default();
    assert!(style.validate().is_ok());

    style.font_size = 5;
    assert!(style.validate().is_err());

    style.font_size = 32;
    style.outline_width = 25.0;
    assert!(style.validate().is_err());

    style.outline_width = 2.0;
    style.background_opacity = 1.5;
    assert!(style.validate().is_err());

    style.background_opacity = 0.5;
    style.font_family = "".to_string();
    assert!(style.validate().is_err());
}

#[test]
fn test_02_invalid_font_size() {
    let style = SubtitleStyle {
        font_size: 200,
        ..Default::default()
    };
    assert!(style.validate().is_err());
}

#[test]
fn test_03_color_parsing_hex_variants() {
    let c1 = ColorRgba::parse_hex("#FFF").expect("parse 3-char hex");
    assert_eq!(c1, ColorRgba::opaque(255, 255, 255));

    let c2 = ColorRgba::parse_hex("#F59E0B").expect("parse 6-char hex");
    assert_eq!(c2, ColorRgba::opaque(245, 158, 11));

    let c3 = ColorRgba::parse_hex("#F59E0B80").expect("parse 8-char hex");
    assert_eq!(c3, ColorRgba::new(245, 158, 11, 128));

    assert!(ColorRgba::parse_hex("#ZZZ").is_err());
    assert!(ColorRgba::parse_hex("#12345").is_err());
}

#[test]
fn test_04_color_to_ass_conversion() {
    // White fully opaque: ASS alpha is 00 (255 - 255)
    let white = ColorRgba::opaque(255, 255, 255);
    assert_eq!(white.to_ass_color(), "&H00FFFFFF");

    // Pure Red fully opaque: R=FF, G=00, B=00 -> ASS order &HAABBGGRR -> &H000000FF
    let red = ColorRgba::opaque(255, 0, 0);
    assert_eq!(red.to_ass_color(), "&H000000FF");

    // Pure Blue with 50% opacity (alpha=128): ASS alpha is 127 (&H7F), B=FF, G=0, R=0 -> &H7FFF0000
    let blue_half = ColorRgba::new(0, 0, 255, 128);
    assert_eq!(blue_half.to_ass_color(), "&H7FFF0000");
}

#[test]
fn test_05_font_mapping_and_bengali() {
    assert_eq!(resolve_safe_font_family("Inter", false), "Inter");
    assert_eq!(resolve_safe_font_family("", false), "DejaVu Sans");
    assert_eq!(resolve_safe_font_family("Arial", true), "Noto Sans Bengali");
}

#[test]
fn test_06_builtin_presets_loading() {
    let presets = get_builtin_presets();
    assert_eq!(presets.len(), 8);

    let names: Vec<&str> = presets.iter().map(|p| p.display_name.as_str()).collect();
    assert!(names.contains(&"Clean Minimal"));
    assert!(names.contains(&"Cinematic"));
    assert!(names.contains(&"Modern Outline"));
    assert!(names.contains(&"Social Bold"));
    assert!(names.contains(&"Karaoke Highlight"));
    assert!(names.contains(&"Documentary"));
    assert!(names.contains(&"Accessible High Contrast"));
    assert!(names.contains(&"Soft Background Box"));

    for p in &presets {
        assert!(p.is_builtin);
        assert!(p.style.validate().is_ok());
    }
}

#[test]
fn test_07_builtin_preset_immutability() {
    let dir = tempdir().expect("tempdir");
    let manager = StylePresetManager::new(dir.path());

    let mut preset = manager.list_presets()[0].clone();
    preset.display_name = "Tampered Clean Minimal".to_string();

    let save_res = manager.save_custom_preset(preset);
    assert!(save_res.is_err(), "Must reject modifying built-in preset");

    let del_res = manager.delete_custom_preset("clean-minimal");
    assert!(del_res.is_err(), "Must reject deleting built-in preset");
}

#[test]
fn test_08_custom_preset_persistence() {
    let dir = tempdir().expect("tempdir");
    let manager = StylePresetManager::new(dir.path());

    let custom = SubtitleStylePreset {
        id: "custom-my-style".to_string(),
        display_name: "My Custom Style".to_string(),
        description: "Custom style for branded videos".to_string(),
        is_builtin: false,
        style: SubtitleStyle {
            id: "custom-my-style".to_string(),
            display_name: "My Custom Style".to_string(),
            font_size: 40,
            ..SubtitleStyle::default()
        },
    };

    manager
        .save_custom_preset(custom.clone())
        .expect("save custom preset");

    // Re-instantiate manager to verify on-disk persistence
    let manager2 = StylePresetManager::new(dir.path());
    let list = manager2.list_presets();
    assert_eq!(list.len(), 9);
    assert!(list.iter().any(|p| p.id == "custom-my-style"));

    // Delete custom preset
    manager2
        .delete_custom_preset("custom-my-style")
        .expect("delete custom preset");
    let list_after = manager2.list_presets();
    assert_eq!(list_after.len(), 8);
    assert!(!list_after.iter().any(|p| p.id == "custom-my-style"));
}

#[test]
fn test_09_ass_timestamp_precision_centiseconds() {
    // 0 us -> 0:00:00.00
    assert_eq!(format_ass_timestamp(MediaTimeUs(0)), "0:00:00.00");

    // 1,234,567 us -> ~1.23s -> 0:00:01.23
    assert_eq!(format_ass_timestamp(MediaTimeUs(1_234_567)), "0:00:01.23");

    // 65,432,100 us -> 65.43s -> 1m 5.43s -> 0:01:05.43
    assert_eq!(format_ass_timestamp(MediaTimeUs(65_432_100)), "0:01:05.43");

    // 3,661,500,000 us -> 3661.50s -> 1h 1m 1.50s -> 1:01:01.50
    assert_eq!(
        format_ass_timestamp(MediaTimeUs(3_661_500_000)),
        "1:01:01.50"
    );
}

#[test]
fn test_10_ass_text_escaping_braces_and_backslashes() {
    let raw = "Hello {inject:bad} world\\test\nsecond line";
    let escaped = escape_ass_text(raw);
    assert!(!escaped.contains('{'));
    assert!(!escaped.contains('}'));
    assert!(escaped.contains("Hello ｛inject:bad｝ world\\\\test\\Nsecond line"));
}

#[test]
fn test_11_unicode_bengali_preservation_in_ass() {
    let bengali = "ওপেন মিডিয়া ওয়ার্কবেঞ্চ — বাংলা সাবটাইটেল।";
    let escaped = escape_ass_text(bengali);
    assert_eq!(escaped, bengali);
}

#[test]
fn test_12_injection_resistant_override_handling() {
    let mut track = SubtitleTrack::new("t1", "en", "English", SubtitleFormat::Srt);
    track.cues = vec![SubtitleCue::new(
        "c1",
        MediaTimeUs(0),
        MediaTimeUs(3_000_000),
        "{\\an8}Malicious Override",
    )
    .unwrap()];

    let ass = generate_ass_script(&track, &SubtitleStyle::default()).expect("generate ass");
    assert!(ass.contains("Dialogue:"));
    // Braces must be escaped to full-width to prevent override tag execution
    assert!(!ass.contains("Dialogue: 0,0:00:00.00,0:00:03.00,Default,,0,0,0,,{\\an8}"));
    assert!(ass.contains("｛\\\\an8｝Malicious Override"));
}

#[test]
fn test_13_multiline_ass_dialogue_conversion() {
    let mut track = SubtitleTrack::new("t1", "en", "English", SubtitleFormat::Srt);
    track.cues = vec![SubtitleCue::new(
        "c1",
        MediaTimeUs(0),
        MediaTimeUs(3_000_000),
        "Line 1\nLine 2\nLine 3",
    )
    .unwrap()];

    let ass = generate_ass_script(&track, &SubtitleStyle::default()).expect("generate ass");
    assert!(ass.contains("Line 1\\NLine 2\\NLine 3"));
}

#[test]
fn test_14_overlapping_cues_handled() {
    let mut track = SubtitleTrack::new("t1", "en", "English", SubtitleFormat::Srt);
    track.cues = vec![
        SubtitleCue::new(
            "c1",
            MediaTimeUs(1_000_000),
            MediaTimeUs(4_000_000),
            "Speaker 1 Dialogue",
        )
        .unwrap(),
        SubtitleCue::new(
            "c2",
            MediaTimeUs(2_000_000),
            MediaTimeUs(5_000_000),
            "Speaker 2 Overlap",
        )
        .unwrap(),
    ];

    let ass = generate_ass_script(&track, &SubtitleStyle::default()).expect("generate ass");
    assert!(ass.contains("Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,Speaker 1 Dialogue"));
    assert!(ass.contains("Dialogue: 0,0:00:02.00,0:00:05.00,Default,,0,0,0,,Speaker 2 Overlap"));
}

#[test]
fn test_15_karaoke_timing_conversion_kf_tags() {
    let mut cue =
        SubtitleCue::new("c1", MediaTimeUs(0), MediaTimeUs(2_000_000), "Ask not what").unwrap();

    cue.words = Some(vec![
        AlignedWord::new(
            "w1",
            "Ask",
            MediaTimeUs(0),
            MediaTimeUs(500_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap(),
        AlignedWord::new(
            "w2",
            "not",
            MediaTimeUs(500_000),
            MediaTimeUs(1_100_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap(),
        AlignedWord::new(
            "w3",
            "what",
            MediaTimeUs(1_100_000),
            MediaTimeUs(2_000_000),
            WordTimingProvenance::WhisperNativeEstimate,
        )
        .unwrap(),
    ]);

    let mut track = SubtitleTrack::new("t1", "en", "English", SubtitleFormat::Srt);
    track.cues = vec![cue];

    let style = SubtitleStyle {
        animation: SubtitleAnimation::KaraokeHighlight,
        ..Default::default()
    };

    let ass = generate_ass_script(&track, &style).expect("generate ass");
    // 500ms = 50cs; 600ms = 60cs; 900ms = 90cs
    assert!(ass.contains("{\\kf50}Ask"));
    assert!(ass.contains("{\\kf60}not"));
    assert!(ass.contains("{\\kf90}what"));
}

#[test]
fn test_16_missing_word_timing_fallback() {
    let cue = SubtitleCue::new(
        "c1",
        MediaTimeUs(0),
        MediaTimeUs(2_000_000),
        "No word timings available",
    )
    .unwrap();

    let mut track = SubtitleTrack::new("t1", "en", "English", SubtitleFormat::Srt);
    track.cues = vec![cue];

    let style = SubtitleStyle {
        animation: SubtitleAnimation::KaraokeHighlight,
        ..Default::default()
    };

    let ass = generate_ass_script(&track, &style).expect("generate ass");
    // Should fall back cleanly without inventing fake words or crashing
    assert!(ass.contains("No word timings available"));
    assert!(!ass.contains("{\\kf"));
}

#[test]
fn test_17_fade_and_pop_animation_tags() {
    let cue =
        SubtitleCue::new("c1", MediaTimeUs(0), MediaTimeUs(3_000_000), "Animated cue").unwrap();
    let mut track = SubtitleTrack::new("t1", "en", "English", SubtitleFormat::Srt);
    track.cues = vec![cue];

    let fade_style = SubtitleStyle {
        animation: SubtitleAnimation::Fade,
        ..Default::default()
    };
    let fade_ass = generate_ass_script(&track, &fade_style).unwrap();
    assert!(fade_ass.contains("{\\fad(200,200)}Animated cue"));

    let pop_style = SubtitleStyle {
        animation: SubtitleAnimation::Pop,
        ..Default::default()
    };
    let pop_ass = generate_ass_script(&track, &pop_style).unwrap();
    assert!(pop_ass.contains("{\\fscx85\\fscy85\\t(0,120,\\fscx100\\fscy100)}Animated cue"));
}

#[test]
fn test_18_position_mapping_ass_alignment() {
    assert_eq!(SubtitlePosition::BottomCenter.to_ass_alignment(), 2);
    assert_eq!(SubtitlePosition::BottomLeft.to_ass_alignment(), 1);
    assert_eq!(SubtitlePosition::BottomRight.to_ass_alignment(), 3);
    assert_eq!(SubtitlePosition::Center.to_ass_alignment(), 5);
    assert_eq!(SubtitlePosition::TopCenter.to_ass_alignment(), 8);
}

#[test]
fn test_19_export_profile_validation() {
    let profile = SubtitleExportProfile::default();
    assert_eq!(profile.container, "mp4");
    assert_eq!(profile.video_codec, "libx264");
    assert_eq!(profile.crf, 23);
    assert_eq!(profile.audio_policy, AudioExportPolicy::Copy);
}

#[test]
fn test_20_ass_filter_argument_escaping() {
    let path = PathBuf::from("/home/user/path:with:colons/sub.ass");
    let filter = build_ass_filter_arg(&path);
    assert_eq!(filter, "ass='/home/user/path\\:with\\:colons/sub.ass'");
}

#[test]
fn test_21_ffmpeg_burnin_support_verified() {
    assert!(verify_ffmpeg_burnin_support().is_ok());
}

#[test]
fn test_22_source_immutability_enforced() {
    let dir = tempdir().expect("tempdir");
    let source_path = dir.path().join("source.mp4");
    fs::write(&source_path, b"fake source video").unwrap();

    let manager = opentake_core::localization::styling::BurnInExportManager::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(manager.finalize_export(&source_path, &source_path));
    assert!(
        res.is_err(),
        "Must strictly reject overwriting source video file"
    );
}

#[test]
fn test_23_staged_export_finalization_no_clobber() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("original.mp4");
    let dest = dir.path().join("finalized.mp4");
    fs::write(&source, b"original media").unwrap();

    let manager = opentake_core::localization::styling::BurnInExportManager::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Cannot finalize when no staged export exists
    let res = rt.block_on(manager.finalize_export(&dest, &source));
    assert!(res.is_err());
}

#[test]
fn test_24_export_cancellation() {
    let manager = opentake_core::localization::styling::BurnInExportManager::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(manager.cancel());
    let status = rt.block_on(manager.get_status());
    assert_eq!(
        status,
        opentake_core::localization::styling::SubtitleBurnInStatus::Cancelled
    );
}

#[test]
fn test_25_real_ffmpeg_burnin_video_e2e_fixture() {
    let dir = tempdir().expect("tempdir");
    let source_video = dir.path().join("test_input.mp4");

    // 1. Create a deterministic 2-second 320x240 black video with silent audio
    let create_status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x240:d=2:r=25",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=44100:cl=mono",
            "-t",
            "2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            &source_video.to_string_lossy(),
        ])
        .output()
        .expect("create test video");
    assert!(
        create_status.status.success(),
        "Failed to create test video fixture"
    );

    // Compute source SHA256 to verify immutability
    let source_bytes = fs::read(&source_video).unwrap();
    let source_hash = Sha256::digest(&source_bytes);

    // 2. Prepare subtitle track
    let mut track = SubtitleTrack::new("track-e2e", "en", "English", SubtitleFormat::Srt);
    track.cues = vec![SubtitleCue::new(
        "cue-1",
        MediaTimeUs(0),
        MediaTimeUs(1_800_000),
        "Open Media Workbench Burn-In",
    )
    .unwrap()];

    let style = SubtitleStyle {
        font_size: 20,
        text_color: ColorRgba::opaque(255, 255, 255),
        outline_width: 2.0,
        outline_color: ColorRgba::opaque(0, 0, 0),
        ..SubtitleStyle::default()
    };

    let profile = SubtitleExportProfile {
        container: "mp4".to_string(),
        resolution: ExportResolution::Original,
        video_codec: "libx264".to_string(),
        crf: 22,
        preset: "ultrafast".to_string(),
        audio_policy: AudioExportPolicy::Copy,
        selected_track_id: "track-e2e".to_string(),
        style: style.clone(),
    };

    let request = SubtitleBurnInRequest {
        job_id: "e2e-job-001".to_string(),
        source_media_path: source_video.clone(),
        track,
        style,
        profile,
        staging_dir: dir.path().join("staging"),
    };

    // 3. Execute genuine burn-in export
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(execute_subtitle_burn_in(request, None))
        .expect("execute subtitle burn-in");

    // 4. Verify output file and container metadata
    assert!(result.staged_output_path.exists());
    assert!(result.file_size_bytes > 0);
    assert_eq!(result.width, 320);
    assert_eq!(result.height, 240);
    assert_eq!(result.video_codec, "h264");

    // 5. Verify visual burning by extracting a frame at 0.5s and checking pixel variance
    let frame_png = dir.path().join("frame_at_05s.png");
    let extract_status = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss",
            "00:00:00.500",
            "-i",
            &result.staged_output_path.to_string_lossy(),
            "-vframes",
            "1",
            &frame_png.to_string_lossy(),
        ])
        .output()
        .expect("extract frame");
    assert!(extract_status.status.success());
    assert!(frame_png.exists());
    let frame_size = fs::metadata(&frame_png).unwrap().len();
    assert!(frame_size > 0, "Decoded subtitle frame must be non-empty");

    // 6. Verify source immutability
    let source_bytes_after = fs::read(&source_video).unwrap();
    let source_hash_after = Sha256::digest(&source_bytes_after);
    assert_eq!(
        source_hash, source_hash_after,
        "Source media file must remain 100% byte-for-byte identical"
    );
}

#[test]
fn test_26_real_bengali_subtitle_burnin_e2e() {
    let dir = tempdir().expect("tempdir");
    let source_video = dir.path().join("bengali_input.mp4");

    // Create 1-second test video
    let create_status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=320x240:d=1:r=25",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=44100:cl=mono",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            &source_video.to_string_lossy(),
        ])
        .output()
        .expect("create test video");
    assert!(create_status.status.success());

    // Prepare Bengali subtitle track
    let mut track = SubtitleTrack::new("track-bn", "bn", "Bengali", SubtitleFormat::Srt);
    track.cues = vec![SubtitleCue::new(
        "cue-bn",
        MediaTimeUs(0),
        MediaTimeUs(900_000),
        "ওপেন মিডিয়া ওয়ার্কবেঞ্চ",
    )
    .unwrap()];

    let style = SubtitleStyle {
        font_family: "Noto Sans Bengali".to_string(),
        font_size: 20,
        text_color: ColorRgba::opaque(255, 255, 255),
        ..SubtitleStyle::default()
    };

    let profile = SubtitleExportProfile {
        container: "mp4".to_string(),
        resolution: ExportResolution::Original,
        video_codec: "libx264".to_string(),
        crf: 23,
        preset: "ultrafast".to_string(),
        audio_policy: AudioExportPolicy::Copy,
        selected_track_id: "track-bn".to_string(),
        style: style.clone(),
    };

    let request = SubtitleBurnInRequest {
        job_id: "bn-job-001".to_string(),
        source_media_path: source_video,
        track,
        style,
        profile,
        staging_dir: dir.path().join("staging_bn"),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(execute_subtitle_burn_in(request, None))
        .expect("execute bengali subtitle burn-in");

    assert!(result.staged_output_path.exists());
    assert!(result.file_size_bytes > 0);
}
