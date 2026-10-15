# Offline Whisper Transcription Engine Architecture

**Document Version:** 1.0.0  
**Status:** Canonical Architecture Specification (STEP 19)  
**Date:** 2026-10-15  
**Workspace:** Localization Studio  

---

## 1. Executive Summary & Design Principles

Open Media Workbench introduces **genuine offline speech-to-text transcription and automatic subtitle generation** for Localization Studio.

The system converts spoken dialogue from authorized local video and audio assets into canonical microsecond-accurate subtitle tracks without transmitting media or metadata over the internet.

### Inviolable Principles:
1. **Zero Cloud Dependencies:** No OpenAI API, third-party speech recognition APIs, external cloud accounts, or remote network calls are permitted during transcription.
2. **Zero Fabricated Transcripts:** Inference results must strictly originate from actual Whisper execution on audio waveforms, never canned mocks or simulated responses.
3. **Zero Bundled Model Weights:** Models are not embedded in application installers. Users install models on demand via the shared AI Model Manager (STEP 18) with explicit cryptographic verification.
4. **Zero Automatic Downloads:** No runtime or model file is ever downloaded silently. All downloads and runtime authorizations require explicit user consent.
5. **Non-Destructive Subtitle Workflow:** AI-generated transcription creates a new canonical `SubtitleTrack` in `SubtitleDocument`, never silently overwriting existing human-curated tracks.
6. **Robust Process Isolation:** FFmpeg preprocessing and Whisper inference execute as isolated OS child processes with process-group isolation, explicit non-shell argument vectors, bounded buffers, and immediate cancellation cleanup.

---

## 2. System Architecture & Component Hierarchy

```
                            Localization Studio
                                     │
                        TranscriptionPanel.tsx
                                     │
                       Tauri IPC Security Bridge
                     (src-tauri/commands/transcription.rs)
                                     │
                        Rust Transcription Orchestrator
                     (opentake-core/src/ai/transcription)
                                     │
         ┌───────────────────────────┼───────────────────────────┐
         │                           │                           │
   Audio Pipeline             Runtime Adapter             Model Registry
(FFmpeg 16kHz mono WAV)       (whisper.cpp CLI)         (STEP 18 Model Manager)
         │                           │                           │
         └───────────────────────────┼───────────────────────────┘
                                     │
                        Structured Segment Parser
                                     │
                        Subtitle Conversion Engine
                     (SubtitleCue / SubtitleTrack)
                                     │
                        Subtitle Editor & Timeline
```

---

## 3. Whisper Runtime Strategy & Upstream Verification

### 3.1 Upstream Evaluation: whisper.cpp
- **Upstream Repository:** [https://github.com/ggml-org/whisper.cpp](https://github.com/ggml-org/whisper.cpp)
- **Source License:** MIT License (confirmed for whisper.cpp and underlying ggml core).
- **Model Weights License:** MIT License (OpenAI Whisper tiny, base, small, medium).
- **Execution Interface:** `whisper-cli` (formerly `main`).
- **Input Contract:** 16 kHz, 16-bit signed PCM, single-channel (mono) WAV.
- **Output Contracts:** Structured JSON (`--output-json` / `-oj`) and SubRip (`--output-srt` / `-osrt`).

### 3.2 Integration Boundary Decision: Managed Subprocess
Open Media Workbench integrates `whisper.cpp` via a **secure managed subprocess adapter**:
- **Address Space Isolation:** Memory safety, crash containment, and CPU thread isolation.
- **Licensing Compliance:** Clear process boundary compliant with GPL-3.0 and MIT licensing.
- **Lightweight App Core:** Keeps application installer lightweight without forcing static C++ compilation dependencies or CUDA/ROCm bloat into the core desktop binary.
- **Extensibility:** Allows running whisper.cpp with CPU or GPU backends discovered on the host system.

### 3.3 Runtime Discovery Hierarchy
The application probes candidate executables in the following strict order:
1. User-configured custom executable (explicitly authorized via native OS file picker `rfd`).
2. App data runtime directory: `~/.local/share/open-media-workbench/bin/whisper-cli`.
3. System directories: `/usr/local/bin/whisper-cli`, `/usr/bin/whisper-cli`, `~/.local/bin/whisper-cli`.
4. Alternative binary names: `whisper-cpp`, `whisper`.

Every discovered candidate executable is probed with `--help` under a 5-second timeout to verify it exhibits the genuine whisper.cpp CLI interface (`--model`, `--file`, `--language`). Arbitrary shell strings or unverified binaries are rejected.

---

## 4. Audio Preprocessing Pipeline

Whisper models require 16 kHz, 16-bit signed PCM mono audio. The audio extraction pipeline leverages the existing secure FFmpeg runner (`opentake_core::media::process`):

```bash
ffmpeg -y -i <authorized_media_path> -vn -ac 1 -ar 16000 -c:a pcm_s16le <staging_output.wav>
```

### Safety & Resource Safeguards:
- **No Shell Execution:** Arguments are passed as discrete `String` arguments.
- **Process Group Isolation:** Process is spawned in its own PGID so cancellation terminates child threads cleanly.
- **Bounded Buffers:** Output stdout/stderr capped at 512 KiB.
- **Missing Audio Handling:** If the source media has no audio stream or corrupted audio, FFmpeg exits non-zero and the pipeline fails with a descriptive `NoAudioStream` or `ExtractionFailed` error.
- **Automatic Staging Cleanup:** The staging WAV is cleaned up on job completion, job failure, or user cancellation.

---

## 5. Model Integration & In-Use Locking

Transcription reuses the STEP 18 shared AI Model Manager:
- Models are located in `~/.local/share/open-media-workbench/ai-models/installed/{model_id}/`.
- Integrity is verified using SHA-256 before inference starts.
- **Concurrency Locking:** The model is locked via `storage.acquire_lock(model_id)` for the duration of the transcription job, preventing accidental deletion or corruption while in use.
- The lock is reliably released in a `Drop` guard or finalizer upon completion, failure, or cancellation.

### Multilingual vs English-Only Rules:
- Multilingual models (`whisper-tiny`, `whisper-base`) support all Whisper languages including English (`en`) and Bengali (`bn`), as well as Auto Detect (`auto`).
- English-only models (e.g. `tiny.en`, `base.en`) are restricted strictly to English; requests for other languages or Auto Detect return a structured `LanguageNotSupported` error.

---

## 6. Timestamp Invariants & Subtitle Conversion

Whisper segment timestamps are parsed and converted into canonical `MediaTimeUs`:
1. **Microsecond Precision:** Offsets in milliseconds are scaled by $1,000$ to produce integer microseconds.
2. **Half-Open Interval Invariant:** Every cue must satisfy $start\_us \ge 0$ and $end\_us > start\_us$. Zero-length segments are padded to a minimum threshold (100 ms) or omitted if empty.
3. **Ordering & Deduplication:** Cues are sorted chronologically by $start\_us$. Repeated identical boundary phrases are unified.
4. **Unicode Preservation:** All text tokens (including Bengali Unicode glyphs, accents, and punctuation) are preserved without character encoding corruption.
5. **Canonical Track Construction:** Cues are packaged into a `SubtitleTrack` equipped with provenance metadata (`generator: whisper.cpp`, `model_id`, `transcription_job_id`).

---

## 7. Job Lifecycle & Cancellation

```
[User Clicks Transcribe]
          │
  [Validate Asset & Model]
          │
  [Acquire Model Lock]
          │
  [Stage: PreparingAudio] ──(FFmpeg 16kHz mono)──► [Staged WAV]
          │
  [Stage: Transcribing] ──(whisper-cli inference)──► [JSON / SRT Output]
          │
  [Stage: Finalizing] ──(Parse & build SubtitleTrack)
          │
  [Release Model Lock & Cleanup Staging WAV]
          │
  [TranscriptionResult Ready in Studio UI]
```

At any stage, clicking **Cancel**:
1. Triggers the job's `CancellationToken`.
2. Sends `SIGTERM` / `SIGKILL` to the active FFmpeg or whisper.cpp process group.
3. Deletes the temporary staged WAV file.
4. Releases the model in-use lock.
5. Sets job status to `Cancelled`.

---

## 8. Localization Studio UI Integration

`TranscriptionPanel.tsx` is integrated into Localization Studio:
- Model selector showing installed Whisper models, with a link to Model Manager if none are installed.
- Runtime status indicator showing whisper.cpp availability, version, or installation guide.
- Language selector supporting Auto Detect, English, Bengali, and additional languages.
- Real-time progress display with active stages (`Preparing Audio`, `Transcribing`, `Finalizing`).
- Segment preview table showing timestamps, duration, and transcribed text.
- User confirmation buttons: **Add as New Subtitle Track**, **Replace Current Track**, or **Discard**.
- Full integration with the STEP 17 visual subtitle editor, waveform overlay, and SRT/WebVTT export.

---

## 9. Word-Level Timestamp Extraction (STEP 20 Extension)

`whisper.cpp` (v1.9.5+) generates token-level and word-level timestamps when executed with `--output-json-full` (or `-ojf`). 

### Extraction Pipeline:
1. **Runtime Token Offsets:** The parsed JSON output structure includes a `tokens` array within each transcription segment. Each token entry provides either an integer millisecond range (`offsets: { from, to }`) or formatted timestamps (`timestamps: { from, to }`).
2. **Checked Microsecond Conversion:** Token offsets in milliseconds are multiplied by $1,000$ to obtain authoritative integer `MediaTimeUs` timestamps.
3. **Punctuation & Grapheme Splitting:** Leading and trailing punctuation marks are extracted safely without splitting Unicode grapheme clusters (e.g. Bengali combining marks and Dari `।`).
4. **Validation & Ordering:** Words must satisfy $start\_us \ge 0$ and $end\_us \ge start\_us$. Words with $end\_us == start\_us$ are given a minimal duration (50,000 $\mu s$).
5. **Quality Scoring:**
   - `Aligned`: Every spoken word has verified, non-empty, chronological timestamps directly extracted from audio inference.
   - `PartiallyAligned`: Some words have verified timings while others fallback to segment bounds.
   - `Estimated`: Token timestamps were unavailable or estimated by the runtime.
   - `Unavailable`: No timing information could be derived.
6. **Zero Fabricated Timestamps:** If word-level tokens are not present in the runtime output, the pipeline never fabricates evenly spaced artificial words; it sets `timingQuality` to `Unavailable` or `Estimated`.

---

## 10. Smart Subtitle Segmentation Integration

Extracted words are fed directly into the STEP 20 smart rule-based segmentation engine (`crates/opentake-core/src/ai/alignment/segmentation.rs`), allowing users to transform long speech bursts into readable, well-paced subtitle cues adhering to professional broadcasting standards (EBU / Netflix guidelines).
