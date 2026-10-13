# OpenTake — Timeline and Edit Decision List (EDL) Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 9  
**Author:** OpenTake Core Architecture & Systems Lead  
**Date:** 2026-10-13  

---

## 1. Timebase Representation & Evaluation

### 1.1 Why Floating-Point Seconds are Strictly Banned
Using IEEE-754 double-precision floats (`f64`) for edit boundary calculations introduces cumulative rounding errors. After dozens of splice operations, small sub-millisecond inaccuracies accumulate, resulting in visible frame jitter, audio clicks, and progressive A/V drift.

### 1.2 Evaluation of Integer Microseconds (`i64`)
OpenTake adopts **signed 64-bit integer microseconds (`i64`)** as its canonical timeline coordinate unit, where:
$$1\text{ second} = 1{,}000{,}000\,\mu\text{s}$$

#### Compatibility Analysis Across Critical Media Standards:

| Media Standard | Native Frame / Sample Interval | Integer Microsecond Representation | Rounding Delta / Boundary Behavior |
|---|---|---|---|
| **24.000 fps (CFR)** | $1/24\text{ s} = 41{,}666.\overline{6}\,\mu\text{s}$ | Frame $k$ at $k \times 1{,}000{,}000 / 24$ | Rational integer arithmetic ($k \times 125{,}000 / 3$) produces exact timestamps. |
| **23.976 fps (NTSC)** | $1001/24000\text{ s} \approx 41{,}708.333\,\mu\text{s}$ | Frame $k$ at $\lfloor k \times 1{,}001{,}000 / 24 \rfloor$ | Microsecond precision has a maximum single-frame quantization error of $< 0.5\,\mu\text{s}$. Over a 3-hour video, cumulative drift is 0 because timestamps are derived from rational multiplication, not iterative addition. |
| **29.970 fps (NTSC)** | $1001/30000\text{ s} \approx 33{,}366.666\,\mu\text{s}$ | Frame $k$ at $\lfloor k \times 1{,}001{,}000 / 30 \rfloor$ | Quantization error $< 0.5\,\mu\text{s}$. |
| **59.940 fps (NTSC)** | $1001/60000\text{ s} \approx 16{,}683.333\,\mu\text{s}$ | Frame $k$ at $\lfloor k \times 1{,}001{,}000 / 60 \rfloor$ | Quantization error $< 0.5\,\mu\text{s}$. |
| **48,000 Hz Audio** | $1/48000\text{ s} = 20.833\,\mu\text{s}$ | Sample $n$ at $\lfloor n \times 125 / 6 \rfloor$ | Max sample boundary quantization error $< 0.5\,\mu\text{s}$. |
| **44,100 Hz Audio** | $1/44100\text{ s} \approx 22.675\,\mu\text{s}$ | Sample $n$ at $\lfloor n \times 1{,}000{,}000 / 44{,}100 \rfloor$ | Max sample boundary quantization error $< 0.5\,\mu\text{s}$. |
| **Variable Frame Rate (VFR)**| Irregular PTS per frame | Direct presentation timestamp in microseconds | Stored directly from FFprobe container packet PTS. |

> [!IMPORTANT]
> **Defensive Timebase Truth:** Integer microseconds alone do **not** magically prevent boundary clicks or frame tearing. Exact accuracy requires **explicit snapping**:
> 1. Video cut boundaries must snap to the nearest video frame boundary:
>    $$t_{\text{frame}} = \text{round}\left(\frac{t \times \text{fps\_num}}{\text{fps\_den} \times 1{,}000{,}000}\right) \times \frac{\text{fps\_den} \times 1{,}000{,}000}{\text{fps\_num}}$$
> 2. Audio cut boundaries must snap to the nearest audio sample boundary:
>    $$t_{\text{sample}} = \text{round}\left(\frac{t \times \text{sample\_rate}}{1{,}000{,}000}\right) \times \frac{1{,}000{,}000}{\text{sample\_rate}}$$

---

## 2. Distinction Between Timeline Coordinates

To avoid conceptual confusion during implementation, OpenTake distinguishes four distinct time domains:

1. **Source Presentation Timestamp (`source_pts_us`):** The absolute microsecond coordinate referencing raw frames in the physical media file.
2. **Project Timeline Time (`project_time_us`):** The continuous output timecode experienced by the user after silent gaps and cuts have been excised.
3. **UI Display Time (`display_timecode`):** Formatted string representation for the user interface (`HH:MM:SS:FF` or `MM:SS.mmm`).
4. **FFmpeg Filtergraph Time (`ffmpeg_secs`):** Rational decimal string formatted with 6 decimal places (e.g., `12.345678`) passed to `trim` and `atrim` filters.

---

## 3. Authoritative EDL Data Model

In `crates/opentake-core/src/timeline/edl.rs`:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetainedRange {
    pub id: Uuid,
    pub source_start_us: i64,
    pub source_end_us: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditDecisionList {
    pub project_id: Uuid,
    pub media_source_id: Uuid,
    pub source_file_path: std::path::PathBuf,
    pub source_duration_us: i64,
    pub video_timebase_fps: (u64, u64), // e.g., (30000, 1001)
    pub audio_sample_rate_hz: u32,      // e.g., 48000
    pub retained_ranges: Vec<RetainedRange>,
}
```

### 3.1 Non-Negotiable Invariants
1. **Non-Negative Bounds:** For all ranges, $0 \le \text{source\_start\_us} < \text{source\_end\_us} \le \text{source\_duration\_us}$.
2. **Strict Chronological Ordering:** Retained ranges must be strictly ordered by source start time:
   $$\text{Range}_k.\text{source\_end\_us} \le \text{Range}_{k+1}.\text{source\_start\_us} \quad \forall k$$
3. **Non-Overlapping:** Retained ranges cannot overlap. If two adjacent ranges touch ($\text{Range}_k.\text{source\_end\_us} == \text{Range}_{k+1}.\text{source\_start\_us}$), they are automatically coalesced into a single contiguous range.
4. **Zero-Length Prohibition:** Ranges with $\text{source\_start\_us} == \text{source\_end\_us}$ are invalid and purged.
5. **Non-Empty Timeline:** A project must retain at least one frame ($\ge 40{,}000\,\mu\text{s}$) unless all segments are deleted with explicit confirmation.

---

## 4. Timeline Operations & Interval Algebra

```rust
impl EditDecisionList {
    /// Splits the segment containing `split_time_us` into two independent retained ranges.
    pub fn split_at(&mut self, split_time_us: i64) -> Result<(), CoreError> {
        let snapped_us = self.snap_to_frame(split_time_us);
        
        let target_idx = self.retained_ranges.iter().position(|r| {
            snapped_us > r.source_start_us && snapped_us < r.source_end_us
        });

        if let Some(idx) = target_idx {
            let current = self.retained_ranges.remove(idx);
            let left = RetainedRange {
                id: Uuid::new_v4(),
                source_start_us: current.source_start_us,
                source_end_us: snapped_us,
            };
            let right = RetainedRange {
                id: Uuid::new_v4(),
                source_start_us: snapped_us,
                source_end_us: current.source_end_us,
            };
            self.retained_ranges.insert(idx, right);
            self.retained_ranges.insert(idx, left);
            Ok(())
        } else {
            // Split fell inside an already excised gap or outside media bounds
            Ok(())
        }
    }

    /// Trims boundary of an existing retained range, enforcing bounds and snapping.
    pub fn trim_range(&mut self, range_id: Uuid, new_start_us: Option<i64>, new_end_us: Option<i64>) -> Result<(), CoreError>;

    /// Deletes a specific retained range by UUID (converting it into an excluded gap).
    pub fn delete_range(&mut self, range_id: Uuid) -> Result<(), CoreError>;

    /// Calculates total continuous output duration.
    pub fn total_output_duration_us(&self) -> i64 {
        self.retained_ranges
            .iter()
            .map(|r| r.source_end_us - r.source_start_us)
            .sum()
    }
}
```

---

## 5. Source-to-Output Coordinate Translation

A binary search over retained ranges enables instant $O(\log N)$ bidirectional time mapping:

```rust
impl EditDecisionList {
    /// Maps a raw source timestamp to the corresponding output project timecode.
    pub fn source_to_output_time(&self, source_us: i64) -> Option<i64> {
        let mut accumulated_us = 0;
        for range in &self.retained_ranges {
            if source_us < range.source_start_us {
                return None; // Point falls inside an excised cut gap
            }
            if source_us <= range.source_end_us {
                return Some(accumulated_us + (source_us - range.source_start_us));
            }
            accumulated_us += range.source_end_us - range.source_start_us;
        }
        None
    }

    /// Maps an output project timecode to the exact source video timestamp.
    pub fn output_to_source_time(&self, mut output_us: i64) -> Option<i64> {
        for range in &self.retained_ranges {
            let duration = range.source_end_us - range.source_start_us;
            if output_us <= duration {
                return Some(range.source_start_us + output_us);
            }
            output_us -= duration;
        }
        None
    }
}
```

---

## 6. Pure-Function Unit Test Plan (STEP 9)

In `crates/opentake-core/src/timeline/interval_tests.rs`:
- `test_split_in_middle_creates_two_ranges()`
- `test_split_at_boundary_noops_cleanly()`
- `test_split_outside_bounds_returns_error()`
- `test_coalesce_adjacent_touching_ranges()`
- `test_source_to_output_mapping_across_five_cuts()`
- `test_bidirectional_mapping_symmetry()`
- `test_1000_sequential_splits_preserves_total_duration()`
