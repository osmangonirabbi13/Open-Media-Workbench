# OpenTake — Technical Constraints & Architecture Specification

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Author:** OpenTake Core Architecture Team  
**Date:** 2026-10-13  

---

## 1. System Architecture Overview

OpenTake is designed as a hybrid desktop application combining a high-performance **Rust backend core** with a modern, reactive **React/TypeScript frontend**, orchestrated across the **Tauri 2** desktop IPC bridge.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        TAURI 2 DESKTOP SHELL                           │
├────────────────────────────────────────────────────────────────────────┤
│  FRONTEND (React 19 + TypeScript + Vite + Tailwind CSS + Radix UI)     │
│  - Timeline UI & Playhead           - Canvas Audio Waveform Renderer   │
│  - Video Transport Controls         - Interactive Cut Review Drawer    │
│  - Project Settings & Modals        - Responsive Job Progress Displays │
├────────────────────────────────────────────────────────────────────────┤
│             TAURI IPC BRIDGE (Typed Commands & Events)                 │
├────────────────────────────────────────────────────────────────────────┤
│  RUST NATIVE BACKEND (Tokio Async Runtime)                             │
│  - Media Probe & Metadata Engine    - Audio PCM Decoding & DSP Silence │
│  - Edit Decision List (EDL) Model   - Project Serialization (JSON)     │
│  - Export Timeline Compiler         - Concurrency & Job Cancellation   │
│  - File System Scoping & Safety     - FFmpeg/FFprobe Child Processes   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Division of Architectural Responsibilities

### 2.1 Rust Core Responsibilities
The native Rust layer is the **single source of authoritative truth** for all computational, media-processing, and state-persistence operations. Specifically, Rust owns:

1. **Child Process Orchestration:** Spawning, monitoring, streaming stdout/stderr, and killing FFmpeg and FFprobe worker processes using `tokio::process::Command`. Shell execution strings (`sh -c`) are strictly prohibited.
2. **Media Probing & Validation:** Running FFprobe to extract structural container information, stream layouts, codecs, pixel formats, durations, and packet timestamp integrity.
3. **Audio Extraction & DSP Analysis:** Decoding audio streams into raw PCM 16-bit linear samples, executing streaming RMS/peak dBFS silence detection algorithms, and computing multi-resolution waveform buckets.
4. **Authoritative Edit Decision List (EDL):** Maintaining the non-destructive interval data structure representing retained and excluded time ranges. All mutations (splits, trims, deletes, restores) are validated in Rust.
5. **Undo/Redo Command Engine:** Managing the operational history stack to ensure bit-level deterministic state across undo/redo actions.
6. **Project Persistence:** Validating, serializing, and writing versioned `.opentake` project files using atomic temp-file swaps.
7. **Export Plan Compilation:** Translating the EDL into an optimized FFmpeg filtergraph (using `trim`, `atrim`, `setpts`, `asetpts`, and `concat` filter pipelines) or a concat demuxer script.
8. **Asynchronous Job Management:** Managing background tasks with unique UUIDs, throttling progress event dispatch over the IPC bridge (max 10 Hz), and handling instant cancellation via `tokio_util::sync::CancellationToken`.
9. **Filesystem Security:** Canonicalizing paths and preventing path traversal or unauthorized access outside user-selected working media directories.

### 2.2 Frontend (React / TypeScript) Responsibilities
The React layer serves as the **presentation and user interaction engine**. It must remain thin and reactive, deferring all heavy computation to Rust:

1. **Desktop Ergonomics:** Rendering an uncluttered, responsive dark-theme desktop interface using Tailwind CSS and Radix UI primitives.
2. **Video Transport & Scrubbing:** Hosting the HTML5 `<video>` preview element, handling playback state (`play`, `pause`, `seeking`), and synchronizing the video timecode with the timeline playhead.
3. **Canvas Waveform Rendering:** High-performance rendering of multi-resolution audio waveform data using HTML5 2D Canvas or WebGL, avoiding DOM node proliferation.
4. **Timeline Interaction:** Handling mouse drag events for timeline scrubbing, boundary trimming, slice actions, and zoom/scroll viewport scaling.
5. **Interactive Review Flow:** Providing dedicated UI controls for stepping through suggested silence cuts, auditioning edits, and toggling cut acceptance.
6. **UI State & IPC Synchronization:** Dispatching typed Tauri commands (`invoke("split_at", { time_us })`) and listening for asynchronous background events (`export_progress`, `analysis_progress`).
7. **Error & Notification Handling:** Displaying non-blocking toast notifications and diagnostic modals for system errors, missing files, or validation failures.

---

## 3. Non-Destructive Editing & Timebase Architecture

### 3.1 Strict Source Immutability
Raw media files are opened exclusively in read-only mode (`std::fs::File::open`). Under no circumstances does OpenTake open a write stream to the source file, append metadata, or modify file timestamps.

### 3.2 Timebase Representation: Microsecond Precision
> [!IMPORTANT]
> **Anti-Pattern Warning:** Floating-point seconds (e.g., `let t: f64 = 12.345`) must NEVER be used for authoritative timeline coordinates or edit decision cuts. Cumulative floating-point rounding errors across dozens of spliced segments cause severe audio/video drift and desynchronization.

OpenTake enforces an integer timebase:
- **Canonical Unit:** Integer microseconds (`i64`), representing $10^{-6}$ seconds ($1\text{ s} = 1,000,000\,\mu\text{s}$).
- Microseconds provide exact integer representation for standard video frame rates (e.g., $24\text{ fps} \approx 41,667\,\mu\text{s}$, $25\text{ fps} = 40,000\,\mu\text{s}$, $30\text{ fps} \approx 33,333\,\mu\text{s}$, $60\text{ fps} \approx 16,667\,\mu\text{s}$) and audio sample rates (e.g., $48{,}000\text{ Hz} \approx 20.833\,\mu\text{s}$, $44{,}100\text{ Hz} \approx 22.675\,\mu\text{s}$) without rounding drift over multi-hour files.
- Where rational math is required for exact container timebases, Rust's `num_rational::Rational64` is utilized.

### 3.3 Edit Decision List (EDL) Data Structure
The editing state is modeled as an ordered set of contiguous and non-overlapping **Retained Ranges**:

```rust
pub struct RetainedRange {
    pub id: uuid::Uuid,
    pub source_start_us: i64,
    pub source_end_us: i64,
}

pub struct EditDecisionList {
    pub source_file_path: std::path::PathBuf,
    pub source_duration_us: i64,
    pub retained_ranges: Vec<RetainedRange>,
}
```

- **Excluded Regions (Cuts):** Defined implicitly as the gaps between successive `RetainedRange` entries:
  $$\text{Cut}_k = [\text{Range}_k.\text{source\_end\_us},\, \text{Range}_{k+1}.\text{source\_start\_us}]$$
- **Total Output Duration:** $\sum_{k} (\text{Range}_k.\text{source\_end\_us} - \text{Range}_k.\text{source\_start\_us})$.
- **Source-to-Output Time Mapping:** A deterministic binary-search lookup maps any source timestamp to its corresponding output timecode, and vice versa.

### 3.4 Handling Variable Frame Rate (VFR) & Audio Sample Boundaries
Smartphones, screen recorders (OBS, Zoom), and webcams frequently record in Variable Frame Rate (VFR). Splicing VFR media via simple frame counts results in progressive A/V desynchronization:
1. **FFprobe Detection:** The probe engine analyzes packet presentation timestamps (PTS) to detect VFR streams (`r_frame_rate != avg_frame_rate`).
2. **Audio Alignment:** Silence cut boundaries are snapped to the nearest audio sample boundary ($1/48000\text{ s} \approx 20.83\,\mu\text{s}$) to avoid popping/clicking artifacts.
3. **Video PTS Normalization:** During export compilation, all retained segments pass through the `fps` filter or have PTS normalized via `setpts=N/(FPS*TB)` to enforce Constant Frame Rate (CFR) in the output MP4.

---

## 4. Playback Engine Architecture

### 4.1 Evaluation of Playback Strategies

| Strategy | Feasibility in Tauri 2 | Codec Compatibility | Seeking Latency | Memory / CPU | Recommendation for v0.1.0 |
|---|---|---|---|---|---|
| **Direct WebView `<video>`** | High (native browser element) | Platform-dependent (WebKitGTK on Linux relies on GStreamer; WebView2 on Windows uses Media Foundation; WKWebView on macOS uses AVFoundation). | Very fast (< 50 ms) for native H.264/AAC. | Low CPU, zero proxy overhead. | **PRIMARY PREVIEW PIPELINE** for validated MP4/MOV. |
| **Local Custom Streaming Bridge (Tauri Asset Protocol)** | High (custom `tauri://` or `asset://` URI scheme) | Bypasses CORS and filesystem restrictions; streams byte ranges with `Accept-Ranges` support. | Fast seeking via HTTP 206 Partial Content. | Bounded memory buffer. | **REQUIRED INFRASTRUCTURE** for local file access. |
| **Fast Preview Proxy** | High (transcode to lightweight 720p H.264 ultrafast) | 100% universal compatibility across all platforms. | Instant scrubbing once generated. | Temporary disk space required (~200 MB). | **AUTOMATIC FALLBACK** for unplayable codecs (e.g., MKV with Opus or HEVC on older Linux). |
| **Embedded Custom Video Renderer (WGPU / MPV)** | Low (requires custom native window embedding, complex multi-platform event loops) | Universal. | Excellent. | High architectural complexity. | **REJECTED for v0.1.0** (violates 30-day timeline). |

### 4.2 Playback Strategy for v0.1.0 Beta
1. **Primary Route:** Video preview utilizes the HTML5 `<video>` element served via Tauri's secure asset streaming protocol. This ensures hardware-accelerated playback on Windows (WebView2), macOS (WKWebView), and modern Linux (WebKitGTK + GStreamer).
2. **Seamless Cut Auditioning:** During timeline playback with "Skip Silence" enabled, the frontend timeline engine monitors the playhead timestamp. When the playhead reaches `source_end_us` of a retained segment, it immediately seeks the `<video>` element to `source_start_us` of the next retained segment.
3. **Proxy Fallback Trigger:** If the native WebView fails to play or decode a media stream (detected via `<video>` error events or pre-probe inspection), OpenTake prompts the user to generate a lightweight preview proxy.

---

## 5. FFmpeg Processing Pipeline

```
  [Raw Media Input]
         │
         ▼
[Stage 1: Input Validation] ──────── Check path, header magic bytes, access permissions
         │
         ▼
[Stage 2: FFprobe Metadata] ──────── Extract duration, streams, codecs, CFR/VFR, PTS
         │
         ▼
[Stage 3: Audio Extraction & DSP] ── Decode PCM s16le, compute RMS/peak dBFS, find silence
         │
         ▼
[Stage 4: Timeline Compilation] ──── Compile retained ranges to FFmpeg filtergraph
         │
         ▼
[Stage 5: Cut Segment Splicing] ──── Trim video & audio streams with microsecond PTS bounds
         │
         ▼
[Stage 6: PTS Normalization] ─────── Apply setpts=PTS-STARTPTS and asetpts=PTS-STARTPTS
         │
         ▼
[Stage 7: Export Encoding] ───────── Encode via libx264 (CRF preset) + aac, mux into MP4
         │
         ▼
[Stage 8: Output Verification] ───── Run FFprobe on export file; verify duration & streams
         │
         ▼
[Stage 9: Atomic File Swap] ──────── Atomic rename: export.mp4.tmp -> export.mp4
         │
         ▼
[Stage 10: Cleanup] ──────────────── Purge temporary PCM caches and intermediate files
```

### 5.1 Why Re-Encoding is Mandatory (Stream Copy Limitations)
> [!IMPORTANT]
> **Stream Copying (`-c copy`) Pitfall:** Video streams compressed with modern temporal inter-frame codecs (H.264, H.265) organize frames into Groups of Pictures (GOPs), consisting of I-frames (keyframes), P-frames, and B-frames. Keyframes typically occur only once every 1–5 seconds.
> - Fast stream-copy cuts can **only cut cleanly on I-frames**.
> - If an edit cut occurs between two keyframes, stream copy produces severe decoding corruption, gray frames, frozen video, or broken audio sync at every cut point.
> - Therefore, **OpenTake v0.1.0 Beta mandates re-encoding during final export** via `libx264` to achieve 100% frame-accurate cut boundaries. Smart-rendering / cut-boundary-only re-encoding is deferred to P1.

### 5.2 Filtergraph Pipeline Structure
For an edit with $N$ retained ranges, OpenTake constructs an FFmpeg complex filtergraph:
```text
[0:v]trim=start=1.250:end=8.400,setpts=PTS-STARTPTS[v0];
[0:a]atrim=start=1.250:end=8.400,asetpts=PTS-STARTPTS[a0];
[0:v]trim=start=11.100:end=19.850,setpts=PTS-STARTPTS[v1];
[0:a]atrim=start=11.100:end=19.850,asetpts=PTS-STARTPTS[a1];
[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]
```
This filtergraph guarantees exact audio and video alignment across all splice joints.

---

## 6. Background Processing & Concurrency

### 6.1 Concurrency Model
- **Runtime:** Tokio multi-threaded asynchronous runtime in the Rust backend.
- **Worker Pools:** Intensive CPU DSP tasks (e.g., PCM audio silence calculations) are executed on Tokio's blocking threadpool via `tokio::task::spawn_blocking` to avoid starving the async reactor.
- **Process Spawning:** FFmpeg/FFprobe processes are spawned asynchronously using `tokio::process::Command`. Child `stdout` and `stderr` streams are piped and consumed using buffered readers (`tokio::io::BufReader`).

### 6.2 Cancellation Protocol
1. Each long-running background job (silence analysis, waveform generation, export encoding) is instantiated with a `tokio_util::sync::CancellationToken` and a unique `job_id: uuid::Uuid`.
2. When the user clicks "Cancel" in the UI, a Tauri command `cancel_job(job_id)` is invoked.
3. The Rust backend fires the cancellation token.
4. The background task catches the cancellation signal:
   - Immediately sends `SIGINT` (or `SIGTERM` / `kill` on Windows) to the child FFmpeg process.
   - Waits up to 1,500 ms for graceful process exit; if un-reaped, issues a hard kill.
   - Automatically unlinks and removes any partial output file (`*.tmp`).
   - Dispatches a `job_cancelled` event to the frontend.

### 6.3 Memory & Resource Safeguards
- **Streaming Audio DSP:** Audio PCM extraction does not load entire 3-hour audio files into a contiguous RAM buffer. Instead, audio is read from FFmpeg's stdout pipe in 64 KB chunks, analyzed incrementally for silence, and downsampled into waveform peak buckets.
- **Waveform Bucket Compression:** Peak/RMS values are stored in 16-bit integers (2 bytes per bucket: min and max), consuming less than 1.5 MB of RAM per hour of audio.

---

## 7. Media Support Policy (v0.1.0 Beta)

### 7.1 Container & Codec Compatibility Matrix

| Container | Video Codecs Supported | Audio Codecs Supported | Import Status (v0.1.0) | Export Status (v0.1.0) |
|---|---|---|---|---|
| **MP4 (`.mp4`)** | H.264 (AVC) | AAC | **Primary Reference (Fully Supported)** | **Default Output Target** |
| **MP4 (`.mp4`)** | H.265 (HEVC) | AAC, MP3 | Supported if hardware/decoder available | Re-encoded to H.264 / AAC |
| **MOV (`.mov`)** | H.264, Apple ProRes | AAC, PCM (`pcm_s16le`) | Supported for validated combinations | Re-encoded to H.264 / AAC |
| **MKV (`.mkv`)** | H.264, H.265 | AAC, Opus, Vorbis | Supported (may require proxy for preview) | Re-encoded to H.264 / AAC |
| **WebM (`.webm`)** | VP8, VP9, AV1 | Opus, Vorbis | Post-Beta (P1) | Not supported in v0.1.0 |
| **AVI / WMV** | Legacy codecs | Legacy codecs | Rejected (Unsupported format alert) | Not supported in v0.1.0 |

### 7.2 Input Media Edge-Case Policies
1. **Video Without Audio:** If FFprobe detects zero audio streams, the video is ingested successfully, but silence analysis is disabled with an informative UI banner. Manual trimming and cutting remain fully functional.
2. **Audio-Only Files (`.mp3`, `.wav`, `.m4a`):** For v0.1.0 Beta, OpenTake is strictly a video editor. Audio-only ingestion is deferred to P1. If an audio file is dropped, the app displays: *"OpenTake v0.1.0 Beta is designed for video editing. Audio-only file support will arrive in an upcoming update."*
3. **Multiple Audio Tracks:** If the container contains multiple audio streams (e.g., separate microphone and game audio in OBS recordings), OpenTake v0.1.0 defaults to analyzing and playing stream index 0 (the primary audio track). Multi-track selection is scheduled for P1.
4. **Corrupted File Handling:** If FFprobe returns exit code $\ne 0$ or cannot parse container headers within 5 seconds, ingestion is safely aborted with an error dialog; no crash occurs.
5. **Resolution Limits:** Supports up to 4K UHD ($3840 \times 2160$) at 60 fps. 8K files trigger a high-resolution performance warning recommending downscaling.

---

## 8. Offline & Privacy Architecture

- **Air-Gapped Operation:** OpenTake contains zero mandatory network network calls. The application operates with 100% feature completeness in an isolated, air-gapped environment.
- **Zero Telemetry by Default:** No analytics pings, crash telemetry daemons, or user tracking packages are bundled.
- **Local File Scoping:** All filesystem reads and writes are restricted to explicit user selections via native file pickers or designated project cache directories (`~/.cache/opentake` or platform equivalent).
- **Future AI Model Policy (Post-Beta):** When local transcription (whisper.cpp) is introduced in P1, model weights (GGML format) must never be silently downloaded. The user must be explicitly prompted with file size, destination, and license disclosure before initiating an optional download.
