# OpenTake — STEP 6 Implementation Report
**Milestone:** 2 — Media Foundation  
**Step:** 6 of 32 — Secure File Import & Media Metadata Inspection  
**Status:** PASS  
**Target Platform:** Ubuntu Linux x86_64 (Wayland / GTK 3 / WebKitGTK 2.52)  
**Date:** October 14, 2026  

---

## A. Executive Summary

STEP 6 establishes OpenTake's secure, offline-first local media file ingestion and metadata inspection pipeline. It enables users to select local media files through native operating system dialogs or desktop drag-and-drop, validates candidate files against security and filesystem rules, safely inspects container and stream metadata using FFprobe with option-injection defense, and manages imported assets within the session Media Library.

Key deliverables completed:
1. **Native Local File Selection & Desktop Drag-and-Drop:**
   - Integrated native OS dialogs via `rfd` (GTK/XDG Portal) on Linux, ensuring candidate paths originate from explicit user selection.
   - Wired native window drag-and-drop event processing with real-time visual drop-target feedback.
2. **Secure Path Authorization & Validation (`import.rs`):**
   - Strictly enforces filesystem boundaries: verifies path existence, regular file classification, read access, canonicalization, rejection of directories/fifos/devices, zero-byte file rejection, and maximum size limits (500 GiB).
   - Source preservation guarantee: Media ingestion and library removal operations NEVER copy, rename, modify, or delete original files on disk.
3. **FFprobe Process Integration with Option-Injection Defense (`probe.rs`):**
   - Uses managed child process infrastructure (`run_managed_process`) with fixed arguments and a mandatory `--` option-injection guard.
   - Enforces 10-second inspection timeouts and bounded JSON output memory limits (2 MiB).
4. **Structured Normalized Media Models (`metadata.rs`):**
   - Normalized container, video, and audio stream domain models.
   - Preserves exact rational fractions (`Rational { num, den }`) for average frame rate, nominal frame rate, and stream timebases (e.g. `30000/1001`, `24/1`).
   - Flags suspect Variable Frame Rate (VFR) media through frame rate discrepancies.
   - Extracts orientation and rotation degrees from metadata tags and display matrix side-data.
5. **Media Compatibility Classification (`compatibility.rs`):**
   - Evaluates media readiness (`Supported`, `SupportedWithLimitations`, `Unsupported`, `Invalid`).
   - Distinguishes inspectable media from future editing readiness.
6. **Session-Level Media Library & Queue Control (`identity.rs` & `import.rs`):**
   - Manages imported assets in an in-memory session ledger (`MediaLibrarySession`).
   - Generates stable identity fingerprints based on canonical path, size, and mtime.
   - Detects and rejects duplicate imports cleanly.
   - Controls concurrency with a maximum of 2 simultaneous FFprobe probe jobs.
7. **Production-Ready Media Library Interface:**
   - Replaced Step 6 placeholder with live asset list, item count, selection inspector, technical metadata details modal, and asset removal.
8. **Strict Roadmap Scope Discipline:**
   - Zero video playback, zero waveform rendering, zero timeline editing, zero silence detection, and zero export functionality was introduced.

---

## B. Starting Repository State

- **Baseline Commit:** `3915f66` (`feat(media): implement secure ffmpeg and ffprobe process infrastructure`), pushed to `origin/main`.
- **Pre-existing Subsystems:** Tauri 2 desktop shell, React 19 UI, Tailwind CSS v4 design system, `MediaEngineService` discovery and diagnostic process management.

---

## C. Architecture Changes

### 1. Headless Core (`crates/opentake-core/src/media/`)
- `metadata.rs`: Exact rational fractions, domain container/video/audio structures, and FFprobe JSON normalization.
- `compatibility.rs`: Media compatibility evaluator distinguishing container, video, and audio codecs.
- `identity.rs`: Source identity provenance, SHA-256 fingerprinting, and `MediaAsset` session model.
- `probe.rs`: Safe FFprobe runner with `--` argument protection, bounded buffers, and cancellation support.
- `import.rs`: Filesystem path validator, duplicate detector, concurrency semaphore (limit 2), and session asset store (`MediaLibrarySession`).
- `error.rs`: Added domain error codes (`FILE_NOT_FOUND`, `NOT_A_REGULAR_FILE`, `EMPTY_FILE`, `FILE_TOO_LARGE`, `INVALID_PATH`, `CORRUPT_MEDIA`, `NO_STREAMS_FOUND`, `DUPLICATE_FILE`).

### 2. Tauri Desktop IPC Adapter (`src-tauri/src/commands/`)
- `media_import.rs`: Exposes `pick_and_import_files`, `import_files_by_path`, `get_media_library_assets`, `remove_media_library_asset`, and `clear_media_library`.
- `lib.rs`: Registers `MediaLibraryState` in the Tauri application builder.

### 3. Frontend Architecture (`src/`)
- `src/types/media.ts`: TypeScript contracts mirroring Rust models (`Rational`, `MediaMetadata`, `MediaAsset`, `ImportBatchResult`, etc.).
- `src/lib/mediaImport.ts`: Typed IPC invokers.
- `src/components/editor/MediaLibraryPanel.tsx`: Production Media Library panel with native picker initiation, drop-zone overlay, live asset list, item badge, and removal actions.
- `src/components/editor/MediaDetailsModal.tsx`: Technical metadata modal presenting container, stream, and provenance details.

---

## D. File Authorization & Security Boundary

1. **Native Dialog Provenance:** File picking uses `rfd::AsyncFileDialog` directly inside the Rust backend. The dialog runs via the OS file chooser portal / GTK dialog, returning genuine system paths.
2. **Untrusted Path Handling:** All candidate paths (from dialog or drag-and-drop) are treated as untrusted strings:
   - Must exist on the filesystem.
   - Must canonicalize cleanly via `fs::canonicalize` (resolving symlinks and mitigating traversal tricks).
   - Must be verified as regular files (rejects directories, sockets, devices, and FIFOs).
   - Must be readable via explicit file descriptor opening.
   - Empty files (0 bytes) are rejected immediately.
   - Enforces a 500 GiB ceiling to guard against resource exhaustion attacks.
3. **Option Injection Defense:** When invoking FFprobe, an explicit `--` delimiter precedes the target path:
   ```rust
   let args = vec![
       "-v".to_string(),
       "error".to_string(),
       "-print_format".to_string(),
       "json".to_string(),
       "-show_format".to_string(),
       "-show_streams".to_string(),
       "--".to_string(), // Option injection guard
       path_str,
   ];
   ```
   This guarantees that filenames beginning with a hyphen (e.g. `-v`, `--help`, `-dash_name.mp4`) cannot be interpreted as command-line flags.

---

## E. FFprobe Process Implementation

- **Executable Resolution:** Dynamically acquired through validated system candidate path or explicit configuration.
- **Timeout:** Enforced 10-second maximum execution timeout via `run_managed_process`.
- **Memory Bounding:** Maximum standard output memory buffer is capped at 2 MiB; standard error buffer capped at 256 KiB.
- **Process Group Isolation:** Process group isolation (`process_group(0)`) ensures complete process and descendant termination upon cancellation or timeout without zombie creation.
- **JSON Normalization:** Numeric string parsing safely extracts integer microseconds (`duration * 1,000,000`) and signed start times without floating-point drift in domain models.

---

## F. Metadata & Rational Models

- **Rational Timebases:** Rational fractions are held as `{ num: u32, den: u32 }`. String formats such as `30000/1001` or `24/1` are preserved exactly. Fractions with zero denominators (`0/0`, `0/1`, `10/0`) are rejected or parsed as `None`.
- **Display Conversions:** Rational fractions provide a `to_f64_lossy()` helper strictly for UI presentation (e.g. `29.97 fps`), never for editing timecode calculations.
- **VFR Detection:** Media whose average frame rate differs from its nominal frame rate by more than 0.05 fps is automatically flagged with `is_vfr_suspect = true`.

---

## G. Import Queue & Concurrency

- **Concurrency Limit:** Regulated by a Tokio `Semaphore` set to `MAX_CONCURRENT_PROBE_JOBS = 2`, preventing subprocess storming and CPU saturation during bulk imports.
- **Duplicate Detection:** Compares canonical path and file size against assets in `MediaLibrarySession`. Repeated requests are rejected with `DUPLICATE_FILE` and reported in the UI notice banner.
- **Cancellation:** Supports Tokio `CancellationToken`. Cancelled probes abort the subprocess group cleanly and do not register partial assets.

---

## H. User Experience & Media Library UI

- **Import Media Button:** Prominently placed in the empty state and panel header.
- **Drag & Drop:** Displays an active blue/zinc dashed overlay when dragging video files over the workspace.
- **Asset Cards:** Displays filename, duration, file size, video resolution, video codec, frame rate, audio codec, and compatibility badge (`Ready`, `Limitation`, `Unsupported`).
- **Metadata Inspector:** "Info" icon opens the detailed metadata inspector showing stream breakdowns, timebases, rotation angle, and compatibility warnings.
- **Removal:** Clicking the trash icon removes the asset from the current session with clear confirmation that original disk files remain untouched.

---

## I. Security Assessment

| Concern | Mitigation Strategy | Status |
| :--- | :--- | :--- |
| **Path Traversal** | Canonicalized with `fs::canonicalize`, checked for regular file | VERIFIED |
| **Option Injection** | Passed via direct argument vector with `--` flag before path | VERIFIED |
| **Shell Injection** | No shell strings or `/bin/sh` invocation; direct `tokio::process::Command` | VERIFIED |
| **Symlink Loop / Broken** | Handled by canonicalize and metadata existence checks | VERIFIED |
| **0-byte / Corrupt Media** | Rejected with `EMPTY_FILE` or non-zero FFprobe exit handling | VERIFIED |
| **Denial of Service (OOM)** | 2 MiB bounded stdout buffer, 500 GiB file ceiling, max 2 concurrent probes | VERIFIED |
| **Data Loss / Modification** | Media import and removal strictly read-only; disk deletion impossible | VERIFIED |

---

## J. Changed Files

### Core Domain Library (`crates/opentake-core/`)
- `Cargo.toml`: Added `sha2` dependency, `tempfile` dev-dependency.
- `src/media/mod.rs`: Re-exported media submodules and public types.
- `src/media/error.rs`: Added domain error variants for file validation and import.
- `src/media/metadata.rs`: Implemented `Rational`, `MediaMetadata`, and FFprobe parser.
- `src/media/compatibility.rs`: Implemented `MediaCompatibility` and status evaluators.
- `src/media/identity.rs`: Implemented `SourceIdentity` and `MediaAsset`.
- `src/media/probe.rs`: Implemented `probe_media_file` with option-injection defense.
- `src/media/import.rs`: Implemented path validation and `MediaLibrarySession`.
- `tests/media_import_integration.rs`: Synthetic media integration tests.

### Tauri Desktop Application (`src-tauri/`)
- `Cargo.toml`: Added `rfd = "0.15"` for native OS dialogs.
- `src/commands/media_import.rs`: Desktop IPC commands for file picking, drop, and library management.
- `src/commands/mod.rs`: Re-exported media import commands.
- `src/lib.rs`: Registered `MediaLibraryState` and IPC handlers.

### React Application (`src/`)
- `src/types/media.ts`: Domain media types and formatting helpers.
- `src/lib/mediaImport.ts`: IPC invocation wrappers.
- `src/lib/mediaImport.test.ts`: Unit tests for formatting and IPC fallback.
- `src/components/ui/Icons.tsx`: Added `TrashIcon` and `UploadIcon`.
- `src/components/editor/MediaLibraryPanel.tsx`: Production Media Library panel.
- `src/components/editor/MediaDetailsModal.tsx`: Technical metadata modal.
- `package.json`: Updated test script to run all unit tests.

---

## K. Verification Matrix

| Verification Check | Method | Result | Notes |
| :--- | :--- | :--- | :--- |
| **Rust Code Formatting** | `cargo fmt --all -- --check` | **PASS** | 0 formatting issues across workspace |
| **Rust Type Checking** | `cargo check --workspace` | **PASS** | 0 warnings, 0 errors |
| **Core Unit Tests** | `cargo test -p opentake-core` | **PASS** | 29 tests passed |
| **Desktop Unit Tests** | `cargo test -p opentake-desktop` | **PASS** | 6 tests passed |
| **Synthetic Integration Tests** | `cargo test --test media_import_integration` | **PASS** | 7 tests passed (H.264, hyphens, unicode, duplicates, cancel) |
| **Clippy Linting** | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** | 0 warnings across all targets |
| **TypeScript Type Checking**| `npm run typecheck` | **PASS** | `tsc --noEmit` passed with 0 errors |
| **Frontend Linting** | `npm run lint` | **PASS** | ESLint 9 passed with 0 errors |
| **Frontend Production Build**| `npm run build` | **PASS** | Vite production bundle built in 1.04s |
| **Frontend Unit Tests** | `npm test` | **PASS** | 11 tests passed across preferences, engine, and import |
| **Native Ubuntu Runtime** | `target/debug/opentake-desktop` | **PASS** | Desktop window connects cleanly on Wayland |
| **Diagnostics Regression** | Settings Dialog & `get_media_engine_status` | **PASS** | Media engine status discovery unaffected |

---

## L. Performance Findings

- **Probe Execution Speed:** Synthetic 320x240 H.264/AAC MP4 media probed in under 40 milliseconds on the Ubuntu host.
- **Synthetic Integration Test Suite:** All 7 integration tests (generating synthetic MP4s and verifying metadata) completed in 0.16s total.
- **Memory Overhead:** In-flight metadata buffer strictly bounded to 2 MiB per probe process; session asset records consume negligible memory (< 1 KiB per asset).

---

## M. Known Limitations & Security Bounds

1. **VFR Heuristic Limitation:** FFprobe container and stream headers only report average and nominal frame rates. True VFR detection requires packet/frame PTS inspection, scheduled for Milestone 3 analysis jobs.
2. **Native Drag & Drop OS Permissions:** While native dialogs operate through GTK / XDG Desktop Portals with narrow capability grants, dropped files rely on application-level path validation and canonicalization.
3. **No Direct Preview in Step 6:** Video playback and preview seeking are intentionally locked out until Step 7.

---

## N. Git Status

- **Working Tree:** Clean and verified.
- **Commit:** `feat(media): implement secure media import and ffprobe metadata inspection`
- **Push Policy:** Preserved local commit; remote push deferred to user.

---

## O. STEP 7 Handoff

### Destination: STEP 7 — Video Playback & Seeking Foundation
Step 7 will consume the artifacts produced in Step 6:
1. `MediaAsset.id`: Unique immutable handle for linking active preview monitors.
2. `MediaAsset.source.canonicalPath`: Authorized media source for the playback engine or custom Tauri asset protocol.
3. `MediaAsset.metadata.videoStreams`: Video width, height, aspect ratio, frame rate, and rotation degrees for monitor scaling and orientation correction.
4. `MediaAsset.metadata.audioStreams`: Audio channel count and sample rate for audio synchronization.
5. `MediaAsset.compatibility`: Guarantees preview engine only attempts playback on inspectable, supported media.

---

## P. Final Status

**PASS**  
All Step 6 objectives, security constraints, acceptance criteria (AC-06-01 through AC-06-19), and quality gates are completely satisfied.
