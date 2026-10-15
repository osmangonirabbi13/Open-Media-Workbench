# Offline Multilingual Subtitle Translation Architecture

**Module:** `crates/opentake-core/src/ai/translation/`  
**Desktop IPC:** `src-tauri/src/commands/translation.rs`  
**Frontend Components:** `src/workspaces/localization/TranslationPanel.tsx`, `src/lib/translation.ts`  
**Document Version:** 1.0.0  
**Status:** Implemented in STEP 22  
**Scope:** Open Media Workbench — Localization Studio  

---

## 1. Executive Summary & Design Principles

Open Media Workbench provides content creators and localizers with a 100% offline, privacy-guaranteed subtitle translation engine powered by local neural machine inference.

### Key Architectural Invariants
1. **Zero Cloud Inference:** No external network requests, paid translation APIs, telemetry, or remote server dependencies during translation execution.
2. **Deterministic Timing Preservation:** Every translated cue strictly retains the exact microsecond start and end timestamps of the original cue:
   $$\text{target\_start\_us} = \text{source\_start\_us}$$
   $$\text{target\_end\_us} = \text{source\_end\_us}$$
3. **Non-Destructive Workflow:** Source tracks and cues remain 100% immutable. Translations create an independent `SubtitleTrack` in the active `SubtitleDocument`.
4. **Evidence-Based Word Alignment Isolation:** Source-language word alignment timestamps (from STEP 20) are NOT inherited on translated text, as word order and token boundaries do not acoustically map across languages without speech alignment.
5. **Shared AI Model Repository:** Reuses models stored in the shared STEP 18 AI Model Manager (`~/.local/share/opentake/models`), with pre-flight SHA-256 verification and in-use concurrency locking.
6. **Managed Subprocess Safety:** Invokes the verified `llama.cpp` CLI binary via structured non-shell argument vectors, with process group isolation, output bounds, and cooperative cancellation.

---

## 2. System Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│               Localization Studio UI (React 19)                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    TranslationPanel                              │  │
│  │ - Source track selector          - Language pair compatibility   │  │
│  │ - Model Manager link             - Runtime readiness indicator   │  │
│  │ - Progress / Cancellation        - Original vs Translated review │  │
│  │ - Non-blocking CPS warnings      - Apply as New Track            │  │
│  └──────────────────────────────────┬───────────────────────────────┘  │
└─────────────────────────────────────┼──────────────────────────────────┘
                                      │ Tauri 2 IPC
┌─────────────────────────────────────┴──────────────────────────────────┐
│                   Tauri Authorized Commands Layer                      │
│  ├── get_translation_capabilities                                      │
│  ├── list_translation_language_pairs                                   │
│  ├── select_translation_runtime                                        │
│  ├── start_offline_translation                                         │
│  ├── get_offline_translation_status                                    │
│  ├── cancel_offline_translation                                        │
│  ├── get_offline_translation_result                                    │
│  └── apply_translation_result_to_subtitles                             │
└─────────────────────────────────────┬──────────────────────────────────┘
                                      │ Rust Domain Layer
┌─────────────────────────────────────┴──────────────────────────────────┐
│              opentake-core::ai::translation::TranslationManager        │
│  ├── Pre-flight Model & Integrity Validation (SHA-256)                 │
│  ├── In-Use Concurrency Lock (AiStorageManager::acquire_lock)          │
│  ├── TranslationEngine Trait Adapter                                   │
│  └── TranslationPipeline (Prompt formulation, inference, validation)   │
└─────────────────────────────────────┬──────────────────────────────────┘
                                      │ Managed Child Process (No Shell)
┌─────────────────────────────────────┴──────────────────────────────────┐
│                      llama-cli (llama.cpp b11658)                      │
│  - Model: Qwen 2.5 0.5B Instruct (GGUF Q4_K_M, 468 MB)                 │
│  - CPU Threads: Configurable (default: logical CPU count)              │
│  - Arguments: --simple-io --no-display-prompt -st -m <path> -p <prompt>│
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Curated Translation Engine & Model Selection

### 3.1 Runtime Selection: `llama.cpp` CLI
- **Binary:** Official `llama.cpp` CLI release (`llama-cli`), version `b11658`.
- **License:** MIT License (100% compatible with GPL-3.0-or-later workbench).
- **Execution Mode:** Non-interactive, single-turn mode (`--simple-io --no-display-prompt -st`), avoiding interactive REPL loops.
- **Resource Footprint:** Consumes ~650 MB peak RAM during inference.

### 3.2 Model Selection: Qwen 2.5 0.5B Instruct GGUF
- **Artifact:** `qwen2.5-0.5b-instruct-q4_k_m.gguf`
- **Size:** 491,400,032 bytes (468 MB).
- **Architecture:** Compact transformer instruction model with native multilingual vocabulary.
- **License:** Apache-2.0 License.
- **Pinned SHA-256:** `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db`
- **Measured CPU Inference Speed:** ~255 tokens/sec prompt ingestion, ~52 tokens/sec generation on standard x86_64 CPU.

---

## 4. Authoritative Language Pair System

The translation engine enforces authoritative language pair validation before allowing jobs to start:

| Source | Target | Model ID | Runtime | Status |
| :--- | :--- | :--- | :--- | :--- |
| English (`en`) | Spanish (`es`) | `qwen2.5-0.5b-instruct` | `llama-cli` | **Verified** |
| Spanish (`es`) | English (`en`) | `qwen2.5-0.5b-instruct` | `llama-cli` | **Verified** |
| English (`en`) | Bengali (`bn`) | `qwen2.5-0.5b-instruct` | `llama-cli` | Experimental / Unverified |
| Bengali (`bn`) | English (`en`) | `qwen2.5-0.5b-instruct` | `llama-cli` | Experimental / Unverified |
| English (`en`) | French (`fr`) | `qwen2.5-0.5b-instruct` | `llama-cli` | Planned |
| English (`en`) | German (`de`) | `qwen2.5-0.5b-instruct` | `llama-cli` | Planned |
| English (`en`) | Japanese (`ja`) | `qwen2.5-0.5b-instruct` | `llama-cli` | Planned |

---

## 5. Timing Preservation & Subtitle Conversion

### 5.1 The Timing Invariant
Subtitle translation strictly operates under the invariant that dialogue timestamps are dictated by the underlying video and audio timeline:
- The translated cue inherits the original cue's `start_us` and `end_us` with microsecond precision (`MediaTimeUs`).
- Duration is never automatically lengthened or compressed to fit translated text.
- If translated text has high reading speed, non-blocking readability warnings are generated:
  - **CPS Warning:** Characters per second exceeding 25 CPS.
  - **Character Overflow Warning:** Line length exceeding 84 characters.

### 5.2 Word Alignment Handling
In STEP 20, word-level timestamps were extracted from Whisper for the source language. When converting translation results into a new `SubtitleTrack`:
```rust
let mut cue = SubtitleCue::new(&item.translated_cue_id, start_us, end_us, &item.translated_text)?;
cue.words = None; // Source-language word alignment is cleared on translated words
```
Target-language speech-to-text alignment will be addressed in future steps.

---

## 6. End-to-End Workflow

1. User imports a video and subtitle file (`.srt`, `.vtt`, or `.omwsub`).
2. User opens the **Translate** tab in Localization Studio.
3. User selects the active source subtitle track and target language (e.g. English -> Spanish).
4. System validates installed model and runtime operational status.
5. User clicks **Start Offline Translation**.
6. The Rust `TranslationManager` locks the model, launches `llama-cli`, and parses structured results.
7. Progress updates stream in real-time to the UI.
8. Upon completion, a side-by-side comparison table displays original vs translated text, timestamps, and warnings.
9. User clicks **Apply as New Track**.
10. A new `SubtitleTrack` is appended to `SubtitleDocument`, immediately editable in the timeline editor and exportable to `.srt`, `.vtt`, or burned into video via STEP 21.
