use uuid::Uuid;

use super::edl::EditDecisionList;
use super::error::EditError;
use super::interval::SourceRange;
use super::time::MediaTimeUs;

/// A dual-coordinate indexed segment linking source time $[s_{\text{start}}, s_{\text{end}})$
/// to continuous output timeline position $[t_{\text{start}}, t_{\text{end}})$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSegment {
    pub segment_id: Uuid,
    pub source_range: SourceRange,
    pub timeline_range: SourceRange,
}

/// An immutable, binary-searchable timeline index derived deterministically from an `EditDecisionList`.
///
/// Enables $O(\log N)$ bidirectional time mapping between raw source timestamps and continuous
/// edited timeline playback positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineIndex {
    segments: Vec<IndexedSegment>,
    total_duration_us: MediaTimeUs,
}

impl TimelineIndex {
    /// Builds an index from a canonical `EditDecisionList`.
    ///
    /// Computes prefix sums of kept durations with checked arithmetic.
    pub fn build(edl: &EditDecisionList) -> Result<Self, EditError> {
        let mut segments = Vec::with_capacity(edl.segment_count());
        let mut timeline_cursor = MediaTimeUs::ZERO;

        for seg in edl.segments() {
            let seg_duration = seg.duration();
            let next_cursor = timeline_cursor.checked_add(seg_duration)?;

            let timeline_range = SourceRange::new(timeline_cursor, next_cursor)?;

            segments.push(IndexedSegment {
                segment_id: seg.id,
                source_range: seg.range,
                timeline_range,
            });

            timeline_cursor = next_cursor;
        }

        Ok(Self {
            segments,
            total_duration_us: timeline_cursor,
        })
    }

    /// Returns the total continuous output duration of the edited timeline.
    #[inline]
    pub const fn total_duration_us(&self) -> MediaTimeUs {
        self.total_duration_us
    }

    /// Returns the indexed segments.
    #[inline]
    pub fn segments(&self) -> &[IndexedSegment] {
        &self.segments
    }

    /// Maps a raw source timestamp to the corresponding edited timeline position.
    ///
    /// # Return Semantics
    /// - Returns `Some(timeline_time_us)` if `source_time_us` falls within a kept segment.
    /// - Returns `None` if `source_time_us` falls inside a cut/excised gap or outside media bounds.
    ///
    /// # Complexity
    /// $O(\log N)$ binary search over source start boundaries.
    pub fn source_to_timeline(&self, source_time: MediaTimeUs) -> Option<MediaTimeUs> {
        if self.segments.is_empty() {
            return None;
        }

        // Binary search for the segment whose source range could contain source_time.
        let idx = match self
            .segments
            .binary_search_by_key(&source_time, |s| s.source_range.start_us())
        {
            Ok(exact) => exact,
            Err(0) => return None, // Precedes first kept segment
            Err(pos) => pos - 1,   // Potential containing segment
        };

        let seg = &self.segments[idx];
        if seg.source_range.contains_time(source_time) {
            let offset = source_time - seg.source_range.start_us();
            Some(seg.timeline_range.start_us() + offset)
        } else {
            None
        }
    }

    /// Maps a continuous edited timeline playback position to the corresponding raw source timestamp.
    ///
    /// # Half-Open Playback Semantics
    /// For playback positions $0 \le t < \text{total\_duration}$, uses half-open interval $[start, end)$.
    /// At concatenation boundaries where segment $k$ ends and segment $k+1$ begins,
    /// timeline time $t = \text{boundary}$ unambiguously selects segment $k+1$.
    ///
    /// # Complexity
    /// $O(\log N)$ binary search over timeline start boundaries.
    pub fn timeline_to_source(&self, timeline_time: MediaTimeUs) -> Option<MediaTimeUs> {
        if self.segments.is_empty()
            || timeline_time < MediaTimeUs::ZERO
            || timeline_time >= self.total_duration_us
        {
            return None;
        }

        let idx = match self
            .segments
            .binary_search_by_key(&timeline_time, |s| s.timeline_range.start_us())
        {
            Ok(exact) => exact,
            Err(0) => return None,
            Err(pos) => pos - 1,
        };

        let seg = &self.segments[idx];
        if seg.timeline_range.contains_time(timeline_time) {
            let offset = timeline_time - seg.timeline_range.start_us();
            Some(seg.source_range.start_us() + offset)
        } else {
            None
        }
    }

    /// Maps an edited timeline position including the exact final endpoint $t = \text{total\_duration}$.
    ///
    /// When $t == \text{total\_duration}$, returns the end timestamp of the final kept segment.
    pub fn timeline_to_source_endpoint(&self, timeline_time: MediaTimeUs) -> Option<MediaTimeUs> {
        if timeline_time == self.total_duration_us && !self.segments.is_empty() {
            return self.segments.last().map(|s| s.source_range.end_us());
        }
        self.timeline_to_source(timeline_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::edl::KeepSegment;
    use crate::editing::source::SourceReference;

    fn build_canonical_edl() -> EditDecisionList {
        // Canonical Example:
        // Source: 60s
        // Keep: [0, 10s), [20s, 35s), [40s, 60s)
        // Edited: 45s
        let src =
            SourceReference::new("canonical_test", MediaTimeUs::from_micros(60_000_000), None)
                .unwrap();
        let raw = vec![
            KeepSegment::new(SourceRange::from_micros(0, 10_000_000).unwrap()),
            KeepSegment::new(SourceRange::from_micros(20_000_000, 35_000_000).unwrap()),
            KeepSegment::new(SourceRange::from_micros(40_000_000, 60_000_000).unwrap()),
        ];
        EditDecisionList::new_normalized(src, raw).unwrap()
    }

    #[test]
    fn test_canonical_source_to_timeline_mapping() {
        let edl = build_canonical_edl();
        let index = TimelineIndex::build(&edl).unwrap();

        assert_eq!(
            index.total_duration_us(),
            MediaTimeUs::from_micros(45_000_000)
        );

        // Source 5s -> Timeline 5s
        assert_eq!(
            index.source_to_timeline(MediaTimeUs::from_micros(5_000_000)),
            Some(MediaTimeUs::from_micros(5_000_000))
        );

        // Source 22s -> Timeline 12s
        assert_eq!(
            index.source_to_timeline(MediaTimeUs::from_micros(22_000_000)),
            Some(MediaTimeUs::from_micros(12_000_000))
        );

        // Source 45s -> Timeline 30s
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
            index.source_to_timeline(MediaTimeUs::from_micros(38_000_000)),
            None
        );

        // Outside source bounds return None
        assert_eq!(
            index.source_to_timeline(MediaTimeUs::from_micros(65_000_000)),
            None
        );
        assert_eq!(
            index.source_to_timeline(MediaTimeUs::from_micros(-10)),
            None
        );
    }

    #[test]
    fn test_canonical_timeline_to_source_mapping() {
        let edl = build_canonical_edl();
        let index = TimelineIndex::build(&edl).unwrap();

        // Timeline 5s -> Source 5s
        assert_eq!(
            index.timeline_to_source(MediaTimeUs::from_micros(5_000_000)),
            Some(MediaTimeUs::from_micros(5_000_000))
        );

        // Timeline 12s -> Source 22s
        assert_eq!(
            index.timeline_to_source(MediaTimeUs::from_micros(12_000_000)),
            Some(MediaTimeUs::from_micros(22_000_000))
        );

        // Timeline 30s -> Source 45s
        assert_eq!(
            index.timeline_to_source(MediaTimeUs::from_micros(30_000_000)),
            Some(MediaTimeUs::from_micros(45_000_000))
        );

        // Boundary concatenation semantics:
        // Segment 1 ends at 10s timeline, Segment 2 starts at 10s timeline.
        // Half-open interval [10s, 25s) selects start of Segment 2 (source 20s).
        assert_eq!(
            index.timeline_to_source(MediaTimeUs::from_micros(10_000_000)),
            Some(MediaTimeUs::from_micros(20_000_000))
        );

        // At exactly total_duration (45s), half-open timeline_to_source returns None
        assert_eq!(
            index.timeline_to_source(MediaTimeUs::from_micros(45_000_000)),
            None
        );

        // But endpoint mapping returns end of final segment (source 60s)
        assert_eq!(
            index.timeline_to_source_endpoint(MediaTimeUs::from_micros(45_000_000)),
            Some(MediaTimeUs::from_micros(60_000_000))
        );
    }
}
