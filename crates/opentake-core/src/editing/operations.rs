use super::edl::{EditDecisionList, KeepSegment};
use super::error::EditError;
use super::interval::SourceRange;
use super::mapping::TimelineIndex;

/// Pure domain transformation subtracting a source-time interval $[start\_us, end\_us)$
/// from an existing `EditDecisionList`.
///
/// # Invariants
/// - Does not modify original media.
/// - Does not mutate the input `edl`.
/// - Produces a normalized, canonical result.
/// - Repeated identical cut operations are idempotent.
pub fn cut_source_range(
    edl: &EditDecisionList,
    cut_range: SourceRange,
) -> Result<EditDecisionList, EditError> {
    if cut_range.end_us() > edl.source().duration_us {
        return Err(EditError::SourceOutOfBounds {
            start_us: cut_range.start_us().as_micros(),
            end_us: cut_range.end_us().as_micros(),
            source_duration_us: edl.source().duration_us.as_micros(),
        });
    }

    let mut new_segments = Vec::new();

    for seg in edl.segments() {
        if !seg.range.overlaps(&cut_range) {
            new_segments.push(seg.clone());
        } else {
            let remaining_ranges = seg.range.subtract(&cut_range);
            for r in remaining_ranges {
                new_segments.push(KeepSegment::new(r));
            }
        }
    }

    EditDecisionList::new_normalized(edl.source().clone(), new_segments)
}

/// Pure domain transformation restoring an excised source-time interval $[start\_us, end\_us)$
/// back into the kept segments of an `EditDecisionList`.
///
/// Merges overlapping and adjacent segments into the canonical representation.
pub fn restore_source_range(
    edl: &EditDecisionList,
    restore_range: SourceRange,
) -> Result<EditDecisionList, EditError> {
    if restore_range.end_us() > edl.source().duration_us {
        return Err(EditError::SourceOutOfBounds {
            start_us: restore_range.start_us().as_micros(),
            end_us: restore_range.end_us().as_micros(),
            source_duration_us: edl.source().duration_us.as_micros(),
        });
    }

    let mut segments = edl.segments().to_vec();
    segments.push(KeepSegment::new(restore_range));

    EditDecisionList::new_normalized(edl.source().clone(), segments)
}

/// Intersects the currently kept segments with a single bounded source interval,
/// discarding any content outside `keep_range`.
pub fn keep_only(
    edl: &EditDecisionList,
    keep_range: SourceRange,
) -> Result<EditDecisionList, EditError> {
    if keep_range.end_us() > edl.source().duration_us {
        return Err(EditError::SourceOutOfBounds {
            start_us: keep_range.start_us().as_micros(),
            end_us: keep_range.end_us().as_micros(),
            source_duration_us: edl.source().duration_us.as_micros(),
        });
    }

    let mut kept = Vec::new();
    for seg in edl.segments() {
        if let Some(inter) = seg.range.intersection(&keep_range) {
            kept.push(KeepSegment::new(inter));
        }
    }

    EditDecisionList::new_normalized(edl.source().clone(), kept)
}

/// Pure domain transformation applying a cut specified in **continuous edited timeline coordinates**
/// $[t_{\text{start}}, t_{\text{end}})$.
///
/// # Discontinuous Multi-Segment Slicing
/// A cut on the edited timeline may span multiple discontinuous source segments.
/// This function:
/// 1. Builds the `TimelineIndex`.
/// 2. Validates that the timeline cut lies within edited duration bounds.
/// 3. Intersects `timeline_cut` with each indexed segment $[t_k.\text{start}, t_k.\text{end})$.
/// 4. Maps each timeline intersection back to its corresponding source range.
/// 5. Subtracts all mapped source ranges from the EDL.
pub fn cut_timeline_range(
    edl: &EditDecisionList,
    timeline_cut: SourceRange,
) -> Result<EditDecisionList, EditError> {
    let index = TimelineIndex::build(edl)?;

    if timeline_cut.end_us() > index.total_duration_us() {
        return Err(EditError::TimelineOutOfBounds {
            start_us: timeline_cut.start_us().as_micros(),
            end_us: timeline_cut.end_us().as_micros(),
            timeline_duration_us: index.total_duration_us().as_micros(),
        });
    }

    let mut current_edl = edl.clone();

    for indexed in index.segments() {
        if let Some(tl_intersection) = indexed.timeline_range.intersection(&timeline_cut) {
            // Map timeline intersection back to source coordinates:
            // source_offset = tl_offset
            let offset_start = tl_intersection.start_us() - indexed.timeline_range.start_us();
            let offset_end = tl_intersection.end_us() - indexed.timeline_range.start_us();

            let src_cut_start = indexed.source_range.start_us() + offset_start;
            let src_cut_end = indexed.source_range.start_us() + offset_end;

            let src_cut_range = SourceRange::new(src_cut_start, src_cut_end)?;
            current_edl = cut_source_range(&current_edl, src_cut_range)?;
        }
    }

    Ok(current_edl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::source::SourceReference;
    use crate::editing::time::MediaTimeUs;

    fn test_source() -> SourceReference {
        SourceReference::new("op_test", MediaTimeUs::from_micros(60_000_000), None).unwrap()
    }

    #[test]
    fn test_cut_and_restore_cycle() {
        let src = test_source();
        let full = EditDecisionList::new_full_source(src).unwrap();

        // 1. Cut [10s, 20s)
        let cut1 = SourceRange::from_micros(10_000_000, 20_000_000).unwrap();
        let edl_cut1 = cut_source_range(&full, cut1).unwrap();
        assert_eq!(edl_cut1.segment_count(), 2);
        assert_eq!(
            edl_cut1.segments()[0].range,
            SourceRange::from_micros(0, 10_000_000).unwrap()
        );
        assert_eq!(
            edl_cut1.segments()[1].range,
            SourceRange::from_micros(20_000_000, 60_000_000).unwrap()
        );

        // 2. Cut [35s, 40s)
        let cut2 = SourceRange::from_micros(35_000_000, 40_000_000).unwrap();
        let edl_cut2 = cut_source_range(&edl_cut1, cut2).unwrap();
        assert_eq!(edl_cut2.segment_count(), 3);
        assert_eq!(
            edl_cut2.edited_duration_us().unwrap(),
            MediaTimeUs::from_micros(45_000_000)
        );

        // 3. Repeated cut is idempotent
        let edl_repeat = cut_source_range(&edl_cut2, cut2).unwrap();
        assert_eq!(edl_cut2, edl_repeat);

        // 4. Restore [10s, 20s) restores first cut and merges with adjacent [0, 10s) and [20s, 35s) -> [0, 35s)
        let edl_restored = restore_source_range(&edl_cut2, cut1).unwrap();
        assert_eq!(edl_restored.segment_count(), 2);
        assert_eq!(
            edl_restored.segments()[0].range,
            SourceRange::from_micros(0, 35_000_000).unwrap()
        );
        assert_eq!(
            edl_restored.segments()[1].range,
            SourceRange::from_micros(40_000_000, 60_000_000).unwrap()
        );
    }

    #[test]
    fn test_cut_timeline_range_spanning_segments() {
        // Initial EDL: KEEP [0, 10s), KEEP [20s, 35s) -> Edited timeline [0, 25s)
        let src = test_source();
        let raw = vec![
            KeepSegment::new(SourceRange::from_micros(0, 10_000_000).unwrap()),
            KeepSegment::new(SourceRange::from_micros(20_000_000, 35_000_000).unwrap()),
        ];
        let edl = EditDecisionList::new_normalized(src, raw).unwrap();

        // Cut timeline [8s, 13s) -> 5s cut
        // Spans:
        // - Segment 1 [8s, 10s) timeline -> removes Source [8s, 10s)
        // - Segment 2 [10s, 13s) timeline -> removes Source [20s, 23s)
        // Resulting EDL: KEEP [0, 8s), KEEP [23s, 35s)
        let tl_cut = SourceRange::from_micros(8_000_000, 13_000_000).unwrap();
        let result = cut_timeline_range(&edl, tl_cut).unwrap();

        assert_eq!(result.segment_count(), 2);
        assert_eq!(
            result.segments()[0].range,
            SourceRange::from_micros(0, 8_000_000).unwrap()
        );
        assert_eq!(
            result.segments()[1].range,
            SourceRange::from_micros(23_000_000, 35_000_000).unwrap()
        );
        assert_eq!(
            result.edited_duration_us().unwrap(),
            MediaTimeUs::from_micros(20_000_000)
        );
    }
}
