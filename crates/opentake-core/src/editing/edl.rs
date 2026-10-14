use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use super::error::EditError;
use super::interval::SourceRange;
use super::source::SourceReference;
use super::time::MediaTimeUs;

/// Current supported EDL schema serialization version.
pub const CURRENT_EDL_SCHEMA_VERSION: u32 = 1;

/// An individual retained segment within the non-destructive Edit Decision List.
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepSegment {
    /// Unique identifier for this retained clip segment.
    pub id: Uuid,
    /// Authoritative source-time interval $[start\_us, end\_us)$ retained from the original media.
    pub range: SourceRange,
}

impl PartialEq for KeepSegment {
    fn eq(&self, other: &Self) -> bool {
        self.range == other.range
    }
}

impl KeepSegment {
    /// Constructs a new retained segment with a randomly generated UUID.
    pub fn new(range: SourceRange) -> Self {
        Self {
            id: Uuid::new_v4(),
            range,
        }
    }

    /// Constructs a retained segment with an explicit UUID.
    pub fn with_id(id: Uuid, range: SourceRange) -> Self {
        Self { id, range }
    }

    /// Returns the segment duration: $end\_us - start\_us$.
    #[inline]
    pub fn duration(&self) -> MediaTimeUs {
        self.range.duration()
    }
}

/// The authoritative, Rust-owned, non-destructive Edit Decision List (EDL).
///
/// Represents editing decisions mathematically as an ordered sequence of non-overlapping,
/// disjoint keep-intervals referencing a validated media source.
///
/// # Canonical Normalization Invariants
/// 1. $0 \le \text{start\_us} < \text{end\_us} \le \text{source.duration\_us}$ for all segments.
/// 2. Segments are strictly ordered by source time: $s_k.\text{end\_us} < s_{k+1}.\text{start\_us}$.
/// 3. Overlapping and adjacent segments are coalesced into a single continuous range.
/// 4. An empty EDL has 0 segments and 0 edited duration (explicit empty timeline state).
/// 5. A full EDL has exactly 1 segment spanning $[0, \text{source.duration\_us})$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDecisionList {
    schema_version: u32,
    source: SourceReference,
    segments: Vec<KeepSegment>,
}

impl EditDecisionList {
    /// Creates a full-source EDL where the entire media is retained: $[0, \text{source.duration\_us})$.
    pub fn new_full_source(source: SourceReference) -> Result<Self, EditError> {
        let full_range = SourceRange::new(MediaTimeUs::ZERO, source.duration_us)?;
        Ok(Self {
            schema_version: CURRENT_EDL_SCHEMA_VERSION,
            source,
            segments: vec![KeepSegment::new(full_range)],
        })
    }

    /// Creates an empty EDL retaining zero duration (all media excised).
    pub fn new_empty(source: SourceReference) -> Result<Self, EditError> {
        Ok(Self {
            schema_version: CURRENT_EDL_SCHEMA_VERSION,
            source,
            segments: Vec::new(),
        })
    }

    /// Constructs an EDL from raw segments by applying canonical normalization.
    ///
    /// # Normalization Process
    /// 1. Verifies each segment does not exceed `source.duration_us`.
    /// 2. Sorts segments by `start_us`.
    /// 3. Merges overlapping and touching (adjacent) intervals.
    /// 4. Produces a strictly ordered, canonical sequence of keep segments.
    pub fn new_normalized(
        source: SourceReference,
        raw_segments: Vec<KeepSegment>,
    ) -> Result<Self, EditError> {
        let mut sorted = raw_segments;
        sorted.sort_by_key(|s| s.range.start_us());

        let mut canonical: Vec<KeepSegment> = Vec::with_capacity(sorted.len());

        for seg in sorted {
            // Verify bounds against source duration
            if seg.range.end_us() > source.duration_us {
                return Err(EditError::SourceOutOfBounds {
                    start_us: seg.range.start_us().as_micros(),
                    end_us: seg.range.end_us().as_micros(),
                    source_duration_us: source.duration_us.as_micros(),
                });
            }

            if let Some(last) = canonical.last_mut() {
                if last.range.overlaps(&seg.range) || last.range.is_adjacent(&seg.range) {
                    // Merge into a contiguous hull
                    last.range = last.range.hull(&seg.range);
                } else if seg.range.start_us() > last.range.end_us() {
                    canonical.push(seg);
                } else {
                    // Unreachable due to sort, but safe guard
                    return Err(EditError::NonCanonicalSegments {
                        reason: format!("Unexpected segment order: {:?} after {:?}", seg, last),
                    });
                }
            } else {
                canonical.push(seg);
            }
        }

        Ok(Self {
            schema_version: CURRENT_EDL_SCHEMA_VERSION,
            source,
            segments: canonical,
        })
    }

    /// Returns the schema version.
    #[inline]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the associated source reference.
    #[inline]
    pub const fn source(&self) -> &SourceReference {
        &self.source
    }

    /// Returns the ordered slice of canonical keep segments.
    #[inline]
    pub fn segments(&self) -> &[KeepSegment] {
        &self.segments
    }

    /// Returns true if the EDL retains zero segments (empty timeline).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Returns true if the EDL retains the entire source without any cuts.
    pub fn is_full_source(&self) -> bool {
        if self.segments.len() != 1 {
            return false;
        }
        let seg = &self.segments[0];
        seg.range.start_us() == MediaTimeUs::ZERO && seg.range.end_us() == self.source.duration_us
    }

    /// Returns the number of kept segments.
    #[inline]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Calculates the total edited output duration: $\sum \text{segment.duration()}$.
    ///
    /// Uses checked arithmetic to protect against overflow.
    pub fn edited_duration_us(&self) -> Result<MediaTimeUs, EditError> {
        let mut total = MediaTimeUs::ZERO;
        for seg in &self.segments {
            total = total.checked_add(seg.duration())?;
        }
        Ok(total)
    }

    /// Computes total excised (cut) duration: $\text{source\_duration} - \text{edited\_duration}$.
    pub fn cut_duration_us(&self) -> Result<MediaTimeUs, EditError> {
        self.source
            .duration_us
            .checked_sub(self.edited_duration_us()?)
    }

    /// Computes the complementary cut ranges (gaps excised from the original source).
    pub fn cut_ranges(&self) -> Result<Vec<SourceRange>, EditError> {
        let mut cuts = Vec::new();
        let mut cursor = MediaTimeUs::ZERO;

        for seg in &self.segments {
            if seg.range.start_us() > cursor {
                cuts.push(SourceRange::new(cursor, seg.range.start_us())?);
            }
            cursor = seg.range.end_us();
        }

        if cursor < self.source.duration_us {
            cuts.push(SourceRange::new(cursor, self.source.duration_us)?);
        }

        Ok(cuts)
    }
}

// ---------------------------------------------------------------------------
// Serde Serialization with Deserialization Validation Invariants
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditDecisionListWire {
    schema_version: u32,
    source: SourceReference,
    segments: Vec<KeepSegment>,
}

impl Serialize for EditDecisionList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        EditDecisionListWire {
            schema_version: self.schema_version,
            source: self.source.clone(),
            segments: self.segments.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EditDecisionList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EditDecisionListWire::deserialize(deserializer)?;

        if wire.schema_version != CURRENT_EDL_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "Unsupported EDL schema version: {} (current: {})",
                wire.schema_version, CURRENT_EDL_SCHEMA_VERSION
            )));
        }

        Self::new_normalized(wire.source, wire.segments).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_source() -> SourceReference {
        SourceReference::new("test_asset", MediaTimeUs::from_micros(60_000_000), None).unwrap()
    }

    #[test]
    fn test_full_source_and_empty_edl() {
        let src = test_source();

        let full = EditDecisionList::new_full_source(src.clone()).unwrap();
        assert_eq!(full.segment_count(), 1);
        assert!(full.is_full_source());
        assert_eq!(full.edited_duration_us().unwrap(), MediaTimeUs(60_000_000));
        assert_eq!(full.cut_duration_us().unwrap(), MediaTimeUs::ZERO);
        assert!(full.cut_ranges().unwrap().is_empty());

        let empty = EditDecisionList::new_empty(src).unwrap();
        assert_eq!(empty.segment_count(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.edited_duration_us().unwrap(), MediaTimeUs::ZERO);
        assert_eq!(empty.cut_duration_us().unwrap(), MediaTimeUs(60_000_000));
        let cuts = empty.cut_ranges().unwrap();
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0], SourceRange::from_micros(0, 60_000_000).unwrap());
    }

    #[test]
    fn test_normalization_merges_overlapping_and_adjacent() {
        let src = test_source();

        // Segments: [20, 35), [0, 10), [10, 20) -> All touch/overlap, should normalize to single [0, 35)
        let raw = vec![
            KeepSegment::new(SourceRange::from_micros(20_000_000, 35_000_000).unwrap()),
            KeepSegment::new(SourceRange::from_micros(0, 10_000_000).unwrap()),
            KeepSegment::new(SourceRange::from_micros(10_000_000, 20_000_000).unwrap()),
        ];

        let edl = EditDecisionList::new_normalized(src, raw).unwrap();
        assert_eq!(edl.segment_count(), 1);
        assert_eq!(
            edl.segments()[0].range,
            SourceRange::from_micros(0, 35_000_000).unwrap()
        );
    }

    #[test]
    fn test_normalization_idempotence() {
        let src = test_source();
        let raw = vec![
            KeepSegment::new(SourceRange::from_micros(0, 10_000_000).unwrap()),
            KeepSegment::new(SourceRange::from_micros(20_000_000, 35_000_000).unwrap()),
            KeepSegment::new(SourceRange::from_micros(40_000_000, 60_000_000).unwrap()),
        ];

        let edl1 = EditDecisionList::new_normalized(src.clone(), raw).unwrap();
        let edl2 = EditDecisionList::new_normalized(src, edl1.segments().to_vec()).unwrap();

        assert_eq!(edl1, edl2);
    }

    #[test]
    fn test_out_of_bounds_rejection() {
        let src = test_source(); // 60s
        let raw = vec![KeepSegment::new(
            SourceRange::from_micros(50_000_000, 65_000_000).unwrap(),
        )];

        assert!(EditDecisionList::new_normalized(src, raw).is_err());
    }

    #[test]
    fn test_serde_roundtrip_with_validation() {
        let src = test_source();
        let edl = EditDecisionList::new_full_source(src).unwrap();

        let json = serde_json::to_string(&edl).unwrap();
        let deserialized: EditDecisionList = serde_json::from_str(&json).unwrap();

        assert_eq!(edl, deserialized);
    }
}
