use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::error::EditError;
use super::time::MediaTimeUs;

/// A validated, non-empty, half-open microsecond interval $[start\_us, end\_us)$.
///
/// # Half-Open Interval Semantics
/// - `start_us` is **inclusive**.
/// - `end_us` is **exclusive**.
/// - Duration is exactly `end_us - start_us`.
/// - Adjacent intervals $[0, 10)$ and $[10, 20)$ share endpoint 10 without overlapping.
/// - Invariant: $0 \le start\_us < end\_us$.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRange {
    start_us: MediaTimeUs,
    end_us: MediaTimeUs,
}

impl SourceRange {
    /// Constructs and validates a half-open interval $[start\_us, end\_us)$.
    ///
    /// # Errors
    /// Returns `EditError::NegativeTime` if `start_us < 0`.
    /// Returns `EditError::ZeroLengthRange` if `start_us == end_us`.
    /// Returns `EditError::InvalidInterval` if `start_us > end_us`.
    pub fn new(start_us: MediaTimeUs, end_us: MediaTimeUs) -> Result<Self, EditError> {
        if !start_us.is_non_negative() {
            return Err(EditError::NegativeTime {
                time_us: start_us.as_micros(),
            });
        }
        if start_us == end_us {
            return Err(EditError::ZeroLengthRange {
                time_us: start_us.as_micros(),
            });
        }
        if start_us > end_us {
            return Err(EditError::InvalidInterval {
                start_us: start_us.as_micros(),
                end_us: end_us.as_micros(),
            });
        }

        Ok(Self { start_us, end_us })
    }

    /// Constructs a `SourceRange` directly from raw integer microseconds.
    pub fn from_micros(start_us: i64, end_us: i64) -> Result<Self, EditError> {
        Self::new(
            MediaTimeUs::from_micros(start_us),
            MediaTimeUs::from_micros(end_us),
        )
    }

    /// Returns the inclusive start timestamp.
    #[inline]
    pub const fn start_us(&self) -> MediaTimeUs {
        self.start_us
    }

    /// Returns the exclusive end timestamp.
    #[inline]
    pub const fn end_us(&self) -> MediaTimeUs {
        self.end_us
    }

    /// Calculates the non-zero duration: $end\_us - start\_us$.
    #[inline]
    pub fn duration(&self) -> MediaTimeUs {
        self.end_us - self.start_us
    }

    /// Checks whether a timestamp falls within this half-open interval ($start\_us \le t < end\_us$).
    #[inline]
    pub fn contains_time(&self, time: MediaTimeUs) -> bool {
        time >= self.start_us && time < self.end_us
    }

    /// Checks whether another interval is entirely contained within this interval.
    #[inline]
    pub fn contains_range(&self, other: &Self) -> bool {
        self.start_us <= other.start_us && other.end_us <= self.end_us
    }

    /// Checks whether two intervals overlap (share a non-empty intersection):
    /// $$\max(s_1, s_2) < \min(e_1, e_2)$$
    #[inline]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start_us.max(other.start_us) < self.end_us.min(other.end_us)
    }

    /// Checks whether two intervals are directly adjacent without a gap:
    /// $$e_1 == s_2 \lor e_2 == s_1$$
    #[inline]
    pub fn is_adjacent(&self, other: &Self) -> bool {
        self.end_us == other.start_us || other.end_us == self.start_us
    }

    /// Computes the non-empty intersection of two intervals, if they overlap.
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let max_start = self.start_us.max(other.start_us);
        let min_end = self.end_us.min(other.end_us);

        if max_start < min_end {
            Self::new(max_start, min_end).ok()
        } else {
            None
        }
    }

    /// Computes the minimal bounding interval (convex hull) spanning both intervals:
    /// $$[\min(s_1, s_2), \max(e_1, e_2))$$
    pub fn hull(&self, other: &Self) -> Self {
        Self {
            start_us: self.start_us.min(other.start_us),
            end_us: self.end_us.max(other.end_us),
        }
    }

    /// Subtracts a cut interval $[c_{\text{start}}, c_{\text{end}})$ from this interval,
    /// returning 0, 1, or 2 resulting sub-intervals.
    ///
    /// - If the cut does not overlap, returns `[*self]`.
    /// - If the cut entirely covers this interval, returns `[]`.
    /// - If the cut trims the start, returns `[[c_end, self.end)]`.
    /// - If the cut trims the end, returns `[[self.start, c_start)]`.
    /// - If the cut splits the interior, returns `[[self.start, c_start), [c_end, self.end)]`.
    pub fn subtract(&self, cut: &Self) -> Vec<Self> {
        if !self.overlaps(cut) {
            return vec![*self];
        }

        let mut result = Vec::with_capacity(2);

        // Retain left residual segment if cut starts after self.start
        if self.start_us < cut.start_us {
            if let Ok(left) = Self::new(self.start_us, cut.start_us) {
                result.push(left);
            }
        }

        // Retain right residual segment if cut ends before self.end
        if cut.end_us < self.end_us {
            if let Ok(right) = Self::new(cut.end_us, self.end_us) {
                result.push(right);
            }
        }

        result
    }
}

impl fmt::Display for SourceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {})", self.start_us, self.end_us)
    }
}

// ---------------------------------------------------------------------------
// Serde Serialization with Invariant Validation
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SourceRangeWire {
    #[serde(rename = "startUs")]
    start_us: i64,
    #[serde(rename = "endUs")]
    end_us: i64,
}

impl Serialize for SourceRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SourceRangeWire {
            start_us: self.start_us.as_micros(),
            end_us: self.end_us.as_micros(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SourceRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SourceRangeWire::deserialize(deserializer)?;
        Self::from_micros(wire.start_us, wire.end_us).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_source_range_construction() {
        let range = SourceRange::from_micros(0, 10_000_000).unwrap();
        assert_eq!(range.start_us(), MediaTimeUs(0));
        assert_eq!(range.end_us(), MediaTimeUs(10_000_000));
        assert_eq!(range.duration(), MediaTimeUs(10_000_000));
    }

    #[test]
    fn test_invalid_interval_rejections() {
        // Negative start
        assert!(SourceRange::from_micros(-1, 10).is_err());
        // Zero length
        assert!(SourceRange::from_micros(10, 10).is_err());
        // Reversed bounds
        assert!(SourceRange::from_micros(20, 10).is_err());
    }

    #[test]
    fn test_containment_and_overlap() {
        let r1 = SourceRange::from_micros(10, 30).unwrap();
        let r2 = SourceRange::from_micros(20, 40).unwrap();
        let r3 = SourceRange::from_micros(30, 50).unwrap();
        let inner = SourceRange::from_micros(15, 25).unwrap();

        assert!(r1.contains_time(MediaTimeUs(10)));
        assert!(r1.contains_time(MediaTimeUs(29)));
        assert!(!r1.contains_time(MediaTimeUs(30))); // Half-open exclusive end

        assert!(r1.contains_range(&inner));
        assert!(r1.overlaps(&r2));
        assert!(!r1.overlaps(&r3)); // Adjacent, not overlapping
        assert!(r1.is_adjacent(&r3));
    }

    #[test]
    fn test_intersection_and_hull() {
        let r1 = SourceRange::from_micros(10, 30).unwrap();
        let r2 = SourceRange::from_micros(20, 40).unwrap();

        let inter = r1.intersection(&r2).unwrap();
        assert_eq!(inter, SourceRange::from_micros(20, 30).unwrap());

        let hull = r1.hull(&r2);
        assert_eq!(hull, SourceRange::from_micros(10, 40).unwrap());
    }

    #[test]
    fn test_subtraction_cases() {
        let base = SourceRange::from_micros(10, 50).unwrap();

        // 1. Cut inside -> splits into two
        let cut_inside = SourceRange::from_micros(20, 30).unwrap();
        let split = base.subtract(&cut_inside);
        assert_eq!(split.len(), 2);
        assert_eq!(split[0], SourceRange::from_micros(10, 20).unwrap());
        assert_eq!(split[1], SourceRange::from_micros(30, 50).unwrap());

        // 2. Cut left
        let cut_left = SourceRange::from_micros(0, 25).unwrap();
        let left_res = base.subtract(&cut_left);
        assert_eq!(left_res.len(), 1);
        assert_eq!(left_res[0], SourceRange::from_micros(25, 50).unwrap());

        // 3. Cut right
        let cut_right = SourceRange::from_micros(40, 60).unwrap();
        let right_res = base.subtract(&cut_right);
        assert_eq!(right_res.len(), 1);
        assert_eq!(right_res[0], SourceRange::from_micros(10, 40).unwrap());

        // 4. Cut covers all
        let cut_all = SourceRange::from_micros(5, 55).unwrap();
        assert!(base.subtract(&cut_all).is_empty());

        // 5. Cut disjoint
        let cut_disjoint = SourceRange::from_micros(60, 70).unwrap();
        let dis_res = base.subtract(&cut_disjoint);
        assert_eq!(dis_res.len(), 1);
        assert_eq!(dis_res[0], base);
    }
}
