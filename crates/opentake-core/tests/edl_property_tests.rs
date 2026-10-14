use proptest::prelude::*;

use opentake_core::editing::{
    cut_source_range, restore_source_range, EditDecisionList, KeepSegment, MediaTimeUs,
    SourceRange, SourceReference, TimelineIndex,
};

// ============================================================================
// Property-Based Testing for the 15 Core Invariants (Section 25)
// ============================================================================

const MAX_TEST_DURATION_US: i64 = 1_000_000_000; // 1000 seconds

prop_compose! {
    /// Generates a valid test source with duration up to 1000 seconds
    fn arb_source()(duration_us in 1_000_000i64..MAX_TEST_DURATION_US) -> SourceReference {
        SourceReference::new("prop_source", MediaTimeUs::from_micros(duration_us), None).unwrap()
    }
}

prop_compose! {
    /// Generates a valid source range within a given duration
    fn arb_source_range(max_us: i64)(
        a in 0i64..max_us - 1,
        b in 0i64..max_us - 1,
    ) -> SourceRange {
        let (start, end) = if a < b { (a, b + 1) } else { (b, a + 1) };
        SourceRange::from_micros(start, end).unwrap()
    }
}

proptest! {
    // ------------------------------------------------------------------------
    // Invariant 1, 2, 3, 4, 5: Canonical Normalization Invariants
    // ------------------------------------------------------------------------
    #[test]
    fn prop_normalization_invariants(
        source in arb_source(),
        raw_starts in prop::collection::vec(0i64..500_000_000, 0..30),
        raw_lengths in prop::collection::vec(1_000i64..50_000_000, 0..30),
    ) {
        let max_us = source.duration_us.as_micros();
        let mut raw_segments = Vec::new();

        let count = raw_starts.len().min(raw_lengths.len());
        for i in 0..count {
            let start = raw_starts[i] % max_us;
            let length = raw_lengths[i] % (max_us - start).max(1);
            if length > 0 {
                let end = start + length;
                if let Ok(range) = SourceRange::from_micros(start, end) {
                    raw_segments.push(KeepSegment::new(range));
                }
            }
        }

        let edl = EditDecisionList::new_normalized(source.clone(), raw_segments).unwrap();

        // Invariant 1: Every canonical interval is valid (start < end, non-negative)
        for seg in edl.segments() {
            prop_assert!(seg.range.start_us() >= MediaTimeUs::ZERO);
            prop_assert!(seg.range.start_us() < seg.range.end_us());
            prop_assert!(seg.range.end_us() <= source.duration_us);
        }

        // Invariant 2 & 3: Strictly ordered and non-overlapping
        for pair in edl.segments().windows(2) {
            prop_assert!(pair[0].range.end_us() < pair[1].range.start_us(),
                "Segments must be strictly ordered with a gap: {:?} vs {:?}", pair[0], pair[1]);
        }

        // Invariant 4: Adjacent intervals must have been merged
        for pair in edl.segments().windows(2) {
            prop_assert!(!pair[0].range.is_adjacent(&pair[1].range),
                "Adjacent intervals should have merged: {:?} vs {:?}", pair[0], pair[1]);
        }

        // Invariant 5: Normalization is idempotent: normalize(normalize(edl)) == normalize(edl)
        let normalized_again = EditDecisionList::new_normalized(source, edl.segments().to_vec()).unwrap();
        prop_assert_eq!(edl, normalized_again);
    }

    // ------------------------------------------------------------------------
    // Invariant 6, 7, 8, 9: Edited Duration and Bidirectional Time Mapping
    // ------------------------------------------------------------------------
    #[test]
    fn prop_mapping_and_duration_invariants(
        source in arb_source(),
        cut_start in 0i64..400_000_000,
        cut_len in 1_000i64..100_000_000,
    ) {
        let max_us = source.duration_us.as_micros();
        let c_start = cut_start % max_us;
        let c_len = cut_len % (max_us - c_start).max(1);
        if c_len == 0 {
            return Ok(());
        }
        let cut_range = SourceRange::from_micros(c_start, c_start + c_len).unwrap();

        let full = EditDecisionList::new_full_source(source.clone()).unwrap();
        let edl = cut_source_range(&full, cut_range).unwrap();

        // Invariant 6: Edited duration equals sum of keep durations
        let expected_sum: i64 = edl.segments().iter().map(|s| s.duration().as_micros()).sum();
        prop_assert_eq!(edl.edited_duration_us().unwrap().as_micros(), expected_sum);

        let index = TimelineIndex::build(&edl).unwrap();

        // Invariant 7: Retained source mapping symmetry: timeline_to_source(source_to_timeline(s)) == s
        for seg in edl.segments() {
            let midpoint = seg.range.start_us() + (seg.duration().as_micros() / 2).into();
            let tl_time = index.source_to_timeline(midpoint);
            prop_assert!(tl_time.is_some());
            let src_back = index.timeline_to_source(tl_time.unwrap());
            prop_assert_eq!(src_back, Some(midpoint));
        }

        // Invariant 8: Removed source timestamps have no forward mapping
        let cut_mid = cut_range.start_us() + (cut_range.duration().as_micros() / 2).into();
        prop_assert_eq!(index.source_to_timeline(cut_mid), None);

        // Invariant 9: Timeline mapping stays strictly inside source bounds
        if index.total_duration_us() > MediaTimeUs::ZERO {
            let t_probe = MediaTimeUs::from_micros(index.total_duration_us().as_micros() / 2);
            let s_probe = index.timeline_to_source(t_probe).unwrap();
            prop_assert!(s_probe >= MediaTimeUs::ZERO);
            prop_assert!(s_probe <= source.duration_us);
        }
    }

    // ------------------------------------------------------------------------
    // Invariant 10, 11, 12, 13: Cut and Restore Monotonicity & Idempotence
    // ------------------------------------------------------------------------
    #[test]
    fn prop_cut_and_restore_monotonicity(
        source in arb_source(),
        c1_start in 0i64..300_000_000,
        c1_len in 10_000i64..50_000_000,
        c2_start in 0i64..300_000_000,
        c2_len in 10_000i64..50_000_000,
    ) {
        let max_us = source.duration_us.as_micros();
        let s1 = c1_start % max_us;
        let l1 = c1_len % (max_us - s1).max(1);
        if l1 == 0 { return Ok(()); }
        let cut1 = SourceRange::from_micros(s1, s1 + l1).unwrap();

        let s2 = c2_start % max_us;
        let l2 = c2_len % (max_us - s2).max(1);
        if l2 == 0 { return Ok(()); }
        let cut2 = SourceRange::from_micros(s2, s2 + l2).unwrap();

        let full = EditDecisionList::new_full_source(source).unwrap();
        let dur_initial = full.edited_duration_us().unwrap();

        // Invariant 10: Cut operations cannot increase kept duration
        let edl_cut1 = cut_source_range(&full, cut1).unwrap();
        let dur_cut1 = edl_cut1.edited_duration_us().unwrap();
        prop_assert!(dur_cut1 <= dur_initial);

        // Invariant 12: Repeated identical cuts are idempotent
        let edl_cut1_repeat = cut_source_range(&edl_cut1, cut1).unwrap();
        prop_assert_eq!(edl_cut1.clone(), edl_cut1_repeat);

        // Invariant 13: Input EDL was not mutated
        prop_assert_eq!(full.edited_duration_us().unwrap(), dur_initial);

        // Invariant 11: Restore operations cannot decrease kept duration
        let edl_restored = restore_source_range(&edl_cut1, cut2).unwrap();
        let dur_restored = edl_restored.edited_duration_us().unwrap();
        prop_assert!(dur_restored >= dur_cut1);
    }
}
