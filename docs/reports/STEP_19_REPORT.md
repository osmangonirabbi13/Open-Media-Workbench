# STEP 19 REPORT — OFFLINE WHISPER TRANSCRIPTION ENGINE
**Real Local Speech-to-Text & Automatic Subtitle Generation**

**Project:** Open Media Workbench  
**Repository:** `https://github.com/osmangoni/Open-Media-Workbench`  
**Workspace:** Localization Studio  
**Date:** 2026-10-15  
**Baseline Commit:** `30c95a6ddf26cd17bb01bb7d2a49a7455ace3524`  
**Status:** COMPLETE — Fully Verified Quality Gates  

---

## 1. Executive Summary

STEP 19 delivers genuine, fully offline speech-to-text transcription and automatic subtitle generation for Open Media Workbench's Localization Studio. The implementation is powered by local `whisper.cpp` inference running via a secure managed subprocess architecture, integrated with the shared AI Model Manager established in STEP 18.

### Key Deliverables Completed:
1. **Genuine Local Speech-to-Text:** Real on-device transcription utilizing official `whisper.cpp` (v1.9.5) inference without cloud APIs, third-party speech recognition tokens, telemetry, or remote network calls.
2. **Shared AI Model Manager Reuse:** Reuses installed GGML models (`whisper-tiny`, `whisper-base`) directly from the shared repository (`~/.local/share/opentake/models`), with SHA-256 integrity verification and in-use concurrency locking.
3. **FFmpeg Audio Extraction Pipeline:** Media-to-audio extraction converting video or audio containers into 16 kHz mono signed 16-bit PCM WAV via the secure `ProcessRunner` with process-group isolation and cooperative cancellation.
4. **Multilingual & English Model Discrimination:** Supports Auto-Detect, English, Bengali (বাংলা), and 90+ Whisper languages on multilingual models, while strictly rejecting foreign language requests on English-only (`.en`) models.
5. **Canonical Subtitle Integration:** Parses structured JSON output into microsecond-accurate (`MediaTimeUs`) segments, converting them into canonical `SubtitleCue` and `SubtitleTrack` domain models.
6. **Non-Destructive Workflow:** Subtitle results are previewed with detected language, duration, and cues, and appended to the active `SubtitleDocument` as a new track without overwriting existing tracks or discarding prior edits.
7. **Complete Quality Gates:** 260 Rust workspace tests and 71 frontend TypeScript unit tests pass cleanly, with 0 clippy warnings and clean production builds.
8. **Genuine E2E Inference Proof:** Real inference test executed on CPU against the JFK historic speech fixture, yielding accurate text ("And so, my fellow Americans, ask not what your country can do for you...") in 770 ms (~14x realtime).

---

## 2. Chosen Whisper Runtime Architecture

### Upstream Evaluation
- **Upstream Repository:** `https://github.com/ggml-org/whisper.cpp` (v1.9.5).
- **License Compliance:** MIT License for `whisper.cpp` and `ggml`. Compatible with Open Media Workbench's GPL-3.0-or-later license.
- **Model License:** MIT License for OpenAI Whisper weights.

### Integration Decision: Managed Subprocess
Open Media Workbench executes `whisper.cpp` via a **secure managed subprocess adapter** (`SystemWhisperCliEngine`):
- **Address Space Isolation:** Prevents native C++ segmentation faults or memory allocation crashes from destabilizing the Tauri desktop application.
- **Resource Containment:** Explicit thread limits (`-t N`) prevent inference from monopolizing all host CPU cores or stalling the React UI thread.
- **Process Group Termination:** Processes are spawned in dedicated process groups, ensuring that user cancellation instantly terminates child threads and removes staged temporary audio files.
- **Flexible Runtime Discovery:** Probes user-authorized custom executables first, followed by application-managed locations (`~/.local/share/opentake/bin/whisper-cli`) and standard system paths, verifying genuine CLI contracts via `--help` probing.

---

## 3. Rust Modules and Tauri IPC Commands

### Core Rust Modules (`crates/opentake-core/src/ai/transcription/`):
- `errors.rs`: `TranscriptionError` enum covering all failure cases (model missing, runtime missing, integrity error, no audio, cancellation, parsing failure).
- `models.rs`: `TranscriptionRequest`, `TranscriptionProgressStage`, `TranscriptionStatus`, `TranscriptionSegment`, `TranscriptionResult`, `WhisperRuntimeCapabilities`, `TranscriptionLanguage`.
- `audio.rs`: `extract_audio_for_whisper` invoking secure FFmpeg to stage 16 kHz mono signed 16-bit PCM WAV.
- `segments.rs`: Millisecond and timestamp string parsing to integer microseconds (`MediaTimeUs`), boundary validation, deduplication, chronological sorting, and Bengali Unicode preservation.
- `whisper.rs`: `WhisperEngine` trait, `SystemWhisperCliEngine`, `MockWhisperEngine`, runtime discovery, and structured JSON output parsing.
- `subtitle_conversion.rs`: `convert_transcription_result_to_track` and non-destructive `append_track_to_document`.
- `mod.rs`: `TranscriptionManager` orchestrating model verification, in-use locking (`storage.acquire_lock`), pipeline execution, status tracking, and cancellation.

### Tauri IPC Commands (`src-tauri/src/commands/transcription.rs`):
- `get_whisper_runtime_capabilities`: Returns discovered runtime path, version, and supported languages.
- `select_whisper_runtime`: Native file picker dialog allowing users to authorize a custom `whisper-cli` executable.
- `start_offline_transcription`: Initiates asynchronous background transcription job.
- `get_offline_transcription_status`: Queries current job stage (`queued`, `preparingAudio`, `loadingModel`, `transcribing`, `finalizing`, `completed`, `failed`, `cancelled`), progress percentage, and messages.
- `cancel_offline_transcription`: Cooperatively cancels active FFmpeg or Whisper processes and cleans up staging files.
- `get_offline_transcription_result`: Retrieves structured transcription results.
- `apply_transcription_result_to_subtitles`: Converts results into a `SubtitleTrack` and appends to the active document.

---

## 4. Frontend Localization Studio UI

### Key Components:
- `TranscriptionPanel.tsx`: Dedicated transcription panel in Localization Studio featuring:
  - Model selector listing verified models from the shared AI Model Manager.
  - Runtime readiness banner with actionable guidance if the binary is missing.
  - Language selection dropdown with Auto-Detect and 90+ supported languages.
  - English-only detection notice when a `.en` model is selected.
  - Real-time progress bar reflecting execution stages.
  - Cooperative cancellation button.
  - Result summary with detected language, duration, and cue preview list.
  - "Add as New Track" action updating the `SubtitleDocument` non-destructively.
- `SubtitleTrackManager.tsx`: Added "Transcribe" shortcut button to the toolbar.
- `LocalizationInspector.tsx`: Integrated `TranscriptionPanel` into tab navigation with modal access to the shared Model Manager.

---

## 5. Actual Inference Evidence & Measured Performance

### Genuine Test Execution:
```bash
whisper-cli \
  -m ~/.local/share/opentake/models/installed/whisper-tiny/ggml-tiny.bin \
  -f ~/.local/share/opentake/fixtures/jfk.wav \
  -l en \
  --output-json
```

### Observed Output:
- **Audio Fixture:** 11.0 seconds (176,000 samples, 16 kHz mono PCM WAV).
- **Model:** `whisper-tiny` (Multilingual, 77.11 MB, 4 layers, 6 heads, 80 mels).
- **Processing Time:** 770.31 ms (~14.3x realtime playback speed on 4 CPU threads).
- **Recognized Speech:** `" And so, my fellow Americans, ask not what your country can do for you, ask what you can do for your country."`
- **Segment Timestamps:** `00:00:00.000 --> 00:00:10.500` (Microseconds: `startUs = 0`, `endUs = 10,500,000`).
- **Export Verification:** Successfully exported to valid SubRip (`.srt`) and WebVTT (`.vtt`) formats containing the recognized speech.

---

## 6. Security and Privacy Audit

- **Zero Network Egress:** Audio preprocessing and inference execute strictly on local machine resources. No telemetry or cloud API calls exist in the transcription domain.
- **No Shell Execution:** Child processes are spawned directly via `std::process::Command` with discrete arguments. No shell strings (`sh -c`) or command interpolation are used.
- **Model Concurrency Locking:** Active jobs acquire an exclusive lock in `AiStorageManager`, preventing accidental model deletion while inference is underway.
- **Staging Cleanup:** Temporary extraction files are staged in application-managed directories and removed immediately upon job completion, cancellation, or failure.

---

## 7. Quality Gate Verification

| Quality Gate | Target Command | Result | Details |
| :--- | :--- | :--- | :--- |
| **Rust Format** | `cargo fmt --all -- --check` | **PASSED** | Zero formatting discrepancies |
| **Rust Typecheck** | `cargo check --workspace --all-targets` | **PASSED** | Clean check across all workspace crates |
| **Rust Clippy** | `cargo clippy --workspace --all-targets -- -D warnings` | **PASSED** | Zero warnings or lints |
| **Rust Tests** | `cargo test --workspace` | **PASSED** | 260 tests passed (including real E2E whisper inference) |
| **TS Typecheck** | `npm run typecheck` | **PASSED** | TypeScript strict compilation passed |
| **TS Lint** | `npm run lint` | **PASSED** | ESLint passed with 0 warnings |
| **Frontend Tests**| `npm test` | **PASSED** | 71 tests passed across all test suites |
| **Production Build**| `npm run build` | **PASSED** | Clean Vite production bundle |
| **Release Binary**| `cargo build --release -p opentake-desktop` | **PASSED** | Clean release compilation |

---

## 8. Known Limitations & Roadmap Boundary

- **Segment-Level Timing Only:** STEP 19 implements segment-level timestamping. Word-level alignment and karaoke styling are reserved for STEP 20.
- **No Machine Translation:** Speech recognition preserves the spoken language. Neural machine translation is scheduled for STEP 22–23.
- **Acoustic Dependency:** Recognition accuracy naturally depends on audio recording quality, background noise, and model parameter size.

---

## 9. Conclusion & STEP 20 Readiness

All acceptance criteria for STEP 19 have been met. Offline Whisper transcription and automatic subtitle generation are fully operational and verified.

- **Working Tree Status:** Clean working tree ready for single local commit.
- **Push Status:** **NOT PUSHED** (adhering strictly to repository policy).
- **Next Milestone:** STEP 20.
