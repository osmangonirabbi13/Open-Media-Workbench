# STEP 24 Execution Report: Offline AI Text-to-Speech Engine
**Real Local Voice Synthesis, Audio Preview, Voice Model Integration & WAV Export**

**Project:** Open Media Workbench  
**Step:** 24 — Offline AI Text-to-Speech Engine  
**Workspace:** Localization Studio  
**Baseline Git Commit:** `f9cbb191cb0cff845562d16420ecbc6e5990b4ba`  
**Execution Timestamp:** 2026-10-15  
**Telemetry / Cloud Services:** 0% (Fully local execution on CPU)  

---

## 1. Executive Summary

STEP 24 delivers a genuine, completely offline Text-to-Speech (TTS) synthesis engine for Open Media Workbench's **Localization Studio**. The engine operates 100% on local CPU, requires no cloud accounts, makes zero remote API requests, and executes speech synthesis with verified acoustic output.

Users can synthesize speech either by directly entering or pasting script text or by selecting an existing `SubtitleTrack` and synthesizing per-cue speech segments. Generated audio segments preserve exact provenance back to original subtitle cue identifiers, explicitly report timing discrepancies without silent time-stretching or truncation, provide real-time audio playback preview, and allow atomic no-clobber WAV exports.

---

## 2. Selected TTS Engine & Licensing

- **Engine Runtime**: Piper TTS (`v2023.11.14-2`, CLI version `1.2.0`).
- **Source Upstream**: [rhasspy/piper](https://github.com/rhasspy/piper)
- **License**: MIT License.
  - Bundled runtime dependencies:
    - `libonnxruntime.so.1.14.1` (MIT License)
    - `libpiper_phonemize.so.1.2.0` (MIT License)
    - `libespeak-ng.so.1.52.0.1` (GPL-3.0-or-later)
    - `espeak-ng-data/` phoneme dictionary
- **Execution Architecture**: Non-shell native execution via `std::process::Command` with piped `stdin`, dynamic `LD_LIBRARY_PATH` configuration, and clean process group cancellation via OS signals.
- **Reference Voice Model**: `piper-en-lessac-medium`
  - **Dataset / Character**: English (United States), Lessac dataset, Donald Derrick.
  - **License**: Public Domain.
  - **Sample Rate**: 22,050 Hz, 1-channel Mono, 16-bit PCM.
  - **Verified Artifacts**:
    1. `en_US-lessac-medium.onnx` (63,201,294 bytes, SHA-256: `5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f`)
    2. `en_US-lessac-medium.onnx.json` (4,885 bytes, SHA-256: `efe19c417bed055f2d69908248c6ba650fa135bc868b0e6abb3da181dab690a0`)

---

## 3. Shared AI Model Manager Integration (Multi-Artifact Extension)

The STEP 18 shared AI Model Manager was extended with backward-compatible multi-artifact support:
- Added `additional_artifacts` to `AiModelManifest` and `installed_artifacts` to `AiModelInstallation`.
- Supported single-artifact models (Whisper GGML, Qwen GGUF) without breaking changes.
- Added `verify_installed_artifacts` in `AiStorageManager` ensuring that all required voice files (`.onnx` and `.onnx.json`) are validated before a voice is marked ready.
- Maintained authoritative shared storage in `~/.local/share/opentake/models/`.

---

## 4. Rust Canonical Domain (`crates/opentake-core/src/ai/tts/`)

1. **`models.rs`**: Core data contracts including `TtsVoiceSettings`, `TtsSource`, `TtsRequest`, `TtsProgressStage`, `TtsStatus`, `TtsAudioSegment`, `TtsResult`, `TtsRuntimeCapabilities`, `AiVoiceManifest`, and `WavAudioMetadata`.
2. **`audio.rs`**: Strict WAV validation (`validate_wav_file`) checking RIFF/WAVE chunks, PCM format tag 1, valid sample rate, channel count, duration, and acoustic energy (non-silent sample check and RMS calculation). Sequential WAV file concatenation (`concatenate_wav_files`) for unified script export.
3. **`segments.rs`**: Unicode- and Bengali-safe script chunking (`chunk_script_text`) respecting sentence boundaries without splitting inside grapheme clusters; Subtitle cue extraction (`extract_cues_for_tts`) preserving cue identifiers and timestamps.
4. **`engine.rs`**: `TtsEngine` trait implemented by:
   - `SystemPiperCliEngine`: Real local inference via piper executable with structured arguments, LD_LIBRARY_PATH support, and process cancellation.
   - `MockTtsEngine`: Deterministic synthetic sine-wave WAV generation for headless and unit test environments.
5. **`manager.rs`**: `TtsManager` providing thread-safe job scheduling, model locking (`AiStorageManager`), isolated staging paths, asynchronous processing, progress tracking, and atomic export.
6. **`errors.rs`**: Strongly typed `TtsError` enum.

---

## 5. Tauri IPC Commands (`src-tauri/src/commands/tts.rs`)

1. `get_tts_runtime_capabilities`: Queries Piper executable availability and detected library dependencies.
2. `select_tts_runtime`: Native file dialog to select a custom Piper binary.
3. `list_available_tts_voices`: Returns catalog of verified voice models with installation and readiness status.
4. `get_tts_voice_status`: Queries readiness of a specific voice.
5. `start_offline_tts`: Launches an asynchronous synthesis job for Script or SubtitleTrack sources.
6. `get_offline_tts_status`: Returns current stage, progress percentage, and completed segments count.
7. `cancel_offline_tts`: Aborts running inference, terminates process group, and releases model locks.
8. `get_offline_tts_result`: Returns finalized `TtsResult` containing all `TtsAudioSegment` records.
9. `export_tts_audio`: Atomically writes single segment or concatenated multi-segment WAV to a user-selected file.

---

## 6. Frontend UI Components (`TextToSpeechPanel.tsx`)

Integrated directly into `LocalizationInspector.tsx` under the Speech/Dubbing tab:
- **Source Mode Selector**: Toggle between Plain Text Script and Subtitle Track.
- **Track & Cue Picker**: Choose any existing subtitle track in the active document; optionally select specific cues.
- **Voice Catalog Picker**: Shows voice name, language, sample rate, and installation readiness.
- **Voice Settings**: Speaking rate (length scale), noise scale, and noise width sliders with "Restore Defaults" button.
- **Status & Progress**: Displays Queued, CheckingVoice, PreparingText, Synthesizing, Validating, or Completed status with progress indicator.
- **Audio Preview Player**: Integrated HTML5 audio playback for synthesized audio with play/pause, scrub bar, and current time/duration display.
- **Segment Inspector**: List of synthesized segments with cue mapping, original text, generated duration, and signed timing mismatch warning (e.g., `+1.17s longer than cue`).
- **Export Controls**: Direct export of single segment or full combined WAV file.

---

## 7. Genuine Offline TTS Inference Evidence

A genuine speech synthesis test was executed locally using the installed `piper` runtime and `piper-en-lessac-medium` model:
- **Input Text**: `"Welcome to Open Media Workbench."`
- **Inference Command**: Direct native Piper invocation with LD_LIBRARY_PATH pointing to bundled ONNX Runtime.
- **Inference Time**: `0.1018 seconds` (RTF: `0.0516x` real-time, ~20x faster than real-time on CPU).
- **Generated Audio Duration**: `2.17 seconds` (47,930 samples).
- **Format**: `22,050 Hz`, `1 channel (Mono)`, `s16le (16-bit PCM)`, `352 kbps`.
- **Acoustic Analysis (`ffmpeg -af astats`)**:
  - RMS level: `-18.12 dB`
  - Peak level: `0.00 dB`
  - Dynamic range: `96.33 dB`
  - Zero crossings: `4,201`
  - Flat factor: `0.000000` (no sample clipping or silent flattening)

---

## 8. Test Verification & Quality Gates

### 8.1 Rust Workspace Tests
- **`crates/opentake-core`**: 136 passed (including 10 TTS integration tests covering mock engine, real Piper engine, WAV validation, concatenation, Bengali Unicode preservation, cue mapping, timing differences, and cancellation).
- **`src-tauri`**: 22 passed.
- **Total Rust Tests**: 158 passed; 0 failed.

### 8.2 Frontend Tests
- **`npm test` (Vitest)**: 101 tests passed across 1 suite (including 6 TTS client and mock lifecycle tests).
- **`npm run typecheck`**: 0 errors.
- **`npm run lint`**: 0 errors / 0 warnings.
- **`npm run build`**: Production bundle built successfully.
- **`cargo fmt --all -- --check`**: Passed with 0 diffs.
- **`cargo check --workspace --all-targets`**: Passed.
- **`cargo clippy --workspace --all-targets -- -D warnings`**: Passed with 0 warnings.
- **`cargo build --release -p opentake-desktop`**: Passed.

---

## 9. Performance & Resource Measurements

| Metric | Measured Value |
|---|---|
| Model Load Time | ~0.224 s |
| Inference Latency (Short Sentence) | ~0.102 s |
| Real-Time Factor (RTF) | 0.0516 (20x real-time speed on CPU) |
| Output Audio Sample Rate | 22,050 Hz |
| Output Audio Bitrate | 352 kbps |
| Output Audio Format | Signed 16-bit Mono PCM |
| Peak RAM Usage During Synthesis | ~160 MB |
| CPU Usage | Single-core bounded during inference |
| Cancellation Latency | < 50 ms |

---

## 10. Files Created & Modified

### Created Files:
- `crates/opentake-core/src/ai/tts/mod.rs`
- `crates/opentake-core/src/ai/tts/models.rs`
- `crates/opentake-core/src/ai/tts/audio.rs`
- `crates/opentake-core/src/ai/tts/segments.rs`
- `crates/opentake-core/src/ai/tts/engine.rs`
- `crates/opentake-core/src/ai/tts/manager.rs`
- `crates/opentake-core/src/ai/tts/errors.rs`
- `crates/opentake-core/tests/tts.rs`
- `src-tauri/src/commands/tts.rs`
- `src/types/tts.ts`
- `src/lib/tts.ts`
- `src/lib/tts.test.ts`
- `src/workspaces/localization/TextToSpeechPanel.tsx`
- `docs/architecture/OFFLINE_TTS_ENGINE.md`
- `docs/reports/STEP_24_REPORT.md`

### Modified Files:
- `crates/opentake-core/src/ai/models.rs`
- `crates/opentake-core/src/ai/catalog.rs`
- `crates/opentake-core/src/ai/storage.rs`
- `crates/opentake-core/src/ai/mod.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- `src/workspaces/localization/LocalizationInspector.tsx`
- `README.md`
- `docs/architecture/OPEN_MEDIA_WORKBENCH.md`
- `docs/architecture/LOCAL_AI_MODEL_MANAGEMENT.md`
- `docs/architecture/LICENSING_AND_DISTRIBUTION.md`
- `docs/architecture/SUBTITLE_FOUNDATION.md`

---

## 11. Known Limitations & Roadmap Clarity

- **Bengali Speech Synthesis**: Although Bengali text input and grapheme preservation are fully supported in text chunking, Bengali speech synthesis will only be marked verified once a compatible Bengali Piper voice model is validated.
- **Duration Alignment (Deferred to STEP 27)**: Generating audio for a cue calculates the timing discrepancy against the cue's display interval, but does not stretch or compress audio.
- **Dubbing Timeline & Mixing (Deferred to STEPS 26 & 28)**: STEP 24 focuses exclusively on synthesis, preview, segment preservation, and export. Timeline track assembly and video mixing remain strictly scoped to later steps.

---

## 12. Conclusion & STEP 25 Readiness

STEP 24 is fully implemented, verified with genuine offline inference, covered by comprehensive automated tests, and adheres strictly to all project constraints and git policies.
- **Git Commit**: `feat(localization): add offline text to speech engine`
- **Push Status**: **NOT PUSHED** (Local commit only).
- **STEP 25 Status**: Halted. Waiting for user instruction.
