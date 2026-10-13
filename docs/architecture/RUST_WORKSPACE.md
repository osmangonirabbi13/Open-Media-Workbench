# OpenTake — Rust Workspace and Module Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 3  
**Author:** OpenTake Core Architecture Team  
**Date:** 2026-10-13  

---

## 1. Executive Summary & Evaluation of Alternatives

For OpenTake v0.1.0 Beta, the structure of the Rust codebase directly determines build times, dependency boundaries, testability, refactoring velocity, and cognitive load for the single core developer within the ~30-day timeline.

We evaluate two distinct structural architectures:

### Alternative A: Two-Tier Workspace (Tauri App Crate + Reusable Core Domain Crate)
- `crates/opentake-core`: A pure, headless Rust library containing all media models, timeline logic, audio DSP algorithms, export compilers, project serialization, and job contracts. It has **zero dependencies on Tauri, Webview, or UI concepts**.
- `src-tauri` (`opentake-desktop`): The desktop integration adapter crate. Owns Tauri command handlers, OS window lifecycles, event bus broadcasting, dialog plugins, and process lifecycle hooks.

### Alternative B: Fine-Grained Multi-Crate Workspace (6+ Crates)
- Separate crates: `opentake-media`, `opentake-timeline`, `opentake-audio`, `opentake-project`, `opentake-export`, `opentake-jobs`, and `opentake-desktop`.

---

## 2. Comparative Evaluation Matrix

| Architectural Criterion | Alternative A: Two-Tier (`core` + `desktop`) | Alternative B: Multi-Crate (6+ Crates) | Rationale & Recommendation |
|---|---|---|---|
| **Incremental Compile Time** | **Fast.** Changes to command bindings only recompile `desktop`; changes to domain logic only recompile `core`. | Moderate. Inter-crate dependency graph recompiles dependents on API touch. | Alternative A wins on rapid local iteration. |
| **Dependency Boundary Enforcement** | **High.** `opentake-core` strictly forbids Tauri, ensuring 100% headless testability. | Very High. Crates isolate sub-domains from each other. | Both satisfy headless domain isolation. |
| **API Refactoring Overhead** | **Low.** Domain model changes stay within one crate's module tree; no version syncing or workspace lock churn. | **High.** Requires updating multiple `Cargo.toml` files, traits, and inter-crate visibility modifiers. | Alternative A drastically reduces refactoring friction during early beta prototyping. |
| **Developer Cognitive Load** | **Low.** Single `src/` hierarchy inside `opentake-core`; straightforward module imports. | **High.** Navigating multiple crate roots, duplicate dev-dependencies, and workspace boilerplate. | Single developer + AI pair programming benefits strongly from Alternative A. |
| **Overengineering Risk** | **Low.** Direct, cohesive, and pragmatic (YAGNI principle). | **Very High.** Speculative decomposition before core domain contracts have stabilized. | Alternative B is premature optimization for v0.1.0. |
| **CI / Test Execution** | `cargo test -p opentake-core` runs in seconds with zero GUI/Tauri overhead. | Modular test runs, but requires workspace-wide test choreography. | Alternative A allows lightning-fast unit test cycles. |

### Architectural Recommendation for STEP 3:
**Alternative A (Two-Tier Workspace: `crates/opentake-core` + `src-tauri`) is firmly recommended.**  
It enforces absolute separation between core media domain logic and the Tauri desktop container, guarantees headless testability, and prevents premature multi-package overhead.

---

## 3. Directory Layout Blueprint

```text
opentake/
├── Cargo.toml                       # Workspace root manifest
├── crates/
│   └── opentake-core/               # Pure headless domain engine (NO Tauri dependencies)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs               # Public API re-exports
│           ├── error.rs             # Unified typed domain errors
│           ├── types.rs             # Core identifiers, timestamps, rational units
│           ├── media/               # Media inspection and validation
│           │   ├── mod.rs
│           │   ├── metadata.rs      # Probed container & stream structures
│           │   ├── probe.rs         # FFprobe runner & output parser
│           │   └── validator.rs     # Codec & container compatibility rules
│           ├── process/             # Safe child process orchestration
│           │   ├── mod.rs
│           │   ├── command.rs       # Injection-proof CommandBuilder
│           │   └── child_guard.rs   # Process lifecycle and automatic kill guards
│           ├── timeline/            # Non-destructive edit decision engine
│           │   ├── mod.rs
│           │   ├── timebase.rs      # Rational timebase & microsecond math
│           │   ├── interval.rs      # Interval algebra, partition, and merge
│           │   ├── edl.rs           # Edit Decision List and RetainedRange models
│           │   ├── mapping.rs       # Source-to-timeline time coordinate translation
│           │   └── history.rs       # Command-pattern Undo/Redo stack
│           ├── audio/               # Audio extraction and DSP analysis
│           │   ├── mod.rs
│           │   ├── pcm.rs           # Streaming 16-bit PCM sample reader
│           │   ├── dsp.rs           # Sliding-window RMS and peak dBFS math
│           │   ├── silence.rs       # Deterministic silence detection algorithm
│           │   └── waveform.rs      # Multi-resolution LOD waveform bucket generator
│           ├── project/             # Persistence and serialization
│           │   ├── mod.rs
│           │   ├── schema.rs        # Strongly typed project JSON data model
│           │   ├── io.rs            # Atomic save, fsync, and reload routines
│           │   └── migration.rs     # Project schema migration handlers
│           ├── export/              # Export pipeline and filtergraph generation
│           │   ├── mod.rs
│           │   ├── compiler.rs      # EDL to FFmpeg filtergraph compiler
│           │   ├── filtergraph.rs   # Filtergraph string assembly (trim, concat)
│           │   └── plan.rs          # Export job specification & preset configurations
│           └── jobs/                # Asynchronous background job abstractions
│               ├── mod.rs
│               ├── types.rs         # JobId, JobState, JobProgress
│               └── cancellation.rs  # CancellationToken management
├── src-tauri/                       # Desktop application adapter (Tauri 2)
│   ├── Cargo.toml
│   ├── tauri.conf.json              # Tauri application configuration & capabilities
│   ├── capabilities/                # Tauri 2 permission capability definitions
│   │   └── default.json
│   └── src/
│       ├── main.rs                  # Application entry point
│       ├── lib.rs                   # Tauri plugin setup and command registration
│       ├── state.rs                 # Shared thread-safe application state (AppState)
│       └── commands/                # Typed IPC handlers (bridge to opentake-core)
│           ├── mod.rs
│           ├── media.rs             # probe_media, inspect_file
│           ├── timeline.rs          # split_segment, trim_range, delete_range, undo
│           ├── audio.rs             # generate_waveform, analyze_silence
│           ├── project.rs           # create_project, save_project, open_project
│           ├── export.rs            # start_export, get_export_status
│           └── jobs.rs              # cancel_job, list_active_jobs
├── src/                             # React 19 + TypeScript frontend (Vite)
└── docs/                            # Comprehensive documentation
```

---

## 4. Module Responsibilities & Public API Boundaries

```
┌────────────────────────────────────────────────────────────────────────┐
│                        src-tauri (Desktop Adapter)                     │
│  - Tauri AppState (Mutex/RwLock)    - Tauri Event Dispatcher (10 Hz)   │
│  - IPC Command Bridges              - System Dialog & Window Controls  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ depends on
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                      crates/opentake-core                              │
│                                                                        │
│   ┌─────────────────────┐   ┌─────────────────────┐                    │
│   │    media::probe     │   │   process::command  │                    │
│   └──────────┬──────────┘   └──────────┬──────────┘                    │
│              │                         │                               │
│              ▼                         ▼                               │
│   ┌───────────────────────────────────────────────┐                    │
│   │         timeline::edl (Authoritative Model)   │                    │
│   └──────────┬─────────────────────────┬──────────┘                    │
│              │                         │                               │
│              ▼                         ▼                               │
│   ┌─────────────────────┐   ┌─────────────────────┐                    │
│   │   audio::silence    │   │   export::compiler  │                    │
│   │   audio::waveform   │   │   export::plan      │                    │
│   └─────────────────────┘   └─────────────────────┘                    │
│              │                         │                               │
│              └────────────┬────────────┘                               │
│                           ▼                                            │
│   ┌───────────────────────────────────────────────┐                    │
│   │         project::schema & io (Persistence)    │                    │
│   └───────────────────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Dependency Direction Rules
1. **Unidirectional Flow:** `src-tauri` depends on `opentake-core`. `opentake-core` **NEVER** depends on `src-tauri`, `tauri`, or webview APIs.
2. **Zero Global State in Core:** All core algorithms and data structures in `opentake-core` are pure functions or structs instantiated per-session. State management is explicitly orchestrated via `AppState` in `src-tauri`.
3. **Mockable Process Runner:** External process spawning in `process::command` is abstracted behind a `ProcessRunner` trait so unit and integration tests can run with mock outputs without requiring FFmpeg binaries.

---

## 5. Unified Error Propagation Design

In `crates/opentake-core/src/error.rs`, a single, comprehensive enum defines all domain errors via `thiserror`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Media probe failed: {0}")]
    ProbeFailed(String),

    #[error("Unsupported media format: container={container}, video_codec={video_codec:?}, audio_codec={audio_codec:?}")]
    UnsupportedFormat {
        container: String,
        video_codec: Option<String>,
        audio_codec: Option<String>,
    },

    #[error("Timeline validation error: {0}")]
    TimelineInvalid(String),

    #[error("Invalid edit operation: {0}")]
    InvalidEdit(String),

    #[error("Audio analysis failed: {0}")]
    AudioAnalysisFailed(String),

    #[error("Export compilation error: {0}")]
    ExportFailed(String),

    #[error("Project serialization error: {0}")]
    ProjectSerialization(#[from] serde_json::Error),

    #[error("Project validation error: {0}")]
    ProjectInvalid(String),

    #[error("Operation cancelled by user: job_id={0}")]
    Cancelled(uuid::Uuid),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
```

In `src-tauri`, errors are converted into serializable user-facing strings or structured error payloads:

```rust
// src-tauri/src/commands/mod.rs
impl serde::Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
```

---

## 6. Migration Path & Decomposition Strategy

If post-beta requirements (e.g., local AI transcription with `whisper.cpp` in P1, or custom plugin SDKs in P2) necessitate further modularization, the migration path is pre-designed:
1. `crates/opentake-core/src/audio/` can be extracted into `crates/opentake-audio` without altering public function signatures.
2. `crates/opentake-core/src/timeline/` can be extracted into `crates/opentake-timeline`.
3. The root `Cargo.toml` workspace will simply add the new crate paths.

For STEP 3 and Milestone 1, **Alternative A** provides the optimal balance of speed, discipline, and architectural clarity.
