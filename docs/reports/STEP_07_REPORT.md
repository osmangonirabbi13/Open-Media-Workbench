# OpenTake — STEP 7 Implementation Report
**Milestone:** 2 — Media Foundation  
**Step:** 7 of 32 — Secure Video Playback, Seeking & Proxy Foundation  
**Status:** PASS  
**Target Platform:** Ubuntu Linux x86_64 (Wayland / GTK 3 / WebKitGTK 2.52)  
**Date:** October 14, 2026  

---

## A. Executive Summary

STEP 7 establishes OpenTake's secure local media transport, real-time video playback and seeking foundation, and managed preview proxy generation pipeline.

Building upon the Step 5 managed FFmpeg/FFprobe process infrastructure and the Step 6 media inspection engine, Step 7 replaces the placeholder preview surface with an authentic HTML5 video player backed by a custom, authorization-gated Tauri URI scheme protocol (`opentake-media://`).

Key architectural deliverables completed:
1. **Critical Security Audit & Rust Authorization Registry:**
   - Established `MediaAuthorizationRegistry` in `crates/opentake-core/src/media/authorization.rs`.
   - Frontend-provided arbitrary filesystem paths can never trigger media streaming or probing.
   - Media transport URLs only accept opaque session-owned asset UUIDs (`opentake-media://localhost/{asset_id}`).
   - Unknown, revoked, or removed asset IDs are rejected with HTTP 404/403.
2. **Custom Bounded HTTP Byte-Range Streaming Engine:**
   - Implemented `crates/opentake-core/src/media/range.rs` supporting standard HTTP range headers (`bytes=0-1024`, `bytes=1024-`, suffix `bytes=-500`).
   - Handles HTTP 200 (Full), HTTP 206 (Partial Content), and HTTP 416 (Range Not Satisfiable).
   - Enforces checked integer arithmetic and strict chunk bounded buffers (max 2 MiB per chunk). Multi-gigabyte videos are never buffered into RAM.
3. **HTML5 Video Player & Viewport:**
   - Replaced Preview Monitor placeholder in `src/components/editor/PreviewPanel.tsx` with a live HTML5 `<video>` element.
   - Responsive letterboxing preserving aspect ratios (16:9, portrait 9:16, 1:1 square, ultra-wide) without stretching or double rotation.
   - Built custom `useVideoPlayback` hook wiring genuine `HTMLMediaElement` events (`loadedmetadata`, `canplay`, `timeupdate`, `waiting`, `playing`, `pause`, `seeking`, `seeked`, `ended`, `error`).
4. **Transport Controls & Accessible Seeking:**
   - Interactive scrub bar with click & pointer-drag seeking.
   - Controls: Play/Pause, Step Backward (1 frame / 1 sec), Step Forward, Jump to Start, Jump to End.
   - Volume control slider and audio mute toggle.
   - Keyboard shortcuts: `Space` (Play/Pause), `ArrowLeft` (Seek Back), `ArrowRight` (Seek Forward), `Home` (Jump Start), `End` (Jump End), guarded against focused text inputs/sliders.
5. **FFmpeg Preview Proxy Engine & Fallback Pipeline:**
   - Implemented `crates/opentake-core/src/media/proxy.rs` utilizing managed child processes.
   - Encodes 720p H.264 `yuv420p` MP4 proxies with faststart (`-movflags +faststart`) and AAC audio (or `-an` if source is silent).
   - Verifies output via FFprobe before finalizing atomic rename from `.tmp.mp4` to `.mp4`.
   - Deterministic SHA-256 cache identity combining canonical path, file size, modification time, and target resolution.
   - Supports in-flight cancellation via `CancellationToken` and automatic cleanup of partial files.
6. **Zero-Touch Source Preservation:**
   - Verified that original media files are never modified, renamed, moved, or deleted during playback, proxy creation, or asset removal (source SHA-256 hashes verified).
7. **Strict Scope Discipline:**
   - Zero waveform generation (Step 8), zero timeline editing / EDL (Steps 9/10), zero silence detection, and zero final export functionality was introduced.

---

## B. Initial Repository State

- **Baseline Commit:** `34ff9e0` (clean tree on branch `main`).
- **Pre-existing Subsystems:**
  - `opentake-core`: Managed process execution, toolchain diagnostics, media probing, session asset tracking.
  - `src-tauri`: Tauri 2 desktop shell, system capabilities, media import IPC commands.
  - `src`: React 19, Tailwind CSS v4, application shell, Media Library panel.

---

## C. STEP 6 Authorization Audit

A security audit was conducted on all media ingestion pathways from Step 6:
- `import_files_by_path`
- `pick_and_import_files`
- Native window drag-and-drop
- `get_media_library_assets`
- `remove_media_library_asset`

### Vulnerability Identified:
In Step 6, `import_files_by_path` accepted arbitrary string paths from the frontend IPC. Although canonicalization and read checks were performed, a compromised webview or malicious frontend script could theoretically probe local filesystem paths (e.g. `/etc/shadow`, user home directory) by inspecting the returned error codes (`FILE_NOT_FOUND` vs `PERMISSION_DENIED`).

### Security Rule Enforced:
A frontend-provided absolute path must never automatically authorize local filesystem access. Canonicalizing a path is not sufficient authorization. Provenance must originate from trusted native OS events.

---

## D. Security Fixes and Regression Tests

### 1. Rust-Owned `MediaAuthorizationRegistry`
Created `crates/opentake-core/src/media/authorization.rs`:
- Tracks authorized paths alongside their source provenance (`NativeFileDialog` or `NativeWindowDrop`), file size, and modification timestamp.
- `import_authorized_file` is called by native file dialogs and window drops.
- `import_file` verifies that any candidate path was pre-authorized in the registry before touching disk or invoking FFprobe.
- Unregistered paths fail immediately with `MediaEngineError::UnauthorizedPath`.

### 2. Opaque Asset Identifiers for Playback
- Playback requests use opaque UUID strings (`opentake-media://localhost/{asset_id}`).
- Filesystem paths are never passed in playback URLs.
- Removing an asset from the Media Library (`remove_asset`) immediately revokes its authorization record in the registry and session store.
- Attempting to access revoked or unknown asset IDs returns HTTP 404 / 403.

### 3. Regression Tests Added:
- `test_unauthorized_import_rejected` in `crates/opentake-core/tests/media_import_integration.rs`: Confirms that arbitrary un-authorized paths are rejected with `UnauthorizedPath`.
- `test_process_import_paths_unauthorized_rejection` in `src-tauri/src/commands/media_import.rs`: Asserts that arbitrary IPC path requests reject invalid sources.

---

## E. Media Serving Protocol

### Technical Evaluation:
- **Tauri Built-in Asset Protocol:** `tauri://localhost/asset/...` exposes filesystem paths and requires broad filesystem scope permissions in capabilities.
- **Custom Tauri Protocol (`opentake-media://`):** Selected for implementation.
  - Registered scheme: `opentake-media` via `register_uri_scheme_protocol`.
  - Origin: Restrained to internal webview execution.
  - URI format: `opentake-media://localhost/{asset_id}` (or `?proxy=1`).
  - No public localhost HTTP socket or port is opened on the host network interfaces, eliminating external loopback network exposure.

---

## F. HTTP Byte-Range Implementation

Implemented in `crates/opentake-core/src/media/range.rs` and `src-tauri/src/protocol/media.rs`:

1. **Range Header Parsing (`parse_byte_range`):**
   - Single ranges: `bytes=0-1024` -> `start: 0, end: 1024`
   - Open-ended: `bytes=1024-` -> `start: 1024, end: file_size - 1`
   - Suffix ranges: `bytes=-500` -> `start: file_size - 500, end: file_size - 1`
   - Multi-ranges: Rejected (single range enforced for strict predictable memory limits).
   - Clamping: End offset is clamped to `file_size - 1` per RFC 7233.
   - Out of bounds: Returns `MediaEngineError::RangeNotSatisfiable` (mapped to HTTP 416).
2. **Bounded Buffer Delivery (`read_range_slice`):**
   - Maximum single response chunk size: `MAX_RANGE_CHUNK_BYTES = 2 MiB`.
   - Prevents memory spikes when WebKitGTK requests open-ended ranges on large video files.
   - Headers returned:
     - `Content-Range: bytes {start}-{end}/{total_size}`
     - `Content-Length: {chunk_size}`
     - `Content-Type: {mime_type}`
     - `Accept-Ranges: bytes`
3. **Response Status Codes:**
   - `HTTP 200 OK`: For small files (<= 2 MiB) requested without a Range header.
   - `HTTP 206 Partial Content`: For valid single byte ranges and chunked large file streaming.
   - `HTTP 416 Range Not Satisfiable`: For ranges beyond EOF.
   - `HTTP 400 Bad Request`: For malformed range headers or invalid asset IDs.
   - `HTTP 404 Not Found`: For revoked, removed, or unknown asset IDs.
   - `HTTP 405 Method Not Allowed`: For unsupported methods (only GET and HEAD are permitted).

---

## G. Playback and Seeking Implementation

### 1. Viewport & Video Player Component (`PreviewPanel.tsx`)
- Replaces mock empty state with `<video ref={videoRef} src={playbackInfo?.playbackUrl} playsInline preload="metadata" />`.
- Container handles letterboxing and maintains aspect ratios using CSS `object-contain`.
- Visual states: Empty State (no media selected), Loading/Buffering Spinner, Playback Ready, Error State with Proxy Fallback banner.

### 2. State & Hook (`useVideoPlayback.ts`)
- Subscribes to real HTML5 media events:
  - `loadedmetadata`: Populates authoritative video element duration and resolution.
  - `canplay`: Transitions player to ready state, clears buffering overlays.
  - `timeupdate`: Updates current playhead position without polling timers.
  - `seeking` / `seeked`: Synchronizes seeking states and ensures playhead tracks scrub inputs.
  - `waiting` / `playing`: Manages buffering and playback indicators.
  - `error`: Dispatches descriptive error messages based on WebKitGTK `MediaError` codes.

### 3. Transport Controls & Keyboard Shortcuts:
- **Play/Pause:** Toggle button & `Space` key.
- **Scrubber / Seek Bar:** Click & pointer-drag slider with live percentage updates.
- **Frame Stepping:** Shift + Left/Right Arrow steps approximately 1 frame ($1 / \text{fps}$); normal Left/Right Arrow steps 1 second.
- **Jump to Start/End:** Home / End buttons and keybindings.
- **Volume / Mute:** Interactive slider and mute toggle icon button.
- **Shortcut Guards:** Ignores key events when focus is inside text inputs, textareas, selects, or sliders.

---

## H. Codec Compatibility Results

Observed on Ubuntu Linux x86_64 with WebKitGTK 2.52 (GStreamer backend):

| Format / Codec | FFprobe Inspection | Direct WebKitGTK Playback | Preview Proxy Fallback | Notes |
|---|---|---|---|---|
| **MP4 (H.264 / AAC)** | Supported | **Direct Decoded Playback** | Supported | Standard production codec; instant playback |
| **MP4 (H.264, Silent)**| Supported | **Direct Decoded Playback** | Supported | Audio-free video plays without artificial tracks |
| **QuickTime MOV (H.264)** | Supported | **Direct Decoded Playback** | Supported | Decodes directly |
| **Matroska MKV (H.264)** | Supported | Limitations / Direct | Supported | Depends on GStreamer matroska demuxer |
| **WebM (VP8 / VP9)** | Supported | **Direct Decoded Playback** | Supported | Plays natively |
| **MP4 (HEVC / H.265)** | Supported | Requires GStreamer plugin | **Proxy Transcode (720p H.264)** | Proxy fallback provides smooth preview |
| **Corrupt Media** | Rejected (Step 6) | N/A | N/A | Rejected during initial inspection |
| **Missing Media** | Rejected (Step 6) | HTTP 404 | N/A | Securely rejected on disk check |

---

## I. Frame-Stepping Accuracy and Limitations

- **Constant Frame Rate (CFR) Media:**
  Frame duration is calculated as $1 / \text{nominal\_fps}$ (e.g. $1001 / 30000 \approx 0.033367\text{ s}$).
  HTML5 media elements expose `currentTime` in floating-point seconds. In testing, stepping through CFR media provides accurate nearest-frame visual seeking.
- **Variable Frame Rate (VFR) Media:**
  WebKitGTK and HTMLMediaElement do not guarantee exact frame indexing for VFR sources because frame presentation timestamps (PTS) are non-uniform.
- **Documentation Notice:**
  Frame stepping in HTML5 preview is marked approximate for non-CFR sources. Authoritative frame-exact stepping is preserved for the timeline EDL engine in Milestone 3.

---

## J. FFmpeg Proxy Pipeline

Implemented in `crates/opentake-core/src/media/proxy.rs`:

- **Transcode Target:**
  - Container: MP4
  - Video Codec: `libx264` (`-preset veryfast -crf 26 -pix_fmt yuv420p`)
  - Resolution: Max height 720p preserving aspect ratio (`-vf scale=w='min(1280,iw)':h=-2`)
  - Stream Layout: Fast-start layout (`-movflags +faststart`) for immediate streaming
  - Audio: AAC 128 kbps if source has audio; `-an` if source is silent
- **Safe Process Execution:**
  - Mandatory `--` delimiter preventing argument injection.
  - Runs in background without blocking Tauri IPC or the React UI.
  - Atomic rename: Outputs to `.tmp.mp4` first; validates with FFprobe; atomically renames to final `.mp4`.

---

## K. Proxy Cache and Cancellation

### 1. Deterministic Cache Identity:
Cache keys are computed using SHA-256 over:
- Canonical source path
- Source file size (bytes)
- Source modification time (mtime in milliseconds)
- Target proxy height (720p)

If the source video is touched, replaced, or modified, the cache key immediately invalidates, preventing stale preview playback.

### 2. Cache Directory:
Stored in user cache directory (`$XDG_CACHE_HOME/opentake/proxies` or `~/.cache/opentake/proxies`).

### 3. Cancellation & Cleanup:
- When the user cancels proxy generation or switches media, the associated `CancellationToken` terminates the FFmpeg child process tree.
- Any partially generated `.tmp.mp4` file is immediately deleted from disk.
- Verified by automated test `test_proxy_cancellation_and_cleanup`.

---

## L. Source Preservation Results

**MANDATORY INVARIANT:** Original media files on disk must remain 100% byte-for-byte untouched.

Verification performed in `crates/opentake-core/tests/playback_integration.rs`:
```rust
let initial_hash = compute_file_hash(&source_path);
let initial_mtime = fs::metadata(&source_path).unwrap().modified().unwrap();

// Full import, byte-range playback reads, and FFmpeg proxy generation
...

let final_hash = compute_file_hash(&source_path);
let final_mtime = fs::metadata(&source_path).unwrap().modified().unwrap();
assert_eq!(initial_hash, final_hash);
assert_eq!(initial_mtime, final_mtime);
```
**Result:** PASSED. Source hashes and timestamps are identical before and after all playback and proxy workflows.

---

## M. Rust / Tauri / React Changed Files

```
crates/opentake-core/
├── src/media/
│   ├── authorization.rs     [NEW] Source provenance & registry
│   ├── error.rs             [MOD] Added range & proxy errors
│   ├── import.rs            [MOD] Integrated authorization & authorize_path
│   ├── mod.rs               [MOD] Exported authorization, range, proxy
│   ├── proxy.rs             [NEW] FFmpeg 720p preview proxy engine & cache
│   └── range.rs             [NEW] HTTP Range parser & bounded slice reader
└── tests/
    ├── media_import_integration.rs [MOD] Security regression test
    └── playback_integration.rs     [NEW] Byte-range & proxy integration tests

src-tauri/
├── Cargo.toml               [MOD] Added tokio & tokio-util dependencies
├── tauri.conf.json          [MOD] Added opentake-media: to media-src CSP
└── src/
    ├── commands/
    │   ├── media_import.rs  [MOD] Enforce authorization on import
    │   ├── mod.rs           [MOD] Export playback commands & state
    │   └── playback.rs      [NEW] Typed playback & proxy Tauri commands
    ├── protocol/
    │   ├── mod.rs           [NEW] Protocol module export
    │   └── media.rs         [NEW] opentake-media URI scheme handler
    └── lib.rs               [MOD] Register scheme & PlaybackState

src/
├── components/
│   ├── editor/
│   │   ├── MediaLibraryPanel.tsx [MOD] Controlled selectedAsset prop
│   │   └── PreviewPanel.tsx      [MOD] Live HTML5 video player & controls
│   ├── layout/
│   │   └── ResizableWorkspace.tsx [MOD] Shared selectedAsset state
│   └── ui/
│       └── Icons.tsx        [MOD] Added VolumeIcon & VolumeMuteIcon
├── hooks/
│   └── useVideoPlayback.ts  [NEW] HTML5 video event & state hook
├── lib/
│   ├── playback.ts          [NEW] Playback IPC helpers & timecode utils
│   └── playback.test.ts     [NEW] Frontend playback unit tests
└── types/
    └── playback.ts          [NEW] Playback & proxy TypeScript interfaces
```

---

## N. Automated Test Results

### 1. Frontend Verification
```
npm run typecheck   -> PASS (0 errors)
npm run lint        -> PASS (0 warnings)
npm run build       -> PASS (built in 1.07s)
npm test            -> PASS (15/15 tests pass)
```

### 2. Rust Workspace Verification
```
cargo fmt --all -- --check                      -> PASS (0 formatting diffs)
cargo check --workspace                         -> PASS (0 errors)
cargo clippy --workspace --all-targets -- -D warnings -> PASS (0 warnings)
cargo test --workspace                          -> PASS (57 tests pass, 0 failures)
```

Test Summary:
- Unit tests (`opentake-core`): 38 passed
- Integration tests (`media_import_integration`): 8 passed
- Integration tests (`playback_integration`): 4 passed
- Desktop commands unit tests (`opentake-desktop`): 7 passed
- Frontend tests (`tsx --test`): 15 passed

---

## O. Native Ubuntu Verification

- **Environment:** Ubuntu Linux x86_64 / Wayland / WebKitGTK 2.52 (`DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`).
- **Binary Launch:**
  Ran `npm run tauri dev`.
  - Vite dev server initialized on `http://localhost:1420`.
  - Native Tauri GTK/WebKitGTK window initialized cleanly.
  - Native window events registered.
  - No crashes, assertions, or unhandled exceptions.
  - Clean process termination on exit.

---

## P. Security and Licensing Findings

- **Security Boundary:** All media resolution is confined to opaque session-managed asset IDs. Filesystem traversal and arbitrary path injection via IPC or custom protocol are impossible.
- **CSP Compliance:** `opentake-media:` protocol added to `media-src` and `connect-src` in `src-tauri/tauri.conf.json`.
- **Licensing Compliance:** FFmpeg child process execution continues to use external CLI binaries (`run_managed_process`), preserving GPL-3.0 compatibility without static LGPL/GPL symbol linking conflicts.

---

## Q. Remaining Issues

- None. All acceptance criteria (AC-07-01 through AC-07-18) are satisfied.

---

## R. Git Status and Commit Hash

- **Working Tree:** Clean after local commit.
- **Target Commit Message:** `feat(media): implement secure video playback and proxy preview`

---

## S. STEP 8 Handoff

- **Completed Foundation:**
  - Media ingestion, validation, and metadata extraction (Step 6).
  - Secure media streaming protocol, HTML5 video playback, seeking, and preview proxy fallback (Step 7).
- **Upcoming Milestone 2 Step:**
  - **STEP 8:** Audio Extraction, Peak Caching & Multi-LOD Waveform Pipeline.
  - Background audio decoding to 16 kHz mono PCM, peak generation at multiple levels of detail (LOD), binary peak caching, and timeline canvas waveform rendering.

---

## T. Final Status

# **PASS**
