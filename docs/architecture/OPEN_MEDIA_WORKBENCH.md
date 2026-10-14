# Open Media Workbench — Unified Desktop Architecture

**Product Name:** Open Media Workbench  
**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Baseline Commit:** `663331b8c5fc0dcb5326a1bf8517125afb9ab8a1`  
**Date:** 2026-10-14  

---

## 1. Vision & Architecture Paradigm

Open Media Workbench is a high-performance, offline-first desktop application engineered as:
> **ONE native desktop application hosting TWO specialized, independent studios:**
> 1. **Repair Studio:** Video corruption diagnosis, MP4/MOV recovery, elementary stream salvage, and repair validation.
> 2. **Localization Studio:** Subtitle editing, local AI speech-to-text, translation, neural speech dubbing, and export.

Rather than fragmenting into multiple disparate tools, both studios operate on top of a single, battle-tested, headless **Rust Media Core** (`opentake-core`) wrapped in a security-hardened **Tauri 2** desktop container with a reactive **React 19 + TypeScript** user interface.

```
┌────────────────────────────────────────────────────────────────────────┐
│               Open Media Workbench Shell (React 19)                    │
│  ┌──────────────────────────────┐    ┌──────────────────────────────┐  │
│  │     🛠️ Repair Studio         │    │   🌐 Localization Studio     │  │
│  │ (Diagnosis, Salvage, Fixes)  │    │ (Subs, ASR, Dubbing, Export) │  │
│  └──────────────┬───────────────┘    └──────────────┬───────────────┘  │
│                 │                                   │                  │
│                 └─────────────────┬─────────────────┘                  │
│                                   │                                    │
│       Shared UI Services: VideoPreview, WaveformCanvas, Timeline       │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Tauri 2 IPC & Custom URI Protocol
┌───────────────────────────────────┴────────────────────────────────────┐
│                    Tauri 2 Native Desktop Layer                        │
│  ├── Window Management & Menus                                         │
│  ├── Custom Streaming Protocol (opentake-media://)                     │
│  ├── File Authorization & Prov-Tracking                                │
│  └── Background Task Cancellation & Event Dispatch                     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Native Crate Dependency
┌───────────────────────────────────┴────────────────────────────────────┐
│                    Rust Media Core (opentake-core)                     │
│  ├── media::process      (Managed, secure FFmpeg/FFprobe sub-processes)│
│  ├── media::range        (Streaming byte-range HTTP protocol provider) │
│  ├── media::waveform     (Multi-LOD audio peak/RMS analysis & cache)   │
│  ├── media::proxy        (Low-latency 720p H.264 proxy generator)      │
│  └── editing::*          (Canonical integer microsecond EDL engine)    │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Shared Rust Media Core & Completed Infrastructure (Steps 1–9)

The foundational work completed across STEPS 1 through 9 remains completely preserved and serves as the common substrate for both studios:

1. **Process Management Infrastructure (`media::process`)**:
   - Secure execution of FFmpeg and FFprobe without shell interpolation.
   - POSIX process group isolation (`setpgid`) allowing deterministic, leak-free termination of runaway transcoding jobs.
   - Memory-bounded stderr parsing and output scrubbing.
2. **Media Inspection & Metadata Probing (`media::metadata`)**:
   - Extraction of video/audio container attributes using rational arithmetic (`Rational { num, den }`).
   - Strict timebase conversions avoiding floating-point inaccuracies.
3. **Secure Custom URI Media Transport (`media::range`)**:
   - Custom desktop protocol handler registered with WebKitGTK and webview.
   - High-performance HTTP 206 Partial Content byte-range serving directly from disk with sub-millisecond seek response.
4. **Proxy Transcoding Pipeline (`media::proxy`)**:
   - Content-addressed Blake3 cache keys for high-bitrate or non-browser-native codecs (HEVC, ProRes, AV1).
   - Fast-start MP4 (`+faststart`) generation with local cache invalidation.
5. **Multi-LOD Audio Waveform Extraction (`media::waveform`)**:
   - Background audio decimation to 16 kHz mono/stereo PCM.
   - Multi-resolution hierarchical peak and RMS cache files with fast memory-mapped chunk loading.
6. **Canonical Non-Destructive EDL Model (`editing::*`)**:
   - Integer microsecond representation (`MediaTimeUs`, `SourceRange`).
   - Immutable timeline manipulation primitives (`cut_source_range`, `restore_source_range`, `keep_only`).
   - $O(\log N)$ coordinate transformation index (`TimelineIndex`) between source time and timeline playhead.

---

## 3. Studio Boundaries & Functional Allocation

### 3.1 Repair Studio (Planned Workspace 1)
**Objective:** Ingest broken, incomplete, or corrupted video files and recover playable media streams without destructive writes.

- **Ingestion & Diagnostics Boundary:**
  - Tolerant parser capable of reading partial containers (e.g., MP4/MOV files missing the `moov` atom header due to recording power loss or crash).
  - Diagnostic report generator identifying missing atoms, unclosed sample tables (`stbl`), damaged chunk offsets (`stco`/`co64`), or non-conforming bitstreams.
- **Salvage Engine:**
  - Reference-file reconstruction: extracts codec configuration records (`avcC`, `mp4a`) from a valid reference clip recorded with identical settings and synthesizes a reconstructed `moov` atom.
  - Raw packet demuxer: recovers orphaned H.264/H.265 NAL units and AAC ADTS frames directly from `mdat` chunks.
  - Container re-indexing: wraps salvaged elementary streams into compliant MP4 or MKV containers.
- **Validation & Split Preview:**
  - Leverages the shared custom-protocol player to present side-by-side A/B comparison of damaged vs salvaged media before export confirmation.

### 3.2 Localization Studio (Planned Workspace 2)
**Objective:** End-to-end subtitle generation, speech recognition, machine translation, and neural speech dubbing.

- **Subtitle Editor & Timebase Boundary:**
  - Subtitle cues modeled as microsecond intervals (`SourceRange`), perfectly synchronized with the canonical EDL engine.
  - Interactive cue manipulation: splitting, merging, retiming, and text editing with real-time waveform visualization.
- **Local AI Transcription (Speech-to-Text):**
  - Fully offline, local Whisper inference (e.g., via `whisper.cpp` / ONNX runtime).
  - Emits word-level and phrase-level timestamped tokens directly into the subtitle cue list.
  - Zero external network requests or third-party cloud dependencies.
- **Local Machine Translation:**
  - On-device translation pipeline mapping source subtitle text to target language scripts.
- **AI Voice Dubbing (Speech Synthesis):**
  - Offline neural text-to-speech (TTS) engine generating synthesized speech clips for each subtitle cue.
  - Automatic time-stretching or rate-scaling to synchronize dubbed audio within subtitle cue interval boundaries.
- **Multiplexed Export:**
  - Uses the shared FFmpeg managed process engine to mux subtitle tracks (SRT/VTT/embedded) and synchronized dubbing audio into the output video.

---

## 4. Shared Player, Timeline & Background Job Services

To prevent code duplication, the two workspaces share core services:

```
┌─────────────────────────────────────────────────────────────┐
│                      Shared Services                        │
├──────────────────────────────┬──────────────────────────────┤
│ VideoPlaybackService         │ WaveformService              │
│ - opentake-media:// streaming│ - Multi-LOD peak extraction  │
│ - HTML5 video event sync     │ - Canvas rendering sync      │
│ - Frame-accurate stepping    │ - Playhead tracking          │
├──────────────────────────────┼──────────────────────────────┤
│ BackgroundJobManager         │ AuthorizationSession         │
│ - Async Tokio state machine  │ - Cryptographic path access  │
│ - PGID process tree killing  │ - Explicit file provenance   │
│ - Progress & ETA estimation  │ - Sandboxed media boundaries │
└──────────────────────────────┴──────────────────────────────┘
```

---

## 5. Data Ownership and Persistence Strategy

1. **Non-Destructive Guarantee:**
   - Input files (damaged media in Repair Studio, original footage in Localization Studio) are treated as **strictly read-only**.
   - All repairs, salvaged streams, synthesized audio, and exports are written to explicit user-designated target directories or sandboxed application scratch paths.
2. **Project Data Format (`.omw` / `.opentake`):**
   - Project definitions are serialized as human-readable, schema-versioned JSON.
   - Atomic disk writes (`fsync` to temporary file before atomic renaming) ensure resilience against sudden power loss.
3. **Workspace Preferences:**
   - Persisted in client `localStorage` with validated fallbacks, schema versioning, and layout reset capabilities.

---

## 6. Workspace Isolation and Dependency Boundaries

To maintain long-term stability and clean separation of concerns:
- **Rust Core Independence:** `crates/opentake-core` contains zero UI logic and zero Tauri dependencies. It is compileable as a pure headless crate and thoroughly testable via standard `cargo test`.
- **UI Decoupling:** Repair Studio and Localization Studio viewports reside in separate directory trees (`src/workspaces/repair/` and `src/workspaces/localization/`), sharing common components from `src/components/ui/` and `src/components/layout/`.
- **Hardware Acceleration:** Future hardware-accelerated decode/encode pipelines interface through clean trait abstractions in `opentake-core::media`.

---

## 7. Deferred Internal Identifier Migration Rationale

During this STEP 10 migration, several internal identifiers have been intentionally preserved:

| Identifier | Current Value | Preservation Rationale |
| :--- | :--- | :--- |
| **Tauri Bundle Identifier** | `org.opentake.app` | Changing bundle ID immediately changes OS app data paths (`~/.local/share/org.opentake.app`), invalidating existing cached waveforms and user preferences. |
| **Custom URI Protocol** | `opentake-media://` | Protocol scheme is registered in WebKitGTK and referenced in CSP rules; migrating must occur in synchronized lockstep with frontend media URLs. |
| **Core Crate Name** | `opentake-core` | Renaming crate generates churn across Cargo.lock and test imports; deferred until feature modules are established. |
| **Desktop Crate Name** | `opentake_desktop` | Preserves build scripts, Tauri CLI configurations, and binary output targets. |
| **Storage Key** | `opentake_workspace_preferences_v1` | Preserves existing user layout customizations across app launches. |

These identifiers will be scheduled for a seamless migration pass once workspace feature parity is achieved, using an automated preference/cache migrator.

---

## 8. Compatibility & Upgrade Strategy

1. **Deterministic Test Gates:**
   - Every commit must pass all existing Rust unit, property, and integration tests (83 tests) and TypeScript tests (16 tests + 6 workspace tests).
2. **Schema Upgraders:**
   - Any future modifications to saved project structures or cache headers will provide backward-compatible deserializers with automated migration paths.
3. **Graceful Degraded Runtime:**
   - If local AI models or hardware encoders are unavailable, the application gracefully alerts the user while keeping standard media playback and manual editing fully operational.

---

## 9. Workspace Domain Model & Lifecycle (STEP 11)

### 9.1 Typed Workspace Identities
Workspaces are strictly typed as:
```typescript
type WorkspaceId = "repair" | "localization";
```
Each workspace is defined declaratively (`WORKSPACE_DEFINITIONS`) with its label, description, badge, and capability matrix specifying implementation status (`available`, `in_development`, `planned`) and roadmap milestone step.

### 9.2 State Retention & Lifecycle
- **Shared Media Ingestion & Authorization:** Both studios mount the canonical `MediaLibraryPanel` backed by the Rust media session. Assets imported in one studio remain authorized and immediately selectable in the other without duplicating memory buffers or session state.
- **Active Asset Persistence:** When the user switches between Repair Studio and Localization Studio, the active asset selection is preserved across viewports.
- **Independent Studio State:** Transient inspector views, diagnostic states, and recovery workflow step indices are scoped to each workspace. Switching tabs does not destroy in-progress work or reset unrelated panel splitters.
- **Resource Ownership:** Video decoder elements (`<video>`) and audio waveform canvas buffers unmount deterministically, preventing background playback leaks or GPU resource hogging.

### 9.3 Keyboard Navigation & Accessibility
- **W3C ARIA Tablist Pattern:** The workspace switcher in `AppHeader` implements standard `role="tablist"` with `role="tab"`, `aria-selected`, `aria-controls`, and `tabIndex` attributes.
- **Keyboard Traversal:** Users can cycle between studios using `ArrowLeft`/`ArrowRight`/`Home`/`End` within the switcher, or globally using non-conflicting dedicated shortcuts:
  - `Alt+1`: Switch to Repair Studio
  - `Alt+2`: Switch to Localization Studio
- **Input Guard:** Global shortcuts strictly ignore keystrokes when focus is inside an `<input>` or `<textarea>` element.

---

## 10. Functional Capabilities vs Deferred Milestones Matrix

| Capability | Studio | Status | Delivery Milestone |
| :--- | :--- | :--- | :--- |
| **Media Import & Probing** | Shared | **Available** | STEP 6 (Verified) |
| **Video Playback & Proxy Engine** | Shared | **Available** | STEP 7 (Verified) |
| **Multi-LOD Waveform Pipeline** | Shared | **Available** | STEP 8 (Verified) |
| **Canonical Non-Destructive EDL** | Shared | **Available** | STEP 9 (Verified) |
| **Dual-Workspace Shell & Switcher** | Shared | **Available** | STEP 11 (Verified) |
| **Container & Atom Diagnostics** | Repair | *Planned* | STEP 12 |
| **MOOV Atom Reconstruction (Untrunc)** | Repair | *Planned* | STEP 13 |
| **Bitstream Demux & Stream Salvage** | Repair | *Planned* | STEP 14 |
| **A/B Repair Preview Validation** | Repair | *Planned* | STEP 15 |
| **Interactive Subtitle Editor (CRUD)** | Localization | *Planned* | STEP 16–17 |
| **Local Offline Whisper Transcription** | Localization | *Planned* | STEP 18–21 |
| **Local Neural Machine Translation** | Localization | *Planned* | STEP 22–23 |
| **Neural Speech Synthesis (Dubbing)** | Localization | *Planned* | STEP 24–28 |
| **Subtitle & Dubbed Multiplex Export** | Localization | *Planned* | STEP 29–31 |

