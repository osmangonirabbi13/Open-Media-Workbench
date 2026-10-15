# STEP 18 REPORT — SHARED LOCAL AI MODEL MANAGER
## Optional Model Downloads, Cache, Security & Offline Runtime Foundation

**Project:** Open Media Workbench  
**Repository:** [https://github.com/osmangonirabbi13/Open-Media-Workbench](https://github.com/osmangonirabbi13/Open-Media-Workbench)  
**Workspace:** Shared Infrastructure (Localization Studio & Future AI Shorts Studio)  
**Execution Date:** 2026-10-15  
**Baseline Hash:** `724c15184248ed15525cafd3d4d33947fb699acd`  

---

### Executive Summary

STEP 18 of **Open Media Workbench** establishes a production-grade, offline-first, shared **AI Model Manager**. The architecture enables on-device neural network execution for both **Localization Studio** (Speech-to-Text transcription, neural voice dubbing, translation) and the upcoming **AI Shorts Studio** (speech alignment, highlight detection, social metadata generation) using a unified local model storage repository.

Crucially, **zero AI model weights are bundled** with the desktop application, preserving a lightweight installer. No models download automatically on startup. Every model installation requires explicit user consent, streams over HTTPS with constant memory, and undergoes strict cryptographic SHA-256 verification before being committed to disk.

---

### Phase-by-Phase Completion Review

#### Phase 0: Baseline Audit
- Verified Git working tree clean at commit `724c15184248ed15525cafd3d4d33947fb699acd`.
- Confirmed all STEP 17 subtitle editor components, timeline lanes, and undo/redo stacks operate without regression.
- Baseline test suites verified: 225 Rust tests and 57 frontend tests passed prior to changes.

#### Phase 1: Shared AI Architecture
- Established the dedicated systems domain `crates/opentake-core/src/ai/`:
  - `catalog.rs`: Static trusted model catalog and allowlist enforcement.
  - `downloads.rs`: Streaming network-to-disk engine with running SHA-256 and cancellation.
  - `errors.rs`: Strongly typed `AiError` domain error hierarchy.
  - `hardware.rs`: Linux procfs (`/proc/meminfo`), `statvfs` disk query, and safe GPU detection.
  - `models.rs`: Versioned data contracts (`AiModelManifest`, `AiModelInstallation`, `AiHardwareInfo`, etc.).
  - `runtime.rs`: Provider-neutral runtime traits (`AiRuntime`) and compatibility evaluators.
  - `storage.rs`: Atomic persistent registry (`registry.json`), directory containment, and in-use locking.
  - `verification.rs`: 64 KiB bounded-memory SHA-256 digest computation.
  - `mod.rs`: Top-level orchestrator `AiModelManager`.

#### Phase 2: Canonical Model Data Contracts
- Implemented versioned data contracts with camelCase serialization matching React frontend expectations.
- Defined 7 model capabilities: `SpeechToText`, `TextGeneration`, `Translation`, `TextToSpeech`, `SpeechAlignment`, `SpeakerDetection`, `VisionAnalysis`.
- Distinct operational statuses: `NotInstalled`, `Installing`, `Installed`, `Verifying`, `Ready`, `Corrupted`, `Incompatible`, `Failed`, `Removing`.
- Enforced invariant: Models are marked `Installed` upon successful checksum verification, but NOT `Ready` until an execution engine (e.g. whisper.cpp) is compiled into the host in future steps.

#### Phase 3: Trusted Curated Model Catalog
- Curated 5 distinct models with exact upstream Git LFS references and verified SHA-256 hashes:
  1. `whisper-tiny` (GGML, ~74 MB, MIT, OpenAI / ggerganov) — Verified & Installable
  2. `whisper-base` (GGML, ~141 MB, MIT, OpenAI / ggerganov) — Verified & Installable
  3. `piper-en-lessac-medium` (ONNX, ~60 MB, Public Domain, Rhasspy) — Verified & Installable
  4. `qwen2.5-0.5b-instruct` (GGUF Q4_K_M, ~468 MB, Apache-2.0, Qwen) — Verified & Installable
  5. `opus-mt-en-es` (ONNX, ~298 MB, CC-BY-4.0, Helsinki-NLP) — Catalog-Only (Disabled pending Marian runtime)

#### Phase 4: Small Installer Strategy
- Verified zero neural weights are committed to Git or embedded in application installers.
- Default application binary and web bundle remain completely lightweight.

#### Phase 5: User-Consented Model Download
- Interactive confirmation modal displays model name, purpose, download size, required disk space, available disk space, license, and upstream URL.
- Zero network requests execute prior to explicit user confirmation.

#### Phase 6: Secure Artifact Download Pipeline
- Downloads stream directly from network to `staging/{id}_{job}.part` in 64 KiB chunks with continuous SHA-256 hashing.
- Enforced hard upper limit (`expected_size + 1 MB`) to prevent disk-exhaustion attacks.
- Cancellation cleans up partial `.part` files immediately.

#### Phase 7: SHA-256 Integrity Verification
- Constant-time string comparison (`constant_time_eq_str`) prevents timing attacks.
- Mismatches immediately delete the staged artifact and record structured failure evidence.

#### Phase 8: Shared Model Storage
- Single repository located at `~/.local/share/open-media-workbench/ai-models`.
- In-use locking (`acquire_lock` / `release_lock`) protects models currently utilized by active inference or playback from being uninstalled.
- Startup reconciliation (`reconcile_startup`) removes orphaned `.part` files and audits `registry.json` against actual disk contents.

#### Phase 9: Hardware Detection
- Reads CPU logical cores via `available_parallelism()`.
- Reads total and available RAM from `/proc/meminfo` on Linux without external crates.
- Queries unprivileged available storage space via POSIX `statvfs`.
- Detects GPU presence via safe `/sys/class/drm/` nodes and reports `gpu_runtime_usable: false` honestly.

#### Phase 10: Runtime Capability Foundation
- Abstract `AiRuntime` trait designed for future whisper.cpp, llama.cpp, and Piper adapters.
- Default `UnavailableRuntime` returns structured errors noting that inference execution engines are scheduled for upcoming roadmap milestones.

#### Phase 11: Tauri IPC Security
- Exposed 9 narrowly-scoped commands: `list_ai_models`, `get_ai_model_status`, `get_ai_hardware_info`, `start_ai_model_download`, `get_ai_model_download_status`, `cancel_ai_model_download`, `verify_ai_model`, `remove_ai_model`, `get_ai_storage_usage`.
- Commands strictly validate model IDs against the curated catalog, rejecting arbitrary filesystem paths, shell commands, or external URLs.

#### Phase 12: Professional Model Manager UI
- Implemented `ModelManagerView.tsx` accessible via:
  1. Global Settings dialog under the "AI Models" tab.
  2. Quick-access "AI Models" button in Localization Studio's subtitle toolbar.
- Responsive category filters (All, Speech to Text, LLM, Translation, TTS).
- Live progress bars, bytes downloaded / total bytes, and verification indicators.
- Explicit notice: *"AI models are optional and run locally when a compatible runtime is available."*

#### Phase 13: Offline-First Behavior
- Application launches, parses media, runs diagnostics, remuxes streams, and edits subtitles 100% offline without network connectivity.
- Model Manager displays cached catalog and installed models completely offline.

---

### Automated Test Coverage

- **Rust Workspace Tests:** **243 passed, 0 failed**
  - Includes 16 comprehensive integration tests in `crates/opentake-core/tests/ai_model_manager.rs` covering schema validation, catalog loading, invalid ID rejection, trusted host checks, untrusted host rejection, SHA-256 matching and mismatching, size mismatch, streaming downloads with local mock HTTP server, cancellation cleanup, HTTP error propagation, uninstall in-use locking, path traversal rejection, startup orphan reconciliation, hardware detection, zero automatic download invariants, and large-file bounded memory verification.
- **Frontend Tests:** **66 passed, 0 failed**
  - Includes 9 unit tests in `src/lib/aiManager.test.ts` verifying catalog schemas, installable vs catalog-only filtering, hardware reporting, byte formatting, capability labels, status badges, download lifecycle, cancellation, and zero-model default execution.
- **Linters & Compilers:**
  - `cargo check --workspace --all-targets`: Passed (0 errors).
  - `cargo clippy --workspace --all-targets -- -D warnings`: Passed (0 warnings).
  - `cargo fmt --all -- --check`: Passed (0 issues).
  - `npm run typecheck`: Passed (0 errors).
  - `npm run lint`: Passed (0 warnings).
  - `npm run build`: Passed (Vite production bundle generated).
  - `cargo build -p opentake-desktop`: Passed (Native desktop binary built).

---

### Measured Footprints & Bundle Sizes

| Artifact | Measured Size | Notes |
| :--- | :--- | :--- |
| **Frontend Production Bundle** | **562 kB** (142 kB gzip) | Generated in `dist/` (JS: 486 kB, CSS: 75 kB, HTML: 1 kB) |
| **Bundled Model Weights** | **0 B** | Strictly zero model weights packaged or committed |
| **Debug Desktop Executable** | 306 MB | Unstripped debug binary in `target/debug/` |
| **Release Desktop Executable** | **24 MB** | Optimized release binary (`target/release/opentake-desktop`) |

---

### Known Limitations & STEP 19 Readiness

1. **No Active AI Inference in STEP 18:** Model weights can be downloaded, verified, and managed on disk, but active transcription or dubbing is deliberately deferred to STEP 19 (Whisper speech recognition engine integration).
2. **Translation Models:** Kept catalog-only pending Marian/Bergamot local translation runtime.
3. **Hardware Acceleration:** Hardware detection reports GPU presence, but GPU inference runtimes are not yet compiled into the application.

**STEP 19 Readiness:** The codebase is fully prepared for STEP 19 (Offline Whisper Speech-to-Text Transcription & Subtitle Generation).
