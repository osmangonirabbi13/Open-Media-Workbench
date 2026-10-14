use opentake_core::media::authorization::{MediaAuthorizationRegistry, SourceProvenance};
use opentake_core::repair::history::{RecoveryHistoryEntry, RecoveryHistoryStore};
use opentake_core::repair::recovery_review::{
    AssetMediaSummary, CheckItem, DurationComparison, RecoveryReview, TimelineEvidence,
    ValidationOutcome, ValidationSummary,
};
use opentake_core::repair::reporting::{
    generate_recovery_html_report, generate_recovery_json_report, html_escape,
    sanitize_privacy_string,
};
use opentake_core::repair::{
    RecoveryMethod, RecoveryReport, RecoveryStatus, RecoveryValidationResult, StreamReportInfo,
    ValidationLevel,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_recovery_review_from_remux_report() {
    let report = RecoveryReport {
        schema_version: 1,
        job_id: "remux-job-1".to_string(),
        source_asset_id: "source-1".to_string(),
        source_file_name: "corrupted.mp4".to_string(),
        recovery_method: RecoveryMethod::LosslessRemux,
        original_container: "QuickTime / MOV".to_string(),
        intended_output_container: "mp4".to_string(),
        actual_output_container: "mp4".to_string(),
        input_streams: vec![StreamReportInfo {
            index: 0,
            stream_type: "video".to_string(),
            codec_name: "h264".to_string(),
            duration_seconds: Some(10.0),
            resolution: Some("1920x1080".to_string()),
            sample_rate: None,
            channels: None,
        }],
        output_streams: vec![StreamReportInfo {
            index: 0,
            stream_type: "video".to_string(),
            codec_name: "h264".to_string(),
            duration_seconds: Some(10.0),
            resolution: Some("1920x1080".to_string()),
            sample_rate: None,
            channels: None,
        }],
        processing_status: RecoveryStatus::Completed,
        validation: RecoveryValidationResult {
            passed: true,
            is_playable: true,
            validation_level: ValidationLevel::SampledDecode,
            streams_present: vec!["video".to_string()],
            streams_expected: vec!["video".to_string()],
            stream_loss_detected: false,
            output_duration_seconds: Some(10.0),
            details: "Sampled validation passed".to_string(),
            warnings: vec![],
        },
        warnings: vec![],
        observed_ffmpeg_errors: vec![],
        output_file_size_bytes: Some(5_000_000),
        output_file_path: Some("/tmp/staging/job-1/output.mp4".to_string()),
        source_duration_seconds: Some(10.0),
        output_duration_seconds: Some(10.0),
        duration_difference_seconds: Some(0.0),
        started_at_epoch_ms: 1000,
        completed_at_epoch_ms: 2000,
        limitations: vec!["Remux container only".to_string()],
    };

    let review =
        RecoveryReview::from_recovery_report(&report, true, Some("candidate-asset-1".to_string()));

    assert_eq!(review.schema_version, 1);
    assert_eq!(review.recovery_job_id, "remux-job-1");
    assert_eq!(review.recovery_status, RecoveryStatus::Completed);
    assert_eq!(
        review.output_asset_id,
        Some("candidate-asset-1".to_string())
    );
    assert_eq!(
        review.validation_summary.overall_outcome,
        ValidationOutcome::Passed
    );
    assert_eq!(
        review.validation_summary.level,
        ValidationLevel::SampledDecode
    );
    assert!(
        !review
            .validation_summary
            .duration_comparison
            .is_source_duration_unknown
    );
    assert_eq!(
        review
            .timeline_evidence
            .as_ref()
            .unwrap()
            .recovered_duration_seconds,
        Some(10.0)
    );
}

#[test]
fn test_recovery_review_unknown_source_duration() {
    let report = RecoveryReport {
        schema_version: 1,
        job_id: "missing-moov-job".to_string(),
        source_asset_id: "source-broken".to_string(),
        source_file_name: "unfinalized.mp4".to_string(),
        recovery_method: RecoveryMethod::TimestampRebuild,
        original_container: "QuickTime / MOV".to_string(),
        intended_output_container: "mp4".to_string(),
        actual_output_container: "mp4".to_string(),
        input_streams: vec![],
        output_streams: vec![StreamReportInfo {
            index: 0,
            stream_type: "video".to_string(),
            codec_name: "h264".to_string(),
            duration_seconds: Some(25.4),
            resolution: Some("1920x1080".to_string()),
            sample_rate: None,
            channels: None,
        }],
        processing_status: RecoveryStatus::Completed,
        validation: RecoveryValidationResult {
            passed: true,
            is_playable: true,
            validation_level: ValidationLevel::SampledDecode,
            streams_present: vec!["video".to_string()],
            streams_expected: vec!["video".to_string()],
            stream_loss_detected: false,
            output_duration_seconds: Some(25.4),
            details: "Playable".to_string(),
            warnings: vec![],
        },
        warnings: vec![],
        observed_ffmpeg_errors: vec![],
        output_file_size_bytes: Some(12_000_000),
        output_file_path: Some("/tmp/staging/job-2/output.mp4".to_string()),
        source_duration_seconds: None, // Missing moov - duration unknown
        output_duration_seconds: Some(25.4),
        duration_difference_seconds: None,
        started_at_epoch_ms: 3000,
        completed_at_epoch_ms: 5000,
        limitations: vec!["Synthesized PTS/DTS timestamps".to_string()],
    };

    let review = RecoveryReview::from_recovery_report(&report, false, None);

    assert!(
        review
            .validation_summary
            .duration_comparison
            .is_source_duration_unknown
    );
    assert_eq!(
        review
            .validation_summary
            .duration_comparison
            .source_duration_seconds,
        None
    );
    assert_eq!(
        review
            .validation_summary
            .duration_comparison
            .recovered_duration_seconds,
        Some(25.4)
    );
    assert!(review
        .validation_summary
        .duration_comparison
        .explanation
        .contains("lacks valid header duration"));
}

#[test]
fn test_recovery_review_intentional_stream_omission() {
    let report = RecoveryReport {
        schema_version: 1,
        job_id: "video-salvage-job".to_string(),
        source_asset_id: "damaged-av".to_string(),
        source_file_name: "broken_audio.mp4".to_string(),
        recovery_method: RecoveryMethod::StreamSalvage,
        original_container: "QuickTime / MOV".to_string(),
        intended_output_container: "mp4".to_string(),
        actual_output_container: "mp4".to_string(),
        input_streams: vec![
            StreamReportInfo {
                index: 0,
                stream_type: "video".to_string(),
                codec_name: "h264".to_string(),
                duration_seconds: Some(12.0),
                resolution: Some("1920x1080".to_string()),
                sample_rate: None,
                channels: None,
            },
            StreamReportInfo {
                index: 1,
                stream_type: "audio".to_string(),
                codec_name: "aac".to_string(),
                duration_seconds: Some(12.0),
                resolution: None,
                sample_rate: Some(48000),
                channels: Some(2),
            },
        ],
        output_streams: vec![StreamReportInfo {
            index: 0,
            stream_type: "video".to_string(),
            codec_name: "h264".to_string(),
            duration_seconds: Some(12.0),
            resolution: Some("1920x1080".to_string()),
            sample_rate: None,
            channels: None,
        }],
        processing_status: RecoveryStatus::Completed,
        validation: RecoveryValidationResult {
            passed: true,
            is_playable: true,
            validation_level: ValidationLevel::SampledDecode,
            streams_present: vec!["video".to_string()],
            streams_expected: vec!["video".to_string()],
            stream_loss_detected: false,
            output_duration_seconds: Some(12.0),
            details: "Video salvaging complete".to_string(),
            warnings: vec![],
        },
        warnings: vec![],
        observed_ffmpeg_errors: vec![],
        output_file_size_bytes: Some(8_000_000),
        output_file_path: Some("/tmp/staging/job-3/output.mp4".to_string()),
        source_duration_seconds: Some(12.0),
        output_duration_seconds: Some(12.0),
        duration_difference_seconds: Some(0.0),
        started_at_epoch_ms: 1000,
        completed_at_epoch_ms: 2000,
        limitations: vec![
            "Audio track intentionally omitted by stream salvage configuration".to_string(),
        ],
    };

    let review = RecoveryReview::from_recovery_report(&report, false, None);

    // Audio checks should reflect intentional omission, NOT an unexpected error
    let audio_check = review
        .validation_summary
        .audio_checks
        .iter()
        .find(|c| c.code == "AUDIO_INTENTIONALLY_OMITTED");
    assert!(audio_check.is_some());
    assert_eq!(audio_check.unwrap().outcome, ValidationOutcome::Passed);
}

#[test]
fn test_recovery_history_persistence_and_bounding() {
    let dir = tempdir().unwrap();
    let history_file = dir.path().join("recovery_history.json");

    let mut store = RecoveryHistoryStore::load_from_file(&history_file);
    assert_eq!(store.entries.len(), 0);

    // Add 55 entries to test 50-entry capping
    for i in 0..55 {
        let entry = RecoveryHistoryEntry {
            job_id: format!("job-{}", i),
            source_asset_id: format!("asset-{}", i),
            source_file_name: format!("source_{}.mp4", i),
            recovery_method: "Lossless Container Remux".to_string(),
            attempt_date_time_epoch_ms: 1000 + i as u64,
            final_status: RecoveryStatus::Completed,
            validation_level: ValidationLevel::SampledDecode,
            validation_outcome: ValidationOutcome::Passed,
            output_available: true,
            output_file_name: Some(format!("recovered_{}.mp4", i)),
            report_exported: false,
            source_duration_seconds: Some(10.0),
            recovered_duration_seconds: Some(10.0),
            limitations: vec![],
        };
        store.add_entry(entry);
    }

    assert_eq!(store.entries.len(), 50);
    assert_eq!(store.entries[0].job_id, "job-54"); // Most recent first

    // Persist and reload from file to ensure atomic disk persistence
    store
        .save_to_file(&history_file)
        .expect("save_to_file must succeed");

    let reloaded = RecoveryHistoryStore::load_from_file(&history_file);
    assert_eq!(reloaded.entries.len(), 50);
    assert_eq!(reloaded.entries[0].job_id, "job-54");

    // Clear history
    let mut store_to_clear = reloaded;
    store_to_clear.clear();
    store_to_clear
        .save_to_file(&history_file)
        .expect("save_to_file must succeed");

    let reloaded_after_clear = RecoveryHistoryStore::load_from_file(&history_file);
    assert_eq!(reloaded_after_clear.entries.len(), 0);
}

#[test]
fn test_recovery_history_corrupt_file_resilience() {
    let dir = tempdir().unwrap();
    let history_file = dir.path().join("corrupt_history.json");

    // Write malformed JSON
    fs::write(&history_file, b"{ invalid json garbage ]").unwrap();

    // Should load gracefully as empty store rather than crashing
    let store = RecoveryHistoryStore::load_from_file(&history_file);
    assert_eq!(store.entries.len(), 0);
}

#[test]
fn test_html_and_json_report_generation_with_xss_and_path_privacy() {
    let review = RecoveryReview {
        schema_version: 1,
        review_id: "rev-xss-test".to_string(),
        source_asset_id: "source-xss".to_string(),
        recovery_job_id: "job-xss".to_string(),
        recovery_method: "<script>alert('xss')</script>".to_string(),
        recovery_status: RecoveryStatus::Completed,
        output_asset_id: None,
        output_candidate_id: None,
        source_summary: AssetMediaSummary {
            file_name: "<img src=x onerror=alert(1)>.mov".to_string(),
            file_size_bytes: Some(1024),
            container_format: "QuickTime / MOV".to_string(),
            duration_seconds: Some(5.0),
            streams: vec![],
            is_playable: false,
        },
        output_summary: Some(AssetMediaSummary {
            file_name: "clean_output.mp4".to_string(),
            file_size_bytes: Some(1024),
            container_format: "MP4".to_string(),
            duration_seconds: Some(5.0),
            streams: vec![],
            is_playable: true,
        }),
        validation_summary: ValidationSummary {
            level: ValidationLevel::SampledDecode,
            overall_outcome: ValidationOutcome::Warning,
            container_checks: vec![CheckItem {
                code: "CHECK_1".to_string(),
                outcome: ValidationOutcome::Passed,
                title: "Header Check".to_string(),
                message: "Valid <safe> container".to_string(),
            }],
            stream_checks: vec![],
            timestamp_checks: vec![],
            decode_checks: vec![],
            audio_checks: vec![],
            duration_comparison: DurationComparison {
                source_duration_seconds: Some(5.0),
                recovered_duration_seconds: Some(5.0),
                difference_seconds: Some(0.0),
                is_source_duration_unknown: false,
                explanation: "Match".to_string(),
            },
            inspection_coverage: "Sampled 5s decode".to_string(),
        },
        timeline_evidence: Some(TimelineEvidence {
            source_duration_seconds: Some(5.0),
            recovered_duration_seconds: Some(5.0),
            video_start_offset_seconds: Some(0.0),
            audio_start_offset_seconds: None,
            av_drift_seconds: None,
            known_decode_failures: vec!["/home/username/private/staged/error.log".to_string()],
            uninspected_regions_note: None,
        }),
        warnings: vec!["Warning: /home/username/secret/video.mp4 has artifacts".to_string()],
        limitations: vec!["Partial reconstruction & test".to_string()],
        created_at_epoch_ms: 1700000000000,
    };

    // Test HTML generation
    let html = generate_recovery_html_report(&review);

    // Verify XSS prevention
    assert!(!html.contains("<script>alert('xss')</script>"));
    assert!(html.contains("&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"));
    assert!(!html.contains("<img src=x onerror=alert(1)>"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));

    // Verify self-contained offline CSS & CSP
    assert!(html.contains("Content-Security-Policy"));
    assert!(html.contains("<style>"));
    assert!(html.contains("font-family:"));

    // Test JSON generation
    let json = generate_recovery_json_report(&review).expect("JSON serialization must succeed");
    assert!(json.contains("\"schemaVersion\": 1"));
    assert!(json.contains("\"reviewId\": \"rev-xss-test\""));

    // Test Privacy Sanitization
    let sensitive_path = if let Ok(home) = std::env::var("HOME") {
        format!("{}/secret_videos/clip.mp4", home)
    } else {
        "/home/testuser/secret_videos/clip.mp4".to_string()
    };
    let sanitized = sanitize_privacy_string(&sensitive_path);
    assert!(!sanitized.contains("/home/"));
    assert!(sanitized.starts_with("~/secret_videos"));
}

#[test]
fn test_html_escape_helper() {
    assert_eq!(
        html_escape("<b>\"test\" & 'foo'</b>"),
        "&lt;b&gt;&quot;test&quot; &amp; &#x27;foo&#x27;&lt;/b&gt;"
    );
}

#[tokio::test]
async fn test_staged_candidate_source_provenance() {
    let prov = SourceProvenance::StagedRecoveryCandidate;
    assert_eq!(format!("{:?}", prov), "StagedRecoveryCandidate");

    let registry = MediaAuthorizationRegistry::new();
    let dummy_path = std::path::PathBuf::from("/nonexistent/file.mp4");
    assert!(!registry.is_authorized(&dummy_path).await);
}
