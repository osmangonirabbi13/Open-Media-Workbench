# OpenTake — Export Engine & Pipeline Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 18, STEP 19, STEP 20 & STEP 21  
**Author:** OpenTake Core Architecture & Media Pipeline Lead  
**Date:** 2026-10-13  

---

## 1. Export Pipeline Overview & 14-Stage Execution Flow

Rendering an edited project into a finalized MP4 video executes across a deterministic 14-stage pipeline:

```
[1. Project Validation] ──────── Check EDL non-empty; valid retained ranges
         │
         ▼
[2. Source Availability] ─────── Confirm raw media exists and matches fingerprint
         │
         ▼
[3. Destination & Disk Check] ── Pre-flight check: Target disk free space > Size + 500 MB
         │
         ▼
[4. Preset Configuration] ────── 1080p, 720p, or Match Source; select CRF quality preset
         │
         ▼
[5. EDL Normalization] ───────── Snap cut boundaries to frame and audio sample boundaries
         │
         ▼
[6. Filtergraph Compilation] ─── Build trim, atrim, setpts, asetpts, concat script
         │
         ▼
[7. Child Process Spawning] ──── Launch FFmpeg with piped stderr progress monitor
         │
         ▼
[8. Video Trimming & Normalization] Enforce CFR timestamps: setpts=PTS-STARTPTS
         │
         ▼
[9. Audio Resampling & Splicing] ── Resample to 48 kHz stereo; asetpts=PTS-STARTPTS
         │
         ▼
[10. Concat Execution] ───────── Splice N video and audio streams seamlessly
         │
         ▼
[11. Encoding & Muxing] ──────── libx264 (YUV420p) + AAC into `.tmp` container
         │
         ▼
[12. Output Verification] ────── FFprobe output file: check duration matches EDL ±1 frame
         │
         ▼
[13. Atomic File Swap] ───────── Atomic rename: `<destination>.tmp` -> `<destination>`
         │
         ▼
[14. Scratch Directory Cleanup] ─ Purge temporary filtergraph files and caches
```

---

## 2. FFmpeg Filtergraph Compilation

For an edit with $N$ retained ranges, compiling the EDL into an FFmpeg complex filtergraph guarantees sample-accurate splicing without cumulative drift:

```rust
// Example Filtergraph Generation for 2 Retained Ranges:
// Range 0: [1.250000s, 8.400000s]
// Range 1: [11.100000s, 19.850000s]

"[0:v]trim=start=1.250000:end=8.400000,setpts=PTS-STARTPTS[v0];\
 [0:a]atrim=start=1.250000:end=8.400000,asetpts=PTS-STARTPTS[a0];\
 [0:v]trim=start=11.100000:end=19.850000,setpts=PTS-STARTPTS[v1];\
 [0:a]atrim=start=11.100000:end=19.850000,asetpts=PTS-STARTPTS[a1];\
 [v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"
```

### Critical Stream Properties Normalization:
1. **Timestamp Reset (`setpts=PTS-STARTPTS`):** Each segment must reset its Presentation Timestamp (PTS) to 0 before feeding into `concat`. Skipping this causes FFmpeg to freeze or drop frames.
2. **Audio Resampling:** Audio is normalized to standard $48{,}000\text{ Hz}$ stereo AAC via `aresample=48000,aformat=channel_layouts=stereo` to prevent concat packet mismatches.
3. **CFR Enforcement:** For VFR source inputs, the video stream passes through the `fps` filter (`fps=fps=30` or native average rate) to eliminate variable timing jitter.

---

## 3. Audio/Video Synchronization & Quality Gates

### 3.1 Provisional Drift Quality Gate: $\pm 25\text{ ms}$
- **What is Measured:** The time delta between the audio event onset (e.g., $1\text{ kHz}$ test tone burst) and the video visual event (e.g., white flash frame) across cut splice points.
- **Reference Standard:** Ground-truth synthetic calibration media (`av_sync_test.mp4`).
- **Target Tolerance:** Less than $\pm 25\text{ ms}$ drift across a 30-minute render (less than 1 frame at 30 fps).
- **Provisional Status:** Marked **PROVISIONAL PENDING EXPERIMENTAL CALIBRATION** in STEP 21. Real-world validation against host hardware will establish the verified baseline.

---

## 4. Export Safety & Atomic Delivery

1. **Temporary Output Collision Prevention:** Video renders strictly to `<output_path>.opentake.tmp`. Existing files are never overwritten until the render passes full verification.
2. **Pre-Flight Disk Space Validation:**
   $$\text{Available Disk Space} \ge \text{Estimated Output Size} + 500\text{ MB}$$
   If disk space is insufficient, export is halted before spawning FFmpeg.
3. **Post-Render Verification:** Once FFmpeg exits with code 0, Rust executes FFprobe on the `.tmp` file:
   - Validates that duration matches $\sum \text{Range Duration} \pm 1\text{ video frame}$.
   - Verifies that video and audio streams exist and contain valid packets.
4. **Atomic Rename Semantics:**
   - On Unix/Linux: `std::fs::rename()` is atomic across the same filesystem mount point.
   - Cross-device fallback: If source and target are across different mount points (e.g., `/tmp` to an external USB drive), OpenTake performs a buffered copy followed by `fsync` and source deletion.
5. **Immediate Cancellation Cleanup:** If the user clicks "Cancel" during export, the child FFmpeg process is killed, and `<output_path>.opentake.tmp` is deleted immediately.
