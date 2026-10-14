# Interrupted MP4/MOV Reconstruction Engine Architecture

**Module:** `crates/opentake-core/src/repair/reconstruction.rs`, `reference.rs`, `reconstruction_report.rs`  
**Document Version:** 1.0.0 (STEP 14)  
**Status:** Implemented & Verified  
**Date:** 2026-10-14  

---

## 1. Executive Summary & Problem Definition

In modern media workflows, unfinalized or interrupted MP4 and MOV files represent one of the most common catastrophic failure modes. When recording is abruptly interrupted (battery death, power loss, application crash, storage disconnect, or operating system panic):

1. The device writes media data (encoded video and audio packets) into an `mdat` (Media Data) box.
2. The recording process terminates **before** writing or finalizing the `moov` (Movie) box.
3. In ISO BMFF (`ISO/IEC 14496-12`) and QuickTime containers, the `moov` box contains the sample tables (`stbl`), sample descriptions (`stsd`), chunk offset tables (`stco`/`co64`), sample size tables (`stsz`), time-to-sample tables (`stts`), and sync sample tables (`stss`).
4. Without the `moov` box, standard demuxers (including FFmpeg, VLC, QuickTime, and browsers) report `moov atom not found` and cannot locate, parse, or decode any packets from the `mdat` payload.

**Reference-Based Reconstruction** solves this by pairing:
- The **damaged recording** (which possesses the media payload in `mdat` but lacks `moov`), and
- A **healthy reference recording** from the same recording device and profile (which possesses matching track definitions, codec extradata, SPS/PPS parameter sets, timescale, and audio sample configurations).

The reconstruction engine extracts codec parameters and track headers from the reference, scans and extracts elementary samples from the damaged payload, computes exact sample sizes, chunk offsets, and sync-frame tables, synthesizes a valid `moov` atom, writes a reconstructed container, and validates the output through bounded decoding.

---

## 2. Engine Candidates & Licensing Audit

### 2.1 Upstream Candidate Evaluation

| Candidate Engine | Source / Upstream | Stated License | Compatibility with GPL-3.0-or-later | Integration Feasibility & Risks |
| :--- | :--- | :--- | :--- | :--- |
| **Native Rust Core Reconstructor** | Built-in (`opentake-core`) | `GPL-3.0-or-later` | **100% Native & Invariant** | **Chosen Primary Engine.** Safe Rust, zero external binary dependency, zero cross-compilation friction, cross-platform (Linux/macOS/Windows), memory-safe, deterministic test fixtures. |
| **Untrunc (anthwlock fork)** | `github.com/anthwlock/untrunc` | GPL-2.0-or-later | **Compatible** (can be combined under GPL-3.0+) | **Secondary Adapter.** Requires external C++ compilation with libavformat/libavcodec development headers. Almost never present by default on user desktops. Subprocess adapter provided when found on `$PATH`. |
| **Untrunc (ponchio original)** | `github.com/ponchio/untrunc` | GPL-2.0-or-later | **Compatible** | Historical upstream. Same C++ libav dependencies as above. Subprocess adapter compatible. |
| **FFmpeg concat / raw demux** | FFmpeg CLI | GPL-2.0+ / GPL-3.0+ | **Compatible** | Useful for post-reconstruction normalization and bounded decode validation, but FFmpeg CLI alone cannot synthesize missing `moov` from scratch without an index. |

### 2.2 Licensing Conclusions & Distribution Policy

1. **License Compatibility:**  
   Both our codebase (`GPL-3.0-or-later`) and `untrunc` (`GPL-2.0-or-later`) permit distribution of composite or integrated works under GPL-3.0.
2. **Process Boundary Isolation:**  
   When invoking external binaries (`untrunc` or `ffmpeg`), execution occurs strictly over an OS subprocess boundary (`tokio::process::Command` without shell execution).
3. **Zero Untrusted Binaries:**  
   Open Media Workbench **does NOT bundle precompiled third-party binaries** from untrusted mirrors.
4. **Primary Native Implementation:**  
   By implementing reference-based ISO BMFF atom reconstruction natively in `opentake-core`, users gain reference-based recovery immediately without installing third-party C++ tooling. When an external `untrunc` binary is detected on the host system or provided in application preferences, the user can select it via the engine adapter.

---

## 3. Architecture & Engine Adapter Boundary

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Repair Studio UI                               │
│      - Select Damaged Asset + Select Healthy Reference (via Native Picker)  │
│      - Compatibility Analysis Panel (Codecs, Dimensions, Audio layout)      │
│      - Reconstruct Button, Progress Tracker, Validation Card, Export        │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ Tauri IPC
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                 Tauri IPC Layer (`src-tauri/src/commands/repair.rs`)        │
│      - MediaAuthorizationRegistry verification of BOTH asset IDs            │
│      - Source != Reference verification & Canonical path checks             │
│      - Job lifecycle management (Queued, Running, Cancelled, Completed)     │
│      - Application staging (`opentake_recovery_staging/<job_id>/`)          │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│          Reconstruction Orchestrator (`repair::reconstruction.rs`)          │
│      - Pre-flight compatibility check (`repair::reference.rs`)              │
│      - Selected Engine Adapter dispatch (`NativeRust` or `UntruncCli`)      │
│      - Staging execution and child process / task cancellation              │
│      - Three-stage output validation (`repair::validation.rs`)              │
│      - Report generation (`ReconstructionReport`)                           │
└───────────────────────┬─────────────────────────────┬───────────────────────┘
                        │                             │
                        ▼                             ▼
       ┌───────────────────────────────┐ ┌───────────────────────────────┐
       │     Native Rust Reconstructor │ │      Untrunc CLI Adapter      │
       │ - Read reference moov/stsd    │ │ - Discovered on $PATH         │
       │ - Scan damaged mdat NALUs     │ │ - Structured args (no shell)  │
       │ - Synthesize stsz, stco, stts │ │ - Process group isolation     │
       │ - Write valid ISO BMFF        │ │ - Output capture to staging   │
       └───────────────────────────────┘ └───────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│             Three-Stage Output Validation (`repair::validation.rs`)          │
│      1. File size check (>0 bytes)                                          │
│      2. Container recognition & FFprobe stream matching                     │
│      3. Bounded null-sink decode test (`ffmpeg -t 8 -f null -`)             │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│              Safe Finalization (`repair::recovery::finalize`)               │
│      - Strict no-clobber check on user destination                          │
│      - Refuse symlink redirection                                           │
│      - Ingest into MediaLibrarySession with NativeFileDialog provenance     │
│      - Clean up temporary staging directory                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Technical Recovery Flow & Atom Reconstruction

### 4.1 Step-by-Step Reconstruction Pipeline

1. **Damaged Structure Inspection:**  
   The damaged file is verified via `scan_iso_bmff_structure`. If `has_mdat == true` and `has_moov == false`, reference-based reconstruction is triggered.
2. **Reference Structure Inspection:**  
   The reference recording is inspected with both `scan_iso_bmff_structure` and `probe_media_file`. It must contain a complete, valid `moov` box with at least one decodable stream.
3. **Compatibility Analysis:**  
   Compare container family, video codec (`h264`, `hevc`), resolution, audio codec (`aac`), sample rate, and channels. If parameters conflict (e.g., H.264 reference vs VP9 or ProRes payload), report incompatibility.
4. **Sample Description Extraction (`stsd`):**  
   Extract the reference video sample entry (e.g. `avc1` containing `avcC` with SPS/PPS) and audio sample entry (e.g. `mp4a` with `esds`).
5. **Damaged Payload Scanning (`mdat`):**  
   Read damaged `mdat` payload. Locate elementary stream boundaries:
   - For H.264/AVC: Locate length-prefixed NAL units or Annex B markers (`0x000001` / `0x00000001`). Detect keyframes (`NAL unit type 5 / IDR`) and inter-frames (`NAL unit type 1`).
   - Extract sample byte offsets and sample sizes.
6. **Sample Table Construction (`stbl`):**  
   Generate:
   - `stsz`: Sample size table with exact byte size of every detected sample.
   - `stco` / `co64`: Chunk offset table with absolute byte positions in output file.
   - `stts`: Time-to-sample table mapping sample duration derived from reference timescale.
   - `stss`: Sync sample table indexing keyframe sample numbers.
   - `stsc`: Sample-to-chunk mapping.
7. **Container Assembly:**  
   Assemble `ftyp` (compatible brands `isom`, `mp42`), synthesized `moov` box with track headers (`tkhd`, `mdia`, `mdhd`, `minf`, `stbl`), followed by `mdat`.
8. **Interleaving & Normalization:**  
   Run a fast stream-copy remux (`ffmpeg -i candidate -c copy`) to ensure optimal chunk interleaving and faststart placement.
9. **Bounded Output Validation:**  
   Execute bounded null-sink decode (`-t 8 -f null -`) to prove the recovered file actually plays.

---

## 5. Security & Safety Invariants

1. **Dual Asset Authorization:**  
   Both `damaged_asset_id` and `reference_asset_id` must be independently registered and verified through `MediaAuthorizationRegistry`. Arbitrary filesystem paths over IPC are strictly prohibited.
2. **Distinct Input Requirement:**  
   The engine strictly verifies `canonical(damaged) != canonical(reference)`. Supplying the damaged file as its own reference is rejected immediately.
3. **Zero Source Mutation:**  
   Both damaged and reference files are opened in read-only mode (`std::fs::File::open`). Pre- and post-recovery SHA-256 hashes must remain 100% byte-identical.
4. **Application-Managed Staging:**  
   Intermediate candidate files are written exclusively to `temp_dir/opentake_recovery_staging/<job_id>/`.
5. **No-Clobber & Symlink Defense:**  
   Finalization enforces atomic destination existence checks and rejects symlinks.
6. **Subprocess Isolation:**  
   External processes (FFmpeg, untrunc) are executed with POSIX process group isolation (`setpgid`), structured arguments (no shell injection), and cooperative cancellation via Tokio cancellation tokens.
7. **Privacy Redaction:**  
   JSON reports exported to users scrub private filesystem paths (`/home/username` -> `~`).

---

## 6. Supported vs Unsupported Scenarios

### Supported Scenarios
- Unfinalized MP4 / MOV recordings from cameras, smartphones, OBS, screen recorders, or drones.
- Files severed mid-recording with genuine `mdat` media payload and missing `moov` atom.
- Truncated files with valid H.264/AVC video and AAC audio tracks.
- Recoveries using a healthy reference video recorded on the same device with identical resolution and codec settings.

### Unsupported Scenarios
- Files where `mdat` is empty, truncated to 0 bytes, or completely filled with zeroes.
- Encrypted or DRM-protected streams (Widevine, FairPlay).
- Proprietary camera metadata formats with non-standard packet encapsulation.
- Codec mismatches between reference and damaged file (e.g. attempting to recover HEVC using an H.264 reference).
- Severely corrupted bitstreams where macroblock headers and slice data have been overwritten with random entropy.

---

## 7. Quality Gates & Verification Matrix

- **Unit & Property Tests:** ISO BMFF atom parsing, `stbl` generation, sample sizing, timebase calculation.
- **Integration Test:** End-to-end synthesis of an unfinalized MP4 missing `moov` with genuine video/audio payload + matching reference -> successful reconstruction -> full decode verification via FFmpeg null-sink.
- **Negative Tests:** Incompatible reference rejection, missing `mdat` rejection, identical file rejection, symlink attack rejection, unauthorized asset ID rejection.
- **Hash Invariance:** Pre- and post-test SHA-256 checks on both damaged and reference files.
