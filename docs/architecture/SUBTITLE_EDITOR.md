# Subtitle Editor Architecture: Timing, Manipulation, Snapping & History

**Document Version:** 1.0.0  
**Status:** Implemented in Step 17  
**Scope:** Open Media Workbench — Localization Studio  

---

## 1. Overview & Architecture Philosophy

The subtitle editor in Open Media Workbench turns the canonical data models established in STEP 16 into a direct-manipulation, high-precision timeline editing environment. Built around microsecond integer precision (`MediaTimeUs`), the editor provides non-destructive cue manipulation, multi-target snapping, interactive waveform alignment, and deterministic multi-level history.

### Core Architectural Principles
1. **Zero Floating-Point Drift:** All internal timing logic, interval calculations, and persistence store time exclusively as 64-bit integer microseconds (`MediaTimeUs`).
2. **Unified Domain Contract:** The exact same operation rules (Add, Delete, UpdateText, SetTiming, Move, Resize, Split, Merge, BatchShift, Duplicate) are implemented with identical validation rules in the Rust processing core (`opentake-core::localization::subtitles::operations`) and the client domain engine (`src/lib/subtitleOperations.ts`).
3. **Zero-Byte Media Immutability:** Editing subtitles never touches or mutates the underlying video or audio containers.
4. **Non-Destructive History:** Edits are tracked in a multi-level transaction history, isolating native text input typing events from document-level timeline edits.
5. **Direct Manipulation with Waveform Alignment:** Subtitles render on an interactive `S1 Subtitle` lane positioned directly beneath the video and audio waveform canvases, sharing the exact same horizontal time ruler and viewport transformations.

---

## 2. Coordinate Spaces & Time Mapping

The editor operates across four explicitly mapped coordinate spaces:

```
[ Display Pixels (px) ]
         ↕ (Viewport Zoom & Scroll: x_px = (t_us - start_us) / span_us * width_px)
[ Timeline Track Time (MediaTimeUs) ]
         ↕ (1:1 Authoritative Mapping in Step 17)
[ Source Media Time (MediaTimeUs) ]
         ↕ (EDL Invariant: t_source >= 0)
[ Canonical ISO Media Container ]
```

### Display Pixel Coordinate Conversion
Given timeline container width $W_{px}$, visible viewport interval $[T_{start}, T_{end}]$ in microseconds, and timestamp $t$ in microseconds:

$$x_{px} = \frac{t - T_{start}}{T_{end} - T_{start}} \times W_{px}$$

Inverse mapping for pointer events:
$$t = T_{start} + \frac{x_{px}}{W_{px}} \times (T_{end} - T_{start})$$

---

## 3. Atomic Edit Operations (`SubtitleOperation`)

Editing operations are strongly typed and serialized over Tauri 2 IPC:

| Operation | Rust & TS Payload | Invariant Checks |
|---|---|---|
| `AddCue` | `{ trackId, cue }` | $start \ge 0$, $end > start$, sort maintained |
| `DeleteCue` | `{ trackId, cueId }` | Validates cue existence; removes cue |
| `DeleteCues` | `{ trackId, cueIds }` | Batch removes specified cue IDs |
| `UpdateCueText` | `{ trackId, cueId, text }` | Unicode preserved; multiline preserved |
| `UpdateCueSpeaker` | `{ trackId, cueId, speaker }` | Updates optional speaker voice tag |
| `SetCueTiming` | `{ trackId, cueId, startUs, endUs }` | $0 \le start < end$; re-sorts track cues |
| `MoveCue` | `{ trackId, cueId, deltaUs }` | $start + \Delta \ge 0$; overflow checks |
| `ResizeCueStart` | `{ trackId, cueId, newStartUs }` | $0 \le newStart < end$ |
| `ResizeCueEnd` | `{ trackId, cueId, newEndUs }` | $newEnd > start$ |
| `SplitCue` | `{ trackId, cueId, splitTimeUs }` | $start < splitTime < end$; splits text at word/newline |
| `MergeCues` | `{ trackId, firstCueId, secondCueId }` | Extends time span; deterministic text concatenation |
| `ShiftSelectedCues` | `{ trackId, cueIds, deltaUs }` | **Atomic:** Rejects batch if ANY cue becomes negative |
| `DuplicateCue` | `{ trackId, cueId }` | Clones cue immediately adjacent with new UUID |
| `UpdateTrackMetadata` | `{ trackId, language?, displayName? }` | Updates language tag and display name |

---

## 4. Snapping Engine Architecture

The snapping engine dynamically guides drag and resize operations to prevent fractional misalignments:

### Snap Targets
1. **Current Playhead:** Snaps boundary directly to $playhead\_us$.
2. **Neighboring Cue Start Times:** Aligns start of current cue with start of neighboring cues.
3. **Neighboring Cue End Times:** Snaps start or end of current cue flush against adjacent cue end times (gapless transitions).
4. **Timeline Grid Intervals:** Snaps to ruler tick intervals based on zoom level.

### Threshold Calculation
Snap threshold is defined in display pixels (default: $8\text{px}$ to $10\text{px}$) and dynamically converted to microsecond threshold:

$$\text{Threshold}_{us} = \frac{\text{SnapThreshold}_{px}}{W_{px}} \times (T_{end} - T_{start})$$

When snapping occurs, an amber indicator vertical guide line is rendered with a high-contrast badge indicating target type.

---

## 5. Multi-Level Undo/Redo & Crash Recovery

### History Architecture
- **History Depth:** Bounded to 50 document snapshots (`MAX_UNDO_STACK_SIZE`).
- **Transaction Boundaries:** User interactions (e.g. pointer release after drag, explicit text commit on blur, split, merge, batch nudge) form discrete atomic history states.
- **Redo Invalidation:** Any new edit action clears the redo stack.
- **Text Input Protection:** Global shortcut listeners check `event.target`. When typing inside `<textarea>` or `<input>`, the browser's native text editing undo stack handles character edits without conflicting with document-level timeline state.

### Crash Recovery & Autosave Snapshots
- Local recovery snapshots are serialized to local storage under key `omw_subtitle_autosave_{assetId}`.
- Whenever an operation mutates the document, the recovery snapshot is saved with a timestamp and dirty flag.
- On launch or asset selection, if a recovery snapshot exists that is newer than the persisted project file, the user is presented with a non-intrusive recovery banner:
  - `Restore`: Instantly recovers the unsaved session into memory.
  - `Dismiss`: Clears the recovery snapshot.
- A clean manual project save (`.omwsub`) clears the autosave snapshot automatically.

---

## 6. Video Preview & Overlapping Subtitle Rendering

- **Multiple Overlapping Cues:** In standard film and broadcast subtitling, dual speakers frequently overlap for several hundred milliseconds. Rather than discarding or hiding overlapping cues, the partition point lookup algorithm returns all concurrently active cues.
- **Stacking Overlay:** `SubtitleOverlay` renders all concurrently active cues stacked vertically with semi-transparent dark backgrounds and text shadows.
- **Zero XSS:** Subtitle text is rendered strictly as plain React children (no `dangerouslySetInnerHTML`).
