# OpenTake — Timeline and Edit Decision List (EDL) Architecture

**Document Version:** 1.1.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implemented (STEP 9 Verified)  
**Location in Workspace:** `crates/opentake-core/src/editing/`  
**Date:** 2026-10-14  

---

## 1. Canonical Timebase & Representation

### 1.1 Why Floating-Point Seconds are Strictly Banned
Using IEEE-754 double-precision floats (`f64`) for edit boundary calculations introduces cumulative rounding errors. After dozens of splice operations, small sub-millisecond inaccuracies accumulate, resulting in visible frame jitter, audio clicks, and progressive A/V drift.

### 1.2 Integer Microseconds (`MediaTimeUs`)
OpenTake adopts **signed 64-bit integer microseconds (`i64`)** wrapped in the validated domain type `MediaTimeUs`:
$$1\text{ second} = 1{,}000{,}000\,\mu\text{s}$$

#### Media Standards Compatibility Matrix:

| Media Standard | Native Frame / Sample Interval | Integer Microsecond Representation | Rounding Delta / Boundary Behavior |
|---|---|---|---|
| **24.000 fps (CFR)** | $1/24\text{ s} = 41{,}666.\overline{6}\,\mu\text{s}$ | Frame $k$ at $k \times 1{,}000{,}000 / 24$ | Rational integer arithmetic ($k \times 125{,}000 / 3$) produces exact timestamps. |
| **23.976 fps (NTSC)** | $1001/24000\text{ s} \approx 41{,}708.333\,\mu\text{s}$ | Frame $k$ at $\lfloor k \times 1{,}001{,}000 / 24 \rfloor$ | Single-frame quantization error $< 0.5\,\mu\text{s}$. Zero cumulative drift because timestamps derive from rational multiplication, not iterative addition. |
| **29.970 fps (NTSC)** | $1001/30000\text{ s} \approx 33{,}366.666\,\mu\text{s}$ | Frame $k$ at $\lfloor k \times 1{,}001{,}000 / 30 \rfloor$ | Quantization error $< 0.5\,\mu\text{s}$. |
| **59.940 fps (NTSC)** | $1001/60000\text{ s} \approx 16{,}683.333\,\mu\text{s}$ | Frame $k$ at $\lfloor k \times 1{,}001{,}000 / 60 \rfloor$ | Quantization error $< 0.5\,\mu\text{s}$. |
| **48,000 Hz Audio** | $1/48000\text{ s} = 20.833\,\mu\text{s}$ | Sample $n$ at $\lfloor n \times 125 / 6 \rfloor$ | Boundary quantization error $< 0.5\,\mu\text{s}$. |
| **44,100 Hz Audio** | $1/44100\text{ s} \approx 22.675\,\mu\text{s}$ | Sample $n$ at $\lfloor n \times 1{,}000{,}000 / 44{,}100 \rfloor$ | Boundary quantization error $< 0.5\,\mu\text{s}$. |
| **Variable Frame Rate (VFR)**| Irregular PTS per packet | Direct microsecond PTS | Converted via rational stream timebase (`from_pts`). |

### 1.3 Rational Conversions and 128-bit Checked Arithmetic
All frame, audio sample, and PTS conversions use intermediate `i128` arithmetic to prevent multiplication overflow:
```rust
let num_128 = (frame_index as i128)
    .checked_mul(1_000_000)
    .and_then(|v| v.checked_mul(fps.den as i128))
    .ok_or(EditError::ArithmeticOverflow)?;
let us = div_round_i128(num_128, fps.num as i128, round)?;
```
Supported rounding modes: `RoundMode::Floor`, `RoundMode::Ceil`, `RoundMode::Nearest` (half rounded away from zero).

---

## 2. Distinction Between Timeline Coordinates

OpenTake strictly distinguishes four distinct time domains:

1. **Source Presentation Timestamp (`source_pts_us`):** Absolute microsecond coordinate referencing raw frames in the physical media file.
2. **Project Timeline Time (`project_time_us`):** Continuous output timecode experienced after silent gaps and cuts have been excised.
3. **UI Display Time (`display_timecode`):** Formatted string representation for the user interface (`HH:MM:SS:FF` or `MM:SS.mmm`).
4. **FFmpeg Filtergraph Time (`ffmpeg_secs`):** Rational decimal string formatted with 6 decimal places (e.g., `12.345678`) passed to `trim` and `atrim` filters.

---

## 3. Half-Open Interval Algebra (`SourceRange`)

All temporal segments are represented as validated half-open intervals $[start\_us, end\_us)$:
- `start_us` is **inclusive**.
- `end_us` is **exclusive**.
- Duration is exactly $end\_us - start\_us$.
- Invariant: $0 \le start\_us < end\_us$.
- Directly adjacent intervals $[0, 10)$ and $[10, 20)$ share the boundary 10 without overlapping.

### Set Operations:
- **Overlap:** $\max(s_1, s_2) < \min(e_1, e_2)$
- **Adjacency:** $e_1 == s_2 \lor e_2 == s_1$
- **Intersection:** $[\max(s_1, s_2), \min(e_1, e_2))$ if overlapping
- **Hull:** $[\min(s_1, s_2), \max(e_1, e_2))$
- **Subtraction:** $R \setminus C$ yields 0, 1, or 2 residual sub-intervals.

---

## 4. Authoritative EDL Data Model

Implemented in `crates/opentake-core/src/editing/`:

```rust
pub struct SourceReference {
    pub source_id: String,
    pub duration_us: MediaTimeUs,
    pub fingerprint: Option<String>,
}

pub struct KeepSegment {
    pub id: Uuid,
    pub range: SourceRange,
}

pub struct EditDecisionList {
    schema_version: u32,
    source: SourceReference,
    segments: Vec<KeepSegment>,
}
```

### 4.1 Canonical Normalization Invariants
1. **Non-Negative & Bounded:** $0 \le \text{start\_us} < \text{end\_us} \le \text{source.duration\_us}$ for all segments.
2. **Strict Chronological Ordering:** Segments are strictly ordered by source start:
   $$s_k.\text{end\_us} < s_{k+1}.\text{start\_us} \quad \forall k$$
3. **Non-Overlapping & Coalesced:** Canonical segments never overlap. Adjacent touching segments ($s_k.\text{end} == s_{k+1}.\text{start}$) are automatically merged into a single continuous range.
4. **Explicit Empty Timeline:** An EDL with 0 segments represents a valid empty timeline (0 duration).
5. **Full Source Representation:** A full EDL has exactly 1 segment $[0, \text{source.duration\_us})$.
6. **Idempotence:** $\text{normalize}(\text{normalize}(\text{edl})) == \text{normalize}(\text{edl})$.

---

## 5. Immutable Domain Operations

Domain transformations produce a new canonical `EditDecisionList` without mutating inputs or touching underlying media files:

1. **`cut_source_range(&edl, cut_range) -> Result<EditDecisionList, EditError>`:**
   Subtracts a source interval from kept segments. Repeated cuts are idempotent.
2. **`restore_source_range(&edl, restore_range) -> Result<EditDecisionList, EditError>`:**
   Restores an excised source interval back into the kept list, merging adjacent and overlapping segments.
3. **`keep_only(&edl, keep_range) -> Result<EditDecisionList, EditError>`:**
   Intersects currently kept segments with a single bounding range.
4. **`cut_timeline_range(&edl, timeline_cut) -> Result<EditDecisionList, EditError>`:**
   Cuts a range specified in continuous edited timeline coordinates, correctly splitting across multiple discontinuous source segments.

---

## 6. Bidirectional Coordinate Mapping (`TimelineIndex`)

The `TimelineIndex` derives prefix sums of kept durations for $O(\log N)$ binary search lookup:

```rust
pub struct IndexedSegment {
    pub segment_id: Uuid,
    pub source_range: SourceRange,
    pub timeline_range: SourceRange,
}

pub struct TimelineIndex {
    segments: Vec<IndexedSegment>,
    total_duration_us: MediaTimeUs,
}
```

### 6.1 Mapping Rules:
- **`source_to_timeline(source_us) -> Option<MediaTimeUs>`:**
  Returns `Some(t)` if `source_us` lies inside a retained segment. Returns `None` if `source_us` falls inside an excised cut gap.
- **`timeline_to_source(timeline_us) -> Option<MediaTimeUs>`:**
  Maps playback position $[0, \text{total\_duration})$ to raw source time. At concatenation boundaries, half-open interval $[t_k.\text{start}, t_k.\text{end})$ selects the following segment.
- **`timeline_to_source_endpoint(timeline_us) -> Option<MediaTimeUs>`:**
  When $t == \text{total\_duration}$, returns the end timestamp of the final kept segment.

---

## 7. Canonical Example Verification

```
Source Duration: 60s
Kept Segments:   [0, 10s), [20s, 35s), [40s, 60s)
Edited Duration: 45s

Source -> Timeline:
  Source  5s -> Timeline  5s
  Source 22s -> Timeline 12s
  Source 45s -> Timeline 30s
  Source 15s -> None (cut region)

Timeline -> Source:
  Timeline  5s -> Source  5s
  Timeline 12s -> Source 22s
  Timeline 30s -> Source 45s
  Timeline 10s -> Source 20s (concatenation boundary)
  Timeline 45s -> Source 60s (endpoint)
```

---

## 8. STEP 10 Handoff & Future Roadmap Alignment

The STEP 9 EDL engine is headless and ready for downstream integration:
- **STEP 10 (Visual Timeline & Manual Editing):** React timeline UI renders kept blocks and cut gaps by querying `edl.segments()` and `index.timeline_to_source()`.
- **STEP 11 (Undo/Redo):** History manager stores immutable `EditDecisionList` snapshots.
- **STEP 12 (Project Persistence):** Serializes `EditDecisionList` with schema version 1.
- **STEP 16 (Smart Cut Generation):** Silence detection DSP produces candidate cut ranges passed directly to `cut_source_range()`.
- **STEP 18 (Export Timeline Compiler):** Compiles `edl.segments()` into FFmpeg `trim`/`atrim` and `concat` filtergraphs.
