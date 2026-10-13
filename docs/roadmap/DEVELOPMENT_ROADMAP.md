# OpenTake — 32-Step Development Roadmap

**Document Version:** 1.0.0-approved  
**Target Release:** v0.1.0 Beta  
**Target Development Window:** ~30 Days  
**Primary Platform:** Ubuntu Linux x86_64 (Windows/macOS validation conditional)  
**Author:** OpenTake Core Architecture Team  
**Date:** 2026-10-13  

---

## 1. Roadmap Architecture & Milestones Overview

The roadmap consists of **8 Milestones** comprising exactly **32 sequential and verifiable steps**. Each step is defined with concrete objectives, technical strategy, prerequisites, file deliverables, difficulty rating, acceptance criteria, verification method, and Definition of Done.

```
Milestone 1: Foundation (Days 1–4)
  ├── STEP 1: Requirements, Scope Lock & Acceptance Baseline [ACTIVE]
  ├── STEP 2: Architecture & Licensing Decisions
  ├── STEP 3: Tauri + Rust + React Project Bootstrap
  └── STEP 4: App Shell, Navigation & Design System

Milestone 2: Media Foundation (Days 5–8)
  ├── STEP 5: FFmpeg/FFprobe Integration
  ├── STEP 6: File Import & Metadata Inspection
  ├── STEP 7: Playback & Seeking Foundation
  └── STEP 8: Audio Waveform Generation

Milestone 3: Editing Engine (Days 9–13)
  ├── STEP 9: Non-Destructive Editing Data Model
  ├── STEP 10: Visual Timeline & Manual Editing
  ├── STEP 11: Undo/Redo & Keyboard Shortcuts
  ├── STEP 12: Project Persistence & Reopen
  └── STEP 13: Preview-to-Timeline Synchronization

Milestone 4: Automatic Editing (Days 14–17)
  ├── STEP 14: Audio Analysis Pipeline
  ├── STEP 15: Silence Detection Algorithm
  ├── STEP 16: Smart Cut Generation
  └── STEP 17: Automatic Edit Review UX

Milestone 5: Export Engine (Days 18–21)
  ├── STEP 18: Export Timeline Compiler
  ├── STEP 19: MP4 Export & Quality Presets
  ├── STEP 20: Export Progress & Cancellation
  └── STEP 21: Audio/Video Sync Validation

Milestone 6: Polish & Reliability (Days 22–25)
  ├── STEP 22: Onboarding, Settings & UX Polish
  ├── STEP 23: Performance, Memory & Large-File Safety
  ├── STEP 24: Security & Dependency Audit
  └── STEP 25: Automated Regression Tests

Milestone 7: Open Source & Distribution (Days 26–28)
  ├── STEP 26: Open-Source Repository Documentation
  ├── STEP 27: Cross-Platform CI & Packaging
  └── STEP 28: Installer/Release Verification

Milestone 8: Beta Launch (Days 29–30)
  ├── STEP 29: External Beta Testing & Critical Fixes
  ├── STEP 30: Launch Materials & Showcase
  ├── STEP 31: GitHub Beta Release
  └── STEP 32: Feedback & Post-Release Roadmap
```

---

## 2. Milestone 1 — Foundation (Days 1–4)

### STEP 1 — Requirements, Scope Lock & Acceptance Baseline
- **Objective:** Establish a rock-solid, verifiable documentation foundation and lock the v0.1.0 Beta scope.
- **Scope:** PRD, Release Scope, Technical Constraints, Acceptance Criteria, ADR-0001, Risk Register, Roadmap, and Step 1 Report. No application code.
- **Technical Strategy:** Documentation-first engineering; define exact testable criteria and invariants before writing code.
- **Prerequisites:** Development environment inspection.
- **Expected Files:** `docs/requirements/PRODUCT_REQUIREMENTS.md`, `docs/requirements/RELEASE_SCOPE.md`, `docs/architecture/TECHNICAL_CONSTRAINTS.md`, `docs/requirements/ACCEPTANCE_CRITERIA.md`, `docs/decisions/ADR-0001.md`, `docs/requirements/RISK_REGISTER.md`, `docs/roadmap/DEVELOPMENT_ROADMAP.md`, `docs/reports/STEP_01_REPORT.md`, `README.md`.
- **Difficulty:** M
- **Acceptance Criteria:** All documentation files exist, are internally consistent, and meet the Step 1 Definition of Done.
- **Required Verification:** File presence, link consistency, and roadmap verification checks.
- **Definition of Done:** Documentation approved by lead engineer with zero unresolved contradictions.
- **Dependencies:** None.

### STEP 2 — Architecture & Licensing Decisions
- **Objective:** Finalize open-source licensing strategy (GPL-3.0 vs AGPL-3.0 vs MIT/Apache with sidecar) and create ADR-0002 for process isolation and sidecar strategy.
- **Scope:** Legal license audit of FFmpeg, libx264, Tauri plugins, and runtime dependencies.
- **Technical Strategy:** Document clear GPL boundary separation; define sidecar packaging vs system binary lookup.
- **Prerequisites:** STEP 1 complete.
- **Expected Files:** `docs/decisions/ADR-0002.md`, `LICENSE`, `COPYING`.
- **Difficulty:** S
- **Acceptance Criteria:** License selected, compatible with FFmpeg GPL distribution, and documented in ADR-0002.
- **Required Verification:** License compatibility matrix cross-check.
- **Definition of Done:** Finalized license file and approved ADR-0002.
- **Dependencies:** STEP 1.

### STEP 3 — Tauri + Rust + React Project Bootstrap
- **Objective:** Initialize the Tauri 2 + Rust + React 19 + TypeScript + Vite project skeleton.
- **Scope:** Clean project bootstrap with verified desktop window launch, IPC ping/pong, and build validation.
- **Technical Strategy:** Use official Tauri CLI; configure `tauri.conf.json` with strict security permissions and CSP.
- **Prerequisites:** STEP 2 complete, verified Rust/Node toolchain.
- **Expected Files:** `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src/App.tsx`, `package.json`, `vite.config.ts`, `tsconfig.json`.
- **Difficulty:** M
- **Acceptance Criteria:** `npm run tauri dev` launches an empty desktop window and returns a greeting over Tauri IPC.
- **Required Verification:** Successful build and run on Ubuntu Linux x86_64.
- **Definition of Done:** Application builds, runs, passes lint, and displays desktop window without console errors.
- **Dependencies:** STEP 2.

### STEP 4 — App Shell, Navigation & Design System
- **Objective:** Implement the professional dark-mode desktop layout and design system.
- **Scope:** Top header (project title, media info), center viewport (preview), bottom deck (timeline container), toolbar, and Radix UI dialogs.
- **Technical Strategy:** Tailwind CSS utility styling, Lucide icons, accessible Radix UI primitives.
- **Prerequisites:** STEP 3 complete.
- **Expected Files:** `src/components/layout/AppShell.tsx`, `src/components/layout/Header.tsx`, `src/components/layout/BottomDeck.tsx`, `src/index.css`.
- **Difficulty:** M
- **Acceptance Criteria:** Clean, ergonomic dark UI rendering empty states with 60 FPS responsiveness.
- **Required Verification:** Browser/Tauri visual inspection and responsiveness testing.
- **Definition of Done:** Complete visual shell with responsive layout and zero layout jumping.
- **Dependencies:** STEP 3.

---

## 3. Milestone 2 — Media Foundation (Days 5–8)

### STEP 5 — FFmpeg/FFprobe Integration
- **Objective:** Establish the Rust child process orchestration engine for FFprobe and FFmpeg.
- **Scope:** Typed command builder, async stdout/stderr streaming, process termination, version checks.
- **Technical Strategy:** Pure `tokio::process::Command` with typed arguments; zero shell string interpolation.
- **Prerequisites:** STEP 4 complete, local FFmpeg/FFprobe binaries.
- **Expected Files:** `src-tauri/src/media/ffmpeg.rs`, `src-tauri/src/media/ffprobe.rs`, `src-tauri/src/media/command.rs`.
- **Difficulty:** M
- **Acceptance Criteria:** Rust commands can invoke `ffprobe` and `ffmpeg`, stream output, and terminate cleanly on command.
- **Required Verification:** Rust integration tests spawning mock and real probe commands.
- **Definition of Done:** Unit tests pass; error handling captures invalid binaries and non-zero exit codes.
- **Dependencies:** STEP 4.

### STEP 6 — File Import & Metadata Inspection
- **Objective:** Ingest local video files and extract technical stream metadata into the application state.
- **Scope:** Drag-and-drop zone, file picker dialog, FFprobe JSON parsing (duration, resolution, fps, codecs).
- **Technical Strategy:** Tauri drag-drop events, `rfd` / Tauri dialog plugin, strongly-typed `MediaMetadata` struct in Rust and TS.
- **Prerequisites:** STEP 5 complete.
- **Expected Files:** `src-tauri/src/commands/import.rs`, `src/components/import/DropZone.tsx`, `src/types/media.ts`.
- **Difficulty:** M
- **Acceptance Criteria:** User can drop MP4/MOV/MKV, inspect duration/resolution/fps, and see unsupported files rejected with helpful errors.
- **Required Verification:** Import tests with valid 1080p MP4, corrupted file, and non-media file.
- **Definition of Done:** Metadata parses accurately within 1.0 second and renders in header.
- **Dependencies:** STEP 5.

### STEP 7 — Playback & Seeking Foundation
- **Objective:** Build the preview player with synchronized transport controls and low-latency seeking.
- **Scope:** HTML5 `<video>` player, play/pause, scrub bar, timecode formatting, Tauri asset streaming protocol.
- **Technical Strategy:** Register custom asset protocol or local stream bridge; use requestAnimationFrame for playhead sync.
- **Prerequisites:** STEP 6 complete.
- **Expected Files:** `src/components/preview/VideoPlayer.tsx`, `src/components/preview/TransportControls.tsx`, `src/hooks/useVideoPlayer.ts`.
- **Difficulty:** L
- **Acceptance Criteria:** Smooth playback of H.264/AAC videos; playhead scrubbing updates video frames in $\le 150\text{ ms}$.
- **Required Verification:** Manual playback and seeking tests with 24fps and 60fps video clips.
- **Definition of Done:** Video plays, pauses, seeks, and displays accurate timecode without audio desync.
- **Dependencies:** STEP 6.

### STEP 8 — Audio Waveform Generation
- **Objective:** Extract audio track, decode to PCM, and compute multi-resolution waveform buckets.
- **Scope:** Background audio decoding via FFmpeg, RMS/peak bucket calculation in Rust, canvas waveform rendering in React.
- **Technical Strategy:** Streaming 64 KB PCM chunks; pyramid LOD storage; HTML5 2D Canvas rendering.
- **Prerequisites:** STEP 7 complete.
- **Expected Files:** `src-tauri/src/audio/waveform.rs`, `src/components/timeline/WaveformCanvas.tsx`.
- **Difficulty:** L
- **Acceptance Criteria:** Waveform renders beneath timeline in $< 3\text{ s}$ for a 10-minute clip; memory usage remains bounded $< 500\text{ MB}$.
- **Required Verification:** Ingest 10-minute and 60-minute audio-bearing clips, inspect waveform rendering and RAM usage.
- **Definition of Done:** Smooth zooming and scrolling over the waveform canvas with zero UI stutter.
- **Dependencies:** STEP 7.

---

## 4. Milestone 3 — Editing Engine (Days 9–13)

### STEP 9 — Non-Destructive Editing Data Model
- **Objective:** Implement the core Edit Decision List (EDL) and RetainedRange data structures in Rust.
- **Scope:** Integer microsecond timebase (`i64`), split, trim, delete, restore, source-to-output mapping algorithms.
- **Technical Strategy:** Pure Rust module with interval math; zero floating-point timeline coordinates.
- **Prerequisites:** STEP 8 complete.
- **Expected Files:** `src-tauri/src/edl/model.rs`, `src-tauri/src/edl/intervals.rs`, `src-tauri/src/edl/mapping.rs`.
- **Difficulty:** M
- **Acceptance Criteria:** Interval partitioning and merging passes 100% of unit tests; original media is never touched.
- **Required Verification:** Comprehensive Rust unit test suite covering split, delete, restore, and edge-case interval overlaps.
- **Definition of Done:** All interval operations tested and documented with zero unsafe code.
- **Dependencies:** STEP 8.

### STEP 10 — Visual Timeline & Manual Editing
- **Objective:** Connect the React timeline UI to the Rust EDL model for visual splitting and trimming.
- **Scope:** Timeline ruler, clip segments, razor split tool (`S`), trim handles, selection, delete segment (`Backspace`).
- **Technical Strategy:** Virtualized timeline container, CSS transforms, Tauri IPC commands (`split_range`, `delete_range`).
- **Prerequisites:** STEP 9 complete.
- **Expected Files:** `src/components/timeline/TimelineRuler.tsx`, `src/components/timeline/ClipSegment.tsx`, `src/components/timeline/TimelineTrack.tsx`.
- **Difficulty:** L
- **Acceptance Criteria:** User can visually split clips at the playhead, drag trim handles, and delete segments with immediate visual feedback.
- **Required Verification:** Interactive UI testing performing 20 manual splits and deletions.
- **Definition of Done:** Timeline tracks faithfully represent backend EDL state at all times.
- **Dependencies:** STEP 9.

### STEP 11 — Undo/Redo & Keyboard Shortcuts
- **Objective:** Implement the Command-Pattern history stack and central keyboard shortcut manager.
- **Scope:** Undo (`Ctrl+Z`), Redo (`Ctrl+Shift+Z` / `Ctrl+Y`), shortcuts for Play/Pause, Split, Delete, and Jump.
- **Technical Strategy:** History stack in Rust with snapshot rollbacks; global event listener in React with input-focus guards.
- **Prerequisites:** STEP 10 complete.
- **Expected Files:** `src-tauri/src/edl/history.rs`, `src/hooks/useKeyboardShortcuts.ts`.
- **Difficulty:** M
- **Acceptance Criteria:** Any editing action can be undone and redone cleanly; shortcuts operate reliably without triggering when typing in inputs.
- **Required Verification:** Automated unit test executing 50 random mutations and asserting state match after 50 undos.
- **Definition of Done:** Undo/Redo stack behaves predictably with zero state desynchronization.
- **Dependencies:** STEP 10.

### STEP 12 — Project Persistence & Reopen
- **Objective:** Save and reload editing projects via versioned `.opentake` JSON files.
- **Scope:** Save Project (`Ctrl+S`), Save As, Open Project (`Ctrl+O`), missing media detection and relink modal.
- **Technical Strategy:** `serde_json` serialization in Rust, atomic temp-file swap (`.tmp` -> `.opentake`), SHA-256 source file check.
- **Prerequisites:** STEP 11 complete.
- **Expected Files:** `src-tauri/src/project/schema.rs`, `src-tauri/src/project/io.rs`, `src/components/project/RelinkDialog.tsx`.
- **Difficulty:** M
- **Acceptance Criteria:** Projects save atomically and reload with 100% edit fidelity; missing media triggers a friendly relink prompt.
- **Required Verification:** Save/load roundtrip integration tests with simulated disk write failures.
- **Definition of Done:** Robust file persistence with schema versioning (`schema_version: 1`).
- **Dependencies:** STEP 11.

### STEP 13 — Preview-to-Timeline Synchronization
- **Objective:** Implement real-time cut-skipping during preview playback.
- **Scope:** When playing with "Skip Silence" enabled, the video preview automatically leaps over excised segments.
- **Technical Strategy:** Time monitor hook polling `<video>` timecode against EDL retained ranges; instant seeking to next range boundary.
- **Prerequisites:** STEP 12 complete.
- **Expected Files:** `src/hooks/useTimelinePlayback.ts`, `src/components/preview/PlaybackOptions.tsx`.
- **Difficulty:** M
- **Acceptance Criteria:** Playback seamlessly jumps across cut gaps without user intervention or player stalls.
- **Required Verification:** Auditioning 5 consecutive cuts during live playback.
- **Definition of Done:** Audio and video continue smoothly after skipping excluded gaps.
- **Dependencies:** STEP 12.

---

## 5. Milestone 4 — Automatic Editing (Days 14–17)

### STEP 14 — Audio Analysis Pipeline
- **Objective:** Build the streaming PCM audio extraction and RMS/peak decibel calculation pipeline in Rust.
- **Scope:** Extract 16-bit PCM via FFmpeg pipe, compute sliding-window audio power in dBFS, report progress over IPC.
- **Technical Strategy:** Streaming reader with 64 KB buffer; windowed RMS formula: $\text{dBFS} = 20 \log_{10}(\text{RMS} / 32768.0)$.
- **Prerequisites:** STEP 13 complete.
- **Expected Files:** `src-tauri/src/audio/pcm.rs`, `src-tauri/src/audio/dsp.rs`.
- **Difficulty:** M
- **Acceptance Criteria:** Processes 1 hour of audio in $< 10\text{ seconds}$ without exceeding 100 MB of RAM.
- **Required Verification:** Benchmarking audio extraction on synthetic sine wave and silence fixtures.
- **Definition of Done:** Clean, verified DSP module producing accurate dBFS time series.
- **Dependencies:** STEP 13.

### STEP 15 — Silence Detection Algorithm
- **Objective:** Implement the core silence interval detection algorithm with configurable thresholds and padding.
- **Scope:** Threshold (-60 to -20 dBFS), Min Duration (200 to 2000 ms), Pre/Post Padding (50 to 500 ms).
- **Technical Strategy:** Two-pointer interval scanner in Rust; boundary expansion with padding clamping; min-gap filtering.
- **Prerequisites:** STEP 14 complete.
- **Expected Files:** `src-tauri/src/audio/silence.rs`, `src-tauri/src/tests/silence_tests.rs`.
- **Difficulty:** M
- **Acceptance Criteria:** Accurately detects silence intervals within $\pm 20\text{ ms}$ of ground truth on synthetic test fixtures.
- **Required Verification:** Automated unit tests with known audio tone/silence patterns.
- **Definition of Done:** Zero false negatives on calibrated synthetic silence fixtures.
- **Dependencies:** STEP 14.

### STEP 16 — Smart Cut Generation
- **Objective:** Convert detected silence candidates into pending EDL cut suggestions.
- **Scope:** Generate candidate cut regions, calculate time saved, clamp to media boundaries.
- **Technical Strategy:** Rust command returning typed `SilenceDetectionResult` with candidate list and statistics.
- **Prerequisites:** STEP 15 complete.
- **Expected Files:** `src-tauri/src/commands/silence.rs`, `src/types/silence.ts`.
- **Difficulty:** S
- **Acceptance Criteria:** Returns ordered candidate cuts with exact microsecond bounds and estimated time saved.
- **Required Verification:** Integration test verifying cut conversion against source duration.
- **Definition of Done:** Candidate cuts generated and verified without mutating the active EDL until user confirms.
- **Dependencies:** STEP 15.

### STEP 17 — Automatic Edit Review UX
- **Objective:** Build the interactive review interface for auditioning, accepting, and rejecting suggested cuts.
- **Scope:** Suggestion drawer, threshold sliders, Next Cut / Prev Cut navigation (`[` and `]`), Accept/Reject buttons, Batch Apply.
- **Technical Strategy:** Visual cut markers overlaid on waveform; keyboard-driven cycling through candidate cut points.
- **Prerequisites:** STEP 16 complete.
- **Expected Files:** `src/components/silence/SilenceConfigModal.tsx`, `src/components/silence/CutReviewDrawer.tsx`, `src/components/timeline/CandidateCutMarker.tsx`.
- **Difficulty:** L
- **Acceptance Criteria:** User can cycle through cuts, audition transitions, toggle accept/reject, and batch-apply with 1 click.
- **Required Verification:** User interaction smoke tests reviewing 10 candidate cuts.
- **Definition of Done:** Polished, responsive review flow with instant timeline visual updates.
- **Dependencies:** STEP 16.

---

## 6. Milestone 5 — Export Engine (Days 18–21)

### STEP 18 — Export Timeline Compiler
- **Objective:** Compile the EDL retained ranges into an optimized FFmpeg filtergraph.
- **Scope:** Generate `trim`, `atrim`, `setpts`, `asetpts`, and `concat` filtergraph strings in Rust.
- **Technical Strategy:** Deterministic filtergraph builder; microsecond-to-second formatting with 6 decimal places.
- **Prerequisites:** STEP 17 complete.
- **Expected Files:** `src-tauri/src/export/compiler.rs`, `src-tauri/src/export/filtergraph.rs`.
- **Difficulty:** M
- **Acceptance Criteria:** Generated filtergraph produces valid FFmpeg command syntax for any valid EDL configuration.
- **Required Verification:** Unit test generating filtergraphs for 1, 5, 50, and 200 retained ranges and validating syntax.
- **Definition of Done:** Filtergraph compiler passes all syntax tests without buffer overflows or malformed tags.
- **Dependencies:** STEP 17.

### STEP 19 — MP4 Export & Quality Presets
- **Objective:** Execute final video rendering using FFmpeg with standard H.264/AAC encoding presets.
- **Scope:** 1080p, 720p, and Match Source presets; CRF quality options (High, Balanced, Draft); atomic `.tmp` file output.
- **Technical Strategy:** Async FFmpeg spawn; progress parsing from stderr; atomic file rename upon success.
- **Prerequisites:** STEP 18 complete.
- **Expected Files:** `src-tauri/src/export/runner.rs`, `src/components/export/ExportModal.tsx`.
- **Difficulty:** L
- **Acceptance Criteria:** Renders valid MP4 file matching selected preset; output plays cleanly in standard external players.
- **Required Verification:** Export test of a 2-minute edited video; inspect output streams with FFprobe.
- **Definition of Done:** Exported video verified with correct resolution, codec, duration, and zero watermark.
- **Dependencies:** STEP 18.

### STEP 20 — Export Progress & Cancellation
- **Objective:** Provide live rendering progress feedback and instant, safe export cancellation.
- **Scope:** Progress percentage, frame count, elapsed time, ETA calculation; responsive "Cancel" button.
- **Technical Strategy:** Throttled Tauri events (10 Hz); `CancellationToken` killing FFmpeg and deleting partial `.tmp` files.
- **Prerequisites:** STEP 19 complete.
- **Expected Files:** `src-tauri/src/export/progress.rs`, `src/components/export/ExportProgressModal.tsx`.
- **Difficulty:** M
- **Acceptance Criteria:** Progress bar updates smoothly; clicking Cancel halts FFmpeg within 1.5 seconds and purges temp file.
- **Required Verification:** Trigger export, cancel at 50%, verify child process reaped and temp file unlinked.
- **Definition of Done:** Zero orphan processes or lingering `.tmp` files after cancellation.
- **Dependencies:** STEP 19.

### STEP 21 — Audio/Video Sync Validation
- **Objective:** Mathematically and empirically validate A/V synchronization across exported cut boundaries.
- **Scope:** Boundary click/pop elimination, timestamp discontinuity testing, drift measurement.
- **Technical Strategy:** Test with synthetic AV-sync flash/beep calibration video; automated script checking sync delta.
- **Prerequisites:** STEP 20 complete.
- **Expected Files:** `tests/fixtures/av_sync_test.mp4`, `tests/verify_sync.py` or Rust test harness.
- **Difficulty:** L
- **Acceptance Criteria:** AV drift remains $< \pm 25\text{ ms}$ throughout the entire exported file; zero audible splice clicks.
- **Required Verification:** Automated calibration test and manual audition with headphones.
- **Definition of Done:** Passes REQ-EXPORT-003 acceptance criteria with documented test logs.
- **Dependencies:** STEP 20.

---

## 7. Milestone 6 — Polish & Reliability (Days 22–25)

### STEP 22 — Onboarding, Settings & UX Polish
- **Objective:** Implement user settings, persistent preferences, and first-run onboarding guides.
- **Scope:** Settings modal (default silence threshold, default export preset, scratch directory path), tooltips, keyboard cheat sheet.
- **Technical Strategy:** Local settings persistence via `tauri-plugin-store` or local JSON config.
- **Prerequisites:** STEP 21 complete.
- **Expected Files:** `src/components/settings/SettingsModal.tsx`, `src/components/help/KeyboardShortcutsModal.tsx`.
- **Difficulty:** S
- **Acceptance Criteria:** Settings persist across application restarts; shortcut cheat sheet is accessible via `?` key.
- **Required Verification:** Manual configuration changes and restart verification.
- **Definition of Done:** Polished desktop interface with comprehensive user guidance.
- **Dependencies:** STEP 21.

### STEP 23 — Performance, Memory & Large-File Safety
- **Objective:** Stress-test OpenTake with large multi-hour recordings (2–3 hours, $\ge 10\text{ GB}$).
- **Scope:** Memory profiling, disk space pre-flight validation, UI responsiveness under 500+ cuts.
- **Technical Strategy:** Linux `valgrind` / `/proc` memory tracking; disk space checks via `fs2` crate in Rust.
- **Prerequisites:** STEP 22 complete.
- **Expected Files:** `src-tauri/src/util/disk.rs`, `docs/reports/BENCHMARK_REPORT.md`.
- **Difficulty:** M
- **Acceptance Criteria:** Memory stays $< 600\text{ MB}$ RSS; pre-flight check blocks export if target disk space is insufficient.
- **Required Verification:** 3-hour fixture video stress test.
- **Definition of Done:** Zero OOM crashes, UI remains $\ge 55\text{ FPS}$ during playback with 500 cuts.
- **Dependencies:** STEP 22.

### STEP 24 — Security & Dependency Audit
- **Objective:** Audit native Rust crates, npm packages, process invocations, and filesystem permissions.
- **Scope:** `cargo audit`, `npm audit`, static audit for forbidden shell strings, path traversal validation.
- **Technical Strategy:** Automated security linting scripts; strict path canonicalization review.
- **Prerequisites:** STEP 23 complete.
- **Expected Files:** `docs/security/AUDIT_REPORT.md`.
- **Difficulty:** S
- **Acceptance Criteria:** Zero critical/high CVEs in dependencies; zero raw shell concatenations in codebase.
- **Required Verification:** Clean `cargo audit` and `npm audit` runs; path traversal test suite execution.
- **Definition of Done:** Signed security audit report meeting REQ-SEC-001 and REQ-SEC-002.
- **Dependencies:** STEP 23.

### STEP 25 — Automated Regression Tests
- **Objective:** Consolidate end-to-end automated integration and regression test suites.
- **Scope:** Rust integration tests, React component tests, end-to-end import-cut-export regression tests.
- **Technical Strategy:** Cargo test harnesses and Vitest component testing.
- **Prerequisites:** STEP 24 complete.
- **Expected Files:** `tests/integration_tests.rs`, `src/__tests__/Timeline.test.tsx`.
- **Difficulty:** M
- **Acceptance Criteria:** Entire test suite runs and passes cleanly via single command (`cargo test && npm test`).
- **Required Verification:** Clean test run in terminal.
- **Definition of Done:** All acceptance criteria tests pass with reproducible results.
- **Dependencies:** STEP 24.

---

## 8. Milestone 7 — Open Source & Distribution (Days 26–28)

### STEP 26 — Open-Source Repository Documentation
- **Objective:** Prepare the repository for global open-source contributors.
- **Scope:** `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, issue templates, PR template, architecture guide.
- **Technical Strategy:** Standard GitHub community health files; clear local setup instructions for Linux/macOS/Windows.
- **Prerequisites:** STEP 25 complete.
- **Expected Files:** `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `.github/ISSUE_TEMPLATE/`, `.github/PULL_REQUEST_TEMPLATE.md`.
- **Difficulty:** S
- **Acceptance Criteria:** All contributor guides exist and provide clear, step-by-step development setup instructions.
- **Required Verification:** Fresh clone setup test following `CONTRIBUTING.md`.
- **Definition of Done:** Repository community health files fully complete.
- **Dependencies:** STEP 25.

### STEP 27 — Cross-Platform CI & Packaging
- **Objective:** Configure GitHub Actions CI workflow to build and package desktop binaries for Linux, Windows, and macOS.
- **Scope:** CI matrix for `ubuntu-latest`, `windows-latest`, `macos-latest`; build AppImage, deb, msi, dmg.
- **Technical Strategy:** Tauri GitHub Action (`tauri-apps/tauri-action`); cache Cargo and npm dependencies.
- **Prerequisites:** STEP 26 complete.
- **Expected Files:** `.github/workflows/ci.yml`, `.github/workflows/release.yml`.
- **Difficulty:** L
- **Acceptance Criteria:** Automated CI builds successfully produce release artifacts across the build matrix.
- **Required Verification:** Test CI workflow run on GitHub.
- **Definition of Done:** Passing CI pipeline generating downloadable application bundles.
- **Dependencies:** STEP 26.

### STEP 28 — Installer & Release Verification
- **Objective:** Smoke test installer packages on clean environments.
- **Scope:** Install and run AppImage and `.deb` on clean Ubuntu; verify desktop launcher and file associations.
- **Technical Strategy:** Verification in clean container / VM environment.
- **Prerequisites:** STEP 27 complete.
- **Expected Files:** `docs/reports/RELEASE_VERIFICATION_REPORT.md`.
- **Difficulty:** M
- **Acceptance Criteria:** AppImage executes directly; deb installs and launches without missing dynamic library errors.
- **Required Verification:** Manual smoke testing on Ubuntu 24.04 LTS.
- **Definition of Done:** Verified working release packages ready for distribution.
- **Dependencies:** STEP 27.

---

## 9. Milestone 8 — Beta Launch (Days 29–30)

### STEP 29 — External Beta Testing & Critical Fixes
- **Objective:** Run focused beta testing with real creator video files and fix critical blockers.
- **Scope:** Ingest test footage from external creators (YouTube tutorials, Zoom recordings, podcasts); rapid triage.
- **Technical Strategy:** Strict bug triage; fix only release blockers; defer non-critical requests to P1.
- **Prerequisites:** STEP 28 complete.
- **Expected Files:** `docs/reports/BETA_FEEDBACK_LOG.md`.
- **Difficulty:** M
- **Acceptance Criteria:** Zero open release-blocking issues; no unhandled crashes reported.
- **Required Verification:** Testing 10 distinct real-world video files from diverse camera/screen sources.
- **Definition of Done:** Beta testing sign-off by lead architect.
- **Dependencies:** STEP 28.

### STEP 30 — Launch Materials & Showcase
- **Objective:** Produce visual documentation, demo GIF/video, and user documentation for the release.
- **Scope:** Screenshot showcase, 30-second silent video demo, quickstart tutorial in docs.
- **Technical Strategy:** High-resolution screenshots showing dark UI, waveform, and cut review in action.
- **Prerequisites:** STEP 29 complete.
- **Expected Files:** `docs/images/`, `README.md` updates.
- **Difficulty:** S
- **Acceptance Criteria:** Visual assets illustrate the complete rough-cut workflow clearly.
- **Required Verification:** Visual inspection of README rendering on GitHub.
- **Definition of Done:** Polished visual assets committed and linked in README.
- **Dependencies:** STEP 29.

### STEP 31 — GitHub Beta Release
- **Objective:** Tag and publish OpenTake `v0.1.0-beta` on GitHub Releases.
- **Scope:** Git tag `v0.1.0-beta`, release notes, checksums (SHA-256), release assets (AppImage, deb, msi, dmg).
- **Technical Strategy:** GitHub Releases via automated tag push workflow.
- **Prerequisites:** STEP 30 complete, all Release Gates passed.
- **Expected Files:** `CHANGELOG.md`, GitHub Release entry.
- **Difficulty:** S
- **Acceptance Criteria:** GitHub release is public, binaries are downloadable, checksums match.
- **Required Verification:** Download binary from GitHub Release, verify checksum, execute.
- **Definition of Done:** v0.1.0 Beta officially released to the public.
- **Dependencies:** STEP 30.

### STEP 32 — Feedback & Post-Release Roadmap
- **Objective:** Establish community feedback channels and initialize the post-beta development backlog.
- **Scope:** GitHub Discussions setup, issue triage rules, transition to P1 features (whisper.cpp transcription).
- **Technical Strategy:** Create milestone boards for v0.2.0; document contributor roadmap.
- **Prerequisites:** STEP 31 complete.
- **Expected Files:** `docs/roadmap/POST_BETA_ROADMAP.md`.
- **Difficulty:** XS
- **Acceptance Criteria:** Public roadmap published with clear P1 feature priorities.
- **Required Verification:** Verification of GitHub Discussions and issue templates.
- **Definition of Done:** Step 32 complete; initial beta release lifecycle concluded.
- **Dependencies:** STEP 31.

---

## 10. Critical Path Analysis

The critical path of the OpenTake 30-day development cycle is:
$$\text{STEP 1} \rightarrow \text{STEP 2} \rightarrow \text{STEP 3} \rightarrow \text{STEP 5} \rightarrow \text{STEP 6} \rightarrow \text{STEP 7} \rightarrow \text{STEP 8} \rightarrow \text{STEP 9} \rightarrow \text{STEP 10} \rightarrow \text{STEP 14} \rightarrow \text{STEP 15} \rightarrow \text{STEP 17} \rightarrow \text{STEP 18} \rightarrow \text{STEP 19} \rightarrow \text{STEP 21} \rightarrow \text{STEP 27} \rightarrow \text{STEP 31}$$

- Any delay on this chain directly impacts the beta release date.
- Parallelization opportunity: UI styling (STEP 4, STEP 22) and documentation (STEP 26, STEP 30) can be developed alongside engine steps when appropriate.

---

## 11. Release Gates (Go / No-Go Criteria)

OpenTake `v0.1.0-beta` will **NOT ship** if any of the following conditions exist:
1. **Source Corruption Risk:** Original media files can be modified, truncated, or renamed under any user action.
2. **Export Failure on Baseline Formats:** Valid MP4 (H.264/AAC) files consistently fail export on the primary platform.
3. **Severe A/V Desynchronization:** Audio and video drift across cuts exceeds $\pm 25\text{ ms}$ or exhibits audible clicks/pops.
4. **Unhandled Crashing:** Ingesting unsupported files or clicking Cancel causes unhandled application panics or window freezes.
5. **Orphaned Processes:** Cancelling jobs leaves runaway FFmpeg processes running in the background.
6. **Unresolved Licensing Violations:** Distribution of binaries without proper license text and source attribution.
