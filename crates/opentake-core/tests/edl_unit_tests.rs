use opentake_core::editing::{
    cut_source_range, cut_timeline_range, keep_only, restore_source_range, EditDecisionList,
    EditError, KeepSegment, MediaTimeUs, RoundMode, SourceRange, SourceReference, TimelineIndex,
};
use opentake_core::media::metadata::Rational;

fn make_test_source(duration_secs: i64) -> SourceReference {
    SourceReference::new(
        "unit_test_media_asset",
        MediaTimeUs::from_micros(duration_secs * 1_000_000),
        Some("test_fingerprint_sha256".to_string()),
    )
    .unwrap()
}

// ============================================================================
// 1. Interval Unit Tests (Section 26)
// ============================================================================

#[test]
fn test_interval_valid_creation() {
    let range = SourceRange::from_micros(0, 10_000_000).unwrap();
    assert_eq!(range.start_us(), MediaTimeUs(0));
    assert_eq!(range.end_us(), MediaTimeUs(10_000_000));
    assert_eq!(range.duration(), MediaTimeUs(10_000_000));
}

#[test]
fn test_interval_negative_start_rejection() {
    let res = SourceRange::from_micros(-5, 100);
    assert_eq!(res, Err(EditError::NegativeTime { time_us: -5 }));
}

#[test]
fn test_interval_zero_length_rejection() {
    let res = SourceRange::from_micros(5_000, 5_000);
    assert_eq!(res, Err(EditError::ZeroLengthRange { time_us: 5_000 }));
}

#[test]
fn test_interval_reversed_bounds_rejection() {
    let res = SourceRange::from_micros(20_000, 10_000);
    assert_eq!(
        res,
        Err(EditError::InvalidInterval {
            start_us: 20_000,
            end_us: 10_000
        })
    );
}

#[test]
fn test_interval_adjacent_and_overlapping() {
    let r1 = SourceRange::from_micros(0, 10_000_000).unwrap();
    let r2 = SourceRange::from_micros(10_000_000, 20_000_000).unwrap();
    let r3 = SourceRange::from_micros(5_000_000, 15_000_000).unwrap();
    let r4 = SourceRange::from_micros(25_000_000, 30_000_000).unwrap();

    // Adjacent
    assert!(r1.is_adjacent(&r2));
    assert!(r2.is_adjacent(&r1));
    assert!(!r1.overlaps(&r2)); // Half-open intervals do not overlap at boundary!

    // Overlapping
    assert!(r1.overlaps(&r3));
    assert!(r2.overlaps(&r3));

    // Disjoint
    assert!(!r1.overlaps(&r4));
    assert!(!r1.is_adjacent(&r4));
}

// ============================================================================
// 2. Normalization & Canonical Invariant Tests (Section 9 & 26)
// ============================================================================

#[test]
fn test_normalization_merges_touching_and_overlapping() {
    let src = make_test_source(60);

    // Provide unsorted, overlapping, and duplicate intervals:
    // [20, 30), [0, 10), [10, 20), [5, 15)
    let raw = vec![
        KeepSegment::new(SourceRange::from_micros(20_000_000, 30_000_000).unwrap()),
        KeepSegment::new(SourceRange::from_micros(0, 10_000_000).unwrap()),
        KeepSegment::new(SourceRange::from_micros(10_000_000, 20_000_000).unwrap()),
        KeepSegment::new(SourceRange::from_micros(5_000_000, 15_000_000).unwrap()),
    ];

    let edl = EditDecisionList::new_normalized(src, raw).unwrap();
    assert_eq!(edl.segment_count(), 1);
    assert_eq!(
        edl.segments()[0].range,
        SourceRange::from_micros(0, 30_000_000).unwrap()
    );
    assert_eq!(
        edl.edited_duration_us().unwrap(),
        MediaTimeUs::from_micros(30_000_000)
    );
}

#[test]
fn test_normalization_exceeding_source_duration_fails() {
    let src = make_test_source(60); // 60s
    let raw = vec![KeepSegment::new(
        SourceRange::from_micros(55_000_000, 60_000_001).unwrap(),
    )];

    let err = EditDecisionList::new_normalized(src, raw).unwrap_err();
    match err {
        EditError::SourceOutOfBounds {
            start_us,
            end_us,
            source_duration_us,
        } => {
            assert_eq!(start_us, 55_000_000);
            assert_eq!(end_us, 60_000_001);
            assert_eq!(source_duration_us, 60_000_000);
        }
        other => panic!("Expected SourceOutOfBounds, got {:?}", other),
    }
}

// ============================================================================
// 3. Cut & Restore Operation Tests (Section 10, 11, 27)
// ============================================================================

#[test]
fn test_cut_scenarios() {
    let src = make_test_source(60);
    let full = EditDecisionList::new_full_source(src).unwrap();

    // A. Single cut in middle [10, 20)
    let cut1 = SourceRange::from_micros(10_000_000, 20_000_000).unwrap();
    let edl1 = cut_source_range(&full, cut1).unwrap();
    assert_eq!(edl1.segment_count(), 2);
    assert_eq!(
        edl1.segments()[0].range,
        SourceRange::from_micros(0, 10_000_000).unwrap()
    );
    assert_eq!(
        edl1.segments()[1].range,
        SourceRange::from_micros(20_000_000, 60_000_000).unwrap()
    );

    // B. Two nonoverlapping cuts: cut [35, 40)
    let cut2 = SourceRange::from_micros(35_000_000, 40_000_000).unwrap();
    let edl2 = cut_source_range(&edl1, cut2).unwrap();
    assert_eq!(edl2.segment_count(), 3);
    assert_eq!(
        edl2.segments()[0].range,
        SourceRange::from_micros(0, 10_000_000).unwrap()
    );
    assert_eq!(
        edl2.segments()[1].range,
        SourceRange::from_micros(20_000_000, 35_000_000).unwrap()
    );
    assert_eq!(
        edl2.segments()[2].range,
        SourceRange::from_micros(40_000_000, 60_000_000).unwrap()
    );

    // C. Overlapping cut: cut [8, 25)
    let cut_overlap = SourceRange::from_micros(8_000_000, 25_000_000).unwrap();
    let edl_overlap = cut_source_range(&edl2, cut_overlap).unwrap();
    assert_eq!(edl_overlap.segment_count(), 3);
    assert_eq!(
        edl_overlap.segments()[0].range,
        SourceRange::from_micros(0, 8_000_000).unwrap()
    );
    assert_eq!(
        edl_overlap.segments()[1].range,
        SourceRange::from_micros(25_000_000, 35_000_000).unwrap()
    );
    assert_eq!(
        edl_overlap.segments()[2].range,
        SourceRange::from_micros(40_000_000, 60_000_000).unwrap()
    );

    // D. Repeated cut is idempotent
    let edl_repeat = cut_source_range(&edl2, cut2).unwrap();
    assert_eq!(edl2, edl_repeat);

    // E. Cut at source start: cut [0, 5)
    let cut_start = SourceRange::from_micros(0, 5_000_000).unwrap();
    let edl_start = cut_source_range(&edl2, cut_start).unwrap();
    assert_eq!(
        edl_start.segments()[0].range,
        SourceRange::from_micros(5_000_000, 10_000_000).unwrap()
    );

    // F. Cut at source end: cut [55, 60)
    let cut_end = SourceRange::from_micros(55_000_000, 60_000_000).unwrap();
    let edl_end = cut_source_range(&edl2, cut_end).unwrap();
    assert_eq!(
        edl_end.segments().last().unwrap().range,
        SourceRange::from_micros(40_000_000, 55_000_000).unwrap()
    );

    // I. Cut entire source: cut [0, 60) -> produces empty EDL
    let cut_full = SourceRange::from_micros(0, 60_000_000).unwrap();
    let edl_empty = cut_source_range(&edl2, cut_full).unwrap();
    assert!(edl_empty.is_empty());
    assert_eq!(edl_empty.edited_duration_us().unwrap(), MediaTimeUs::ZERO);
}

#[test]
fn test_restore_scenarios() {
    let src = make_test_source(60);
    let full = EditDecisionList::new_full_source(src).unwrap();

    // Create 3 segments: [0, 10), [20, 35), [40, 60)
    let cut1 = SourceRange::from_micros(10_000_000, 20_000_000).unwrap();
    let cut2 = SourceRange::from_micros(35_000_000, 40_000_000).unwrap();
    let edl = cut_source_range(&cut_source_range(&full, cut1).unwrap(), cut2).unwrap();

    // J. Restore one cut: restore [10, 20) -> should merge [0, 10) + [10, 20) + [20, 35) into [0, 35)
    let restored1 = restore_source_range(&edl, cut1).unwrap();
    assert_eq!(restored1.segment_count(), 2);
    assert_eq!(
        restored1.segments()[0].range,
        SourceRange::from_micros(0, 35_000_000).unwrap()
    );
    assert_eq!(
        restored1.segments()[1].range,
        SourceRange::from_micros(40_000_000, 60_000_000).unwrap()
    );

    // K. Restore overlapping range: restore [8, 22) into edl
    let restored_overlap = restore_source_range(
        &edl,
        SourceRange::from_micros(8_000_000, 22_000_000).unwrap(),
    )
    .unwrap();
    assert_eq!(restored_overlap.segment_count(), 2);
    assert_eq!(
        restored_overlap.segments()[0].range,
        SourceRange::from_micros(0, 35_000_000).unwrap()
    );

    // L. Restore adjacent range: restore [10, 15) -> creates [0, 15), leaving [15, 20) cut
    let restored_adj = restore_source_range(
        &edl,
        SourceRange::from_micros(10_000_000, 15_000_000).unwrap(),
    )
    .unwrap();
    assert_eq!(restored_adj.segment_count(), 3);
    assert_eq!(
        restored_adj.segments()[0].range,
        SourceRange::from_micros(0, 15_000_000).unwrap()
    );
    assert_eq!(
        restored_adj.segments()[1].range,
        SourceRange::from_micros(20_000_000, 35_000_000).unwrap()
    );
    assert_eq!(
        restored_adj.segments()[2].range,
        SourceRange::from_micros(40_000_000, 60_000_000).unwrap()
    );

    // M. Restore full source -> restores to full EDL
    let restored_full =
        restore_source_range(&edl, SourceRange::from_micros(0, 60_000_000).unwrap()).unwrap();
    assert!(restored_full.is_full_source());
    assert_eq!(
        restored_full.edited_duration_us().unwrap(),
        MediaTimeUs::from_micros(60_000_000)
    );
}

#[test]
fn test_keep_only_operation() {
    let src = make_test_source(60);
    let full = EditDecisionList::new_full_source(src).unwrap();

    let cut1 = SourceRange::from_micros(10_000_000, 20_000_000).unwrap();
    let cut2 = SourceRange::from_micros(35_000_000, 40_000_000).unwrap();
    let edl = cut_source_range(&cut_source_range(&full, cut1).unwrap(), cut2).unwrap();
    // Kept: [0, 10), [20, 35), [40, 60)

    // keep_only [5, 45) -> should yield:
    // [5, 10), [20, 35), [40, 45)
    let filtered = keep_only(
        &edl,
        SourceRange::from_micros(5_000_000, 45_000_000).unwrap(),
    )
    .unwrap();
    assert_eq!(filtered.segment_count(), 3);
    assert_eq!(
        filtered.segments()[0].range,
        SourceRange::from_micros(5_000_000, 10_000_000).unwrap()
    );
    assert_eq!(
        filtered.segments()[1].range,
        SourceRange::from_micros(20_000_000, 35_000_000).unwrap()
    );
    assert_eq!(
        filtered.segments()[2].range,
        SourceRange::from_micros(40_000_000, 45_000_000).unwrap()
    );
}

#[test]
fn test_cut_timeline_range_multi_segment() {
    let src = make_test_source(60);
    let full = EditDecisionList::new_full_source(src).unwrap();

    // Kept: [0, 10s), [20s, 35s), [40s, 60s) -> total edited duration = 45s
    let cut1 = SourceRange::from_micros(10_000_000, 20_000_000).unwrap();
    let cut2 = SourceRange::from_micros(35_000_000, 40_000_000).unwrap();
    let edl = cut_source_range(&cut_source_range(&full, cut1).unwrap(), cut2).unwrap();

    // On edited timeline:
    // Seg 0: timeline [0, 10s) -> source [0, 10s)
    // Seg 1: timeline [10s, 25s) -> source [20s, 35s)
    // Seg 2: timeline [25s, 45s) -> source [40s, 60s)

    // Cut edited timeline from 8s to 27s (19s cut across all 3 segments):
    // - In Seg 0: timeline [8s, 10s) -> cuts source [8s, 10s)
    // - In Seg 1: timeline [10s, 25s) -> cuts entire source [20s, 35s)
    // - In Seg 2: timeline [25s, 27s) -> cuts source [40s, 42s)
    // Expected retained segments:
    // [0, 8s) and [42s, 60s)
    let tl_cut = SourceRange::from_micros(8_000_000, 27_000_000).unwrap();
    let result = cut_timeline_range(&edl, tl_cut).unwrap();

    assert_eq!(result.segment_count(), 2);
    assert_eq!(
        result.segments()[0].range,
        SourceRange::from_micros(0, 8_000_000).unwrap()
    );
    assert_eq!(
        result.segments()[1].range,
        SourceRange::from_micros(42_000_000, 60_000_000).unwrap()
    );
    assert_eq!(
        result.edited_duration_us().unwrap(),
        MediaTimeUs::from_micros(26_000_000)
    );
}

// ============================================================================
// 4. Timebase Conversion Tests (Section 28)
// ============================================================================

#[test]
fn test_standard_frame_rates() {
    // 24 fps
    let fps24 = Rational::new(24, 1).unwrap();
    let t_f24 = MediaTimeUs::from_frame(48, fps24, RoundMode::Nearest).unwrap();
    assert_eq!(t_f24, MediaTimeUs::from_micros(2_000_000));
    assert_eq!(t_f24.to_frame(fps24, RoundMode::Nearest).unwrap(), 48);

    // 25 fps (PAL)
    let fps25 = Rational::new(25, 1).unwrap();
    let t_f25 = MediaTimeUs::from_frame(25, fps25, RoundMode::Nearest).unwrap();
    assert_eq!(t_f25, MediaTimeUs::SECOND);
    assert_eq!(t_f25.to_frame(fps25, RoundMode::Nearest).unwrap(), 25);

    // 59.94 fps (NTSC 60000/1001)
    let fps59 = Rational::new(60000, 1001).unwrap();
    let t_f59 = MediaTimeUs::from_frame(60000, fps59, RoundMode::Nearest).unwrap();
    assert_eq!(t_f59.as_micros(), 1_001_000_000); // exactly 1001 seconds
    assert_eq!(t_f59.to_frame(fps59, RoundMode::Nearest).unwrap(), 60000);
}

#[test]
fn test_ffmpeg_stream_timebase_pts_conversions() {
    // Standard MP4 video stream timebase: 1/90000
    let tb90k = Rational::new(1, 90000).unwrap();

    // PTS 90,000 should equal 1 second = 1,000,000 us
    let t = MediaTimeUs::from_pts(90000, tb90k, RoundMode::Nearest).unwrap();
    assert_eq!(t, MediaTimeUs::SECOND);

    // Reverse: 1,000,000 us -> PTS 90,000
    let pts = t.to_pts(tb90k, RoundMode::Nearest).unwrap();
    assert_eq!(pts, 90000);
}

// ============================================================================
// 5. Mapping Tests with Canonical Example (Section 15, 16, 29)
// ============================================================================

#[test]
fn test_canonical_edl_bidirectional_mapping_and_boundaries() {
    let src = make_test_source(60);
    let full = EditDecisionList::new_full_source(src).unwrap();

    let cut1 = SourceRange::from_micros(10_000_000, 20_000_000).unwrap();
    let cut2 = SourceRange::from_micros(35_000_000, 40_000_000).unwrap();
    let edl = cut_source_range(&cut_source_range(&full, cut1).unwrap(), cut2).unwrap();

    let index = TimelineIndex::build(&edl).unwrap();

    // Source to timeline verification
    assert_eq!(
        index.source_to_timeline(MediaTimeUs::from_micros(5_000_000)),
        Some(MediaTimeUs::from_micros(5_000_000))
    );
    assert_eq!(
        index.source_to_timeline(MediaTimeUs::from_micros(22_000_000)),
        Some(MediaTimeUs::from_micros(12_000_000))
    );
    assert_eq!(
        index.source_to_timeline(MediaTimeUs::from_micros(45_000_000)),
        Some(MediaTimeUs::from_micros(30_000_000))
    );

    // Cut regions return None
    assert_eq!(
        index.source_to_timeline(MediaTimeUs::from_micros(15_000_000)),
        None
    );
    assert_eq!(
        index.source_to_timeline(MediaTimeUs::from_micros(37_000_000)),
        None
    );

    // Timeline to source verification
    assert_eq!(
        index.timeline_to_source(MediaTimeUs::from_micros(5_000_000)),
        Some(MediaTimeUs::from_micros(5_000_000))
    );
    assert_eq!(
        index.timeline_to_source(MediaTimeUs::from_micros(12_000_000)),
        Some(MediaTimeUs::from_micros(22_000_000))
    );
    assert_eq!(
        index.timeline_to_source(MediaTimeUs::from_micros(30_000_000)),
        Some(MediaTimeUs::from_micros(45_000_000))
    );

    // Boundary concatenation points:
    // Timeline 0s -> Source 0s
    assert_eq!(
        index.timeline_to_source(MediaTimeUs::ZERO),
        Some(MediaTimeUs::ZERO)
    );
    // Timeline 10s -> Source 20s (start of second kept segment [20s, 35s))
    assert_eq!(
        index.timeline_to_source(MediaTimeUs::from_micros(10_000_000)),
        Some(MediaTimeUs::from_micros(20_000_000))
    );
    // Timeline 25s -> Source 40s (start of third kept segment [40s, 60s))
    assert_eq!(
        index.timeline_to_source(MediaTimeUs::from_micros(25_000_000)),
        Some(MediaTimeUs::from_micros(40_000_000))
    );

    // At exact final duration endpoint (45s):
    assert_eq!(
        index.timeline_to_source(MediaTimeUs::from_micros(45_000_000)),
        None
    );
    assert_eq!(
        index.timeline_to_source_endpoint(MediaTimeUs::from_micros(45_000_000)),
        Some(MediaTimeUs::from_micros(60_000_000))
    );
}
