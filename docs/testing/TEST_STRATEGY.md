# OpenTake — Testing Architecture and Verification Strategy

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Verification Baseline Locked  
**Author:** OpenTake QA & Test Engineering Lead  
**Date:** 2026-10-13  

---

## 1. Testing Pyramid & Verification Philosophy

OpenTake employs a multi-tiered testing strategy designed to catch bugs at the lowest possible cost, separating pure mathematical algorithms from heavy end-to-end media integration:

```
               ┌─────────────────────────┐
               │  Desktop E2E Smoke (5%) │  (Manual & Release Qualification)
               ├─────────────────────────┤
               │  Integration Tests (25%)│  (FFprobe, Process Lifecycle, Sync)
               ├─────────────────────────┤
               │  Frontend Component (20%) (Timeline UI, Sliders, Dialogs)
               ├─────────────────────────┤
               │   Unit Tests (50%)      │  (Pure Rust Interval Math, EDL, DSP)
               └─────────────────────────┘
```

---

## 2. Test Suites by Layer

### 2.1 Pure Rust Unit Tests (`crates/opentake-core`)
- **Execution Target:** Fast headless execution via `cargo test -p opentake-core` ($< 5\text{ seconds}$).
- **Interval Algebra & EDL (`timeline::interval_tests`):** Partitioning ranges, boundary clamping, interval coalescing, empty segment purging, 1000-split stress test.
- **Rational Timebase Math (`timeline::timebase_tests`):** Snapping microsecond timestamps to exact frame rates (23.976, 24, 25, 29.97, 59.94 fps) and sample rates (44.1 kHz, 48 kHz).
- **DSP Silence Algorithm (`audio::silence_tests`):** Running sliding-window RMS algorithm on synthetic sine waves and zero-sample silence blocks with known ground-truth intervals.
- **Export Filtergraph Generation (`export::compiler_tests`):** Generating complex filtergraphs for 1, 10, 50, and 200 cuts and asserting exact syntax matching.
- **Project Serialization (`project::schema_tests`):** Round-trip JSON serialization and deserialization, rejection of malformed or invalid schemas.

### 2.2 Integration Tests (`tests/`)
- **FFprobe Ingestion (`tests/probe_integration.rs`):** Parsing real container metadata; verifying duration, codecs, DAR, and stream counts.
- **Child Process Lifecycle & Cancellation (`tests/process_lifecycle.rs`):** Spawning long-running tasks, issuing cancellation tokens, and asserting that the child process tree is reaped within $1{,}500\text{ ms}$ with zero orphan processes.
- **Safe Malformed File Handling (`tests/corrupt_file_handling.rs`):** Passing zero-byte files, truncated MP4 headers, and random binary blobs to `probe_media()`, asserting graceful error returns with zero panics.
- **Export Consistency (`tests/export_verification.rs`):** Rendering edited synthetic fixtures, verifying output file duration via FFprobe against calculated EDL total duration ($\pm 1\text{ video frame}$).
- **Provisional A/V Sync Verification (`tests/av_sync_test.rs`):** Comparing flash/beep calibration markers across 30 cuts, asserting drift stays within $\pm 25\text{ ms}$.

### 2.3 Frontend Component & UI Tests (`src/__tests__/`)
- **Timeline Canvas Scrubbing:** Simulating drag events, verifying correct timecode emissions.
- **Cut Review Drawer:** Verifying individual cut rejection and batch acceptance toggles.
- **Progress Modal:** Verifying smooth rendering of 10 Hz progress updates and cancellation triggers.

---

## 3. Synthetic Media Fixture Specifications

To ensure reproducible testing across different developer machines and CI runners without copyright or redistribution complications, **all test media will be generated programmatically using FFmpeg synthesis scripts** in STEP 21 & STEP 25:

| Fixture Name | Technical Specification | Test Purpose |
|---|---|---|
| `fx_syn_24fps_48k.mp4` | 30s, 1080p, 24.00 fps CFR, 48 kHz stereo tone with burnt-in timecode. | Standard film cadence baseline. |
| `fx_syn_23976fps.mp4` | 30s, 1080p, $24000/1001$ fps NTSC, 48 kHz stereo. | Non-integer frame rate rounding test. |
| `fx_syn_2997fps.mp4` | 30s, 1080p, $30000/1001$ fps NTSC, 48 kHz stereo. | NTSC broadcast framerate baseline. |
| `fx_syn_5994fps.mp4` | 30s, 720p, $60000/1001$ fps NTSC, 48 kHz stereo. | High-frame-rate timestamp test. |
| `fx_syn_vfr.mp4` | 30s, 720p, variable frame rate (simulated screen capture). | VFR detection and CFR export normalization. |
| `fx_syn_silence_cal.mp4`| 60s: alternating 5s speech-level noise (-14 dBFS) and 5s digital silence (-60 dBFS). | Silence detection algorithm sensitivity calibration. |
| `fx_syn_no_audio.mp4` | 15s, 1080p 30 fps, video-only stream (0 audio streams). | Ingestion error handling for media without sound. |
| `fx_syn_corrupt.mp4` | Truncated 100 KB binary blob with invalid `ftyp` header. | Malformed container parser crash immunity. |
| `fx_syn_avsync_flash.mp4`| 60s: audio beep pulse synchronized exactly with a 1-frame visual white flash every 2.0s. | Flash/beep synchronization drift measurement. |

### Fixture Generation Provenance & Safety:
- **Zero Copyright Encumbrance:** 100% generated using FFmpeg synthetic test sources (`testsrc2`, `sine`, `anoisesrc`).
- **Deterministic:** Generated via a checked-in shell script (`scripts/generate_test_fixtures.sh`).
- **No Large Blobs in Git:** Fixtures will be generated on-demand during test runs or cached in temporary build artifacts.
