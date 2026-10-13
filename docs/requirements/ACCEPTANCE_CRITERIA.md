# OpenTake — Acceptance Criteria & Verification Baseline

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Baseline Locked  
**Author:** OpenTake Core Architecture & QA Lead  
**Date:** 2026-10-13  

---

## 1. Acceptance Criteria Framework

Every P0 feature must satisfy clear, measurable, and testable acceptance criteria prior to release certification. Each specification follows the structured format:
- **Requirement ID & Title**
- **Description**
- **Preconditions**
- **Action**
- **Expected Result**
- **Error Behavior**
- **Verification Method**
- **Release-Blocker Classification (Blocker vs Non-Blocker)**

---

## 2. Media Ingestion & Inspection

### REQ-MEDIA-001: Media Ingestion via Drag-and-Drop and System File Picker
- **Description:** The user must be able to load local video files using either window drag-and-drop or the native system file dialog.
- **Preconditions:** Application is launched and idle in the ready/dashboard state.
- **Action:** User drags an MP4 file onto the drop zone or clicks "Open Video" and selects a valid file.
- **Expected Result:** Application accepts the file path, initiates FFprobe inspection, transitions UI from empty state to project workspace, and loads the video within 1.0 second.
- **Error Behavior:** If an invalid file or non-media file is dropped (e.g., `.txt`, `.pdf`), drop zone rejects the file with visual feedback and displays an inline toast: "Unsupported file type."
- **Verification Method:** Manual desktop UI smoke test + automated Tauri IPC command test passing file paths.
- **Release Blocker:** YES.

### REQ-MEDIA-002: Stream Metadata Extraction & Validation
- **Description:** Application extracts and validates technical metadata for imported media.
- **Preconditions:** A supported local video file is selected.
- **Action:** OpenTake runs FFprobe via the Rust backend.
- **Expected Result:** Duration (ms), container format, video resolution (width, height), video codec, framerate (fps), audio sample rate (Hz), and channel count are parsed into structured JSON and displayed in the media info header.
- **Error Behavior:** If metadata probing fails or takes longer than 5 seconds, the operation times out with an error dialog: "Unable to read media metadata."
- **Verification Method:** Integration test against synthetic fixture videos (720p, 1080p, 24fps, 30fps, 60fps).
- **Release Blocker:** YES.

### REQ-MEDIA-003: Corrupted or Unsupported Media Handling
- **Description:** Graceful rejection of corrupted video files or unsupported legacy containers.
- **Preconditions:** User attempts to import a truncated video file (0 bytes or missing moov atom) or unsupported format (e.g., proprietary RAW, AVI with Indeo codec).
- **Action:** Ingestion pipeline attempts probe.
- **Expected Result:** Probe error is intercepted in Rust. A modal dialog informs the user: "Unsupported or Corrupted Media File: [Reason]. OpenTake v0.1.0 Beta supports MP4, MOV, and MKV with standard H.264/AAC streams." The application returns to a clean ready state without crashing.
- **Error Behavior:** Application must never panic, crash, or freeze the UI thread.
- **Verification Method:** Automated test passing corrupted binary blobs and zero-byte files to `probe_media()`.
- **Release Blocker:** YES.

---

## 3. Playback & Transport

### REQ-PLAY-001: Preview Playback & Transport Controls
- **Description:** Synchronized preview video playback with standard transport controls.
- **Preconditions:** A valid video file is loaded into the project.
- **Action:** User clicks Play/Pause or presses the `Space` bar.
- **Expected Result:** Video preview toggles smoothly between playing and paused states. Audio plays back synchronously. Timecode updates continuously.
- **Error Behavior:** If HTML5 video encounters a decode error, an inline banner offers: "Preview decode error. Click here to generate a fast preview proxy."
- **Verification Method:** End-to-end desktop UI verification verifying play/pause state transitions and timecode progression.
- **Release Blocker:** YES.

### REQ-PLAY-002: Timeline Scrubbing & Frame-Accurate Seeking
- **Description:** Scrubbing the playhead updates the preview frame with low latency.
- **Preconditions:** Video is loaded and paused.
- **Action:** User drags the playhead along the timeline ruler.
- **Expected Result:** Preview viewport updates video frames synchronously with playhead dragging. Frame latency must be $\le 150\text{ ms}$ on standard 1080p H.264 video.
- **Error Behavior:** If seeking stalls, the player maintains the last decoded frame until the seek completes without blanking the screen.
- **Verification Method:** Latency benchmarking script measuring time delta between scrub event dispatch and `<video>` `seeked` event.
- **Release Blocker:** YES.

### REQ-PLAY-003: Fallback Preview Proxy Generation
- **Description:** Automatic or user-prompted proxy generation when native WebView decoding is unsupported.
- **Preconditions:** Ingested media format is unsupported by the platform's native WebView (e.g., MKV with Opus on certain Linux distributions).
- **Action:** User confirms preview proxy creation.
- **Expected Result:** FFmpeg transcodes a lightweight 720p H.264 proxy to the application cache directory (`~/.cache/opentake/proxies/`). Progress bar displays completion. Upon completion, preview switches to proxy while all edit calculations continue referencing the raw source file.
- **Error Behavior:** If proxy generation is cancelled, user can still edit via waveform or choose another file.
- **Verification Method:** Test with an MKV/Opus fixture on Linux WebKitGTK.
- **Release Blocker:** YES.

---

## 4. Editing Engine & Data Integrity

### REQ-EDIT-001: Absolute Preservation of Original Media (Zero Mutation)
- **Description:** Raw source media files must never be modified under any circumstance.
- **Preconditions:** A source media file exists on disk with pre-computed SHA-256 hash.
- **Action:** User imports media, executes 20+ splits, trims segments, accepts silence cuts, saves project, and exports the final video.
- **Expected Result:** The original source file content, byte size, file permissions, and SHA-256 hash remain bit-for-bit identical before and after all operations.
- **Error Behavior:** Any operation attempting to obtain a write handle to the source file is a critical architectural violation.
- **Verification Method:** Automated end-to-end regression test:
  1. Compute `hash_before = sha256(source_file)`.
  2. Execute full editing and export workflow.
  3. Compute `hash_after = sha256(source_file)`.
  4. Assert `hash_before == hash_after`.
- **Release Blocker:** YES (CRITICAL).

### REQ-EDIT-002: Non-Destructive Retained Ranges (Split & Trim)
- **Description:** Splitting or trimming segments updates the internal Edit Decision List without altering raw media.
- **Preconditions:** Project is active with at least one retained range $[0, T]$.
- **Action:** User positions playhead at $t_{\text{split}}$ and triggers Split (`S`).
- **Expected Result:** The active range is partitioned into two distinct retained ranges $[0, t_{\text{split}}]$ and $[t_{\text{split}}, T]$. The timeline renders two visual segments separated by a cut line.
- **Error Behavior:** If $t_{\text{split}}$ falls on an already excised interval or outside media duration, split command is safely ignored with no state mutation.
- **Verification Method:** Rust unit tests for `EditDecisionList::split_at(timestamp_us)`.
- **Release Blocker:** YES.

### REQ-EDIT-003: Segment Deletion & Restoration
- **Description:** Users can excise unwanted segments and restore them at any point.
- **Preconditions:** Multiple segments exist on the timeline.
- **Action:** User selects a segment and presses `Delete` / `Backspace`. Later, user selects the excised region and clicks "Restore Segment".
- **Expected Result:** When deleted, the segment is removed from the retained ranges list and appears dim/translucent on the timeline. When restored, it returns to the active retained ranges list with original start and end boundaries.
- **Error Behavior:** Cannot delete the last remaining frame of a project without a confirmation warning ("Timeline cannot be completely empty").
- **Verification Method:** Unit tests verifying `EditDecisionList::delete_range()` and `EditDecisionList::restore_range()`.
- **Release Blocker:** YES.

### REQ-EDIT-004: Undo & Redo Command History
- **Description:** All timeline modifications must be reversible and repeatable.
- **Preconditions:** User has performed a sequence of actions: Split → Delete → Trim → Accept Silence Cut.
- **Action:** User presses `Ctrl+Z` four times, then `Ctrl+Shift+Z` twice.
- **Expected Result:** Each `Ctrl+Z` reverses the exact prior action, returning the EDL and timeline to its previous state. `Ctrl+Shift+Z` reapplies the actions deterministically.
- **Error Behavior:** When history stack reaches the bottom or top, Undo/Redo commands no-op cleanly without error or state corruption.
- **Verification Method:** Automated Rust unit test executing 100 random mutation operations followed by full undo and redo, asserting EDL equality at each snapshot.
- **Release Blocker:** YES.

### REQ-EDIT-005: Keyboard Shortcut Operations
- **Description:** Core editing workflows can be executed entirely via keyboard.
- **Preconditions:** Application window is focused.
- **Action:** User tests standard shortcuts: `Space` (Play/Pause), `S` (Split), `Delete` (Delete Segment), `[` (Previous Cut), `]` (Next Cut), `Ctrl+S` (Save), `Ctrl+Z` (Undo).
- **Expected Result:** Each keyboard shortcut triggers the corresponding command immediately without requiring mouse clicks.
- **Error Behavior:** When focus is inside a text input field (e.g., project name), timeline shortcuts are suspended to prevent accidental edits while typing.
- **Verification Method:** Playwright/Webdriver desktop integration tests for keydown events.
- **Release Blocker:** YES.

---

## 5. Audio Waveform

### REQ-WAVE-001: Waveform Generation & Multi-Resolution LOD
- **Description:** Background extraction and multi-resolution rendering of audio waveform.
- **Preconditions:** Audio-bearing video is imported.
- **Action:** Rust background task decodes audio to PCM and generates peak/RMS downsampled buckets.
- **Expected Result:** Canvas waveform renders synchronously beneath the video timeline ruler within 3 seconds for a 10-minute video. Zooming in or out dynamically switches LOD buckets smoothly without UI hitching.
- **Error Behavior:** If audio decoding encounters corrupted audio packets, FFmpeg recovers at the next key packet; waveform skips bad samples without failing entire generation.
- **Verification Method:** Benchmarking waveform extraction speed and canvas frame rate during rapid zoom operations.
- **Release Blocker:** YES.

### REQ-WAVE-002: Bounded Memory Waveform Buffering
- **Description:** Application memory utilization must remain strictly bounded during waveform generation.
- **Preconditions:** A long video file (2 to 3 hours, $\ge 10\text{ GB}$) is imported.
- **Action:** Audio analysis and waveform computation execute.
- **Expected Result:** Total process resident set size (RSS) memory consumption must not exceed 500 MB at any point during waveform generation.
- **Error Behavior:** If system memory pressure is detected, generation throttles chunk size.
- **Verification Method:** Memory profiling using Linux `valgrind` / `massif` or `/proc/[pid]/statm` logging during ingestion of a 3-hour fixture video.
- **Release Blocker:** YES.

---

## 6. Silence Detection & Automatic Editing

### REQ-SILENCE-001: Silence Detection Sensitivity & Accuracy
- **Description:** Audio silence analysis identifies intervals where audio energy falls below a user-configured threshold for at least a minimum duration.
- **Preconditions:** Audio track loaded. Synthetic audio fixture containing three 2.0-second silence gaps with ambient noise at -45 dBFS and speech at -14 dBFS.
- **Action:** User runs silence analysis with Threshold = -35 dBFS, Min Duration = 500 ms.
- **Expected Result:** Analysis completes in $< 2.0\text{ seconds}$ per 10 minutes of video. Exactly three candidate cut regions are identified, corresponding to the known silent intervals within $\pm 20\text{ ms}$ accuracy.
- **Error Behavior:** If audio is completely silent from start to finish, the system warns: "Audio track appears entirely silent. No speech detected."
- **Verification Method:** Automated unit test running `detect_silence()` against synthetic PCM fixtures with known silence intervals.
- **Release Blocker:** YES.

### REQ-SILENCE-002: Padding & Boundary Protection
- **Description:** Pre/post cut padding must be added around speech boundaries to prevent clipping word endings or plosives.
- **Preconditions:** Detected silence interval starts at $t_1$ and ends at $t_2$.
- **Action:** User sets Pre/Post Padding to 150 ms.
- **Expected Result:** The proposed cut region is clamped to $[t_1 + 150\,\text{ms},\, t_2 - 150\,\text{ms}]$. If the remaining cut duration is less than 50 ms, the cut is automatically discarded.
- **Error Behavior:** Padding values cannot exceed half of the minimum silence duration.
- **Verification Method:** Unit test verifying padding math on `SilenceCandidate` structs.
- **Release Blocker:** YES.

### REQ-SILENCE-003: Interactive Suggestion Review & Audition
- **Description:** Users can step through proposed cuts, audition them, and accept or reject them individually.
- **Preconditions:** Silence analysis has generated $K$ cut suggestions.
- **Action:** User clicks "Next Cut" (`]`), plays preview around the cut boundary, and clicks "Reject Cut".
- **Expected Result:** Playhead jumps to 1.0 second before the candidate cut. Rejecting the cut marks that interval as retained; waveform overlay updates from candidate color to active audio color.
- **Error Behavior:** When jumping past the last cut, navigation wraps around or indicates "Last cut reached".
- **Verification Method:** Desktop UI interaction test verifying suggestion state transitions.
- **Release Blocker:** YES.

---

## 7. Project Persistence

### REQ-PROJ-001: Project File Creation & Atomic Persistence
- **Description:** Saving a project creates a versioned JSON `.opentake` file via atomic rename.
- **Preconditions:** Active editing session with multiple cuts.
- **Action:** User presses `Ctrl+S` or selects "Save Project".
- **Expected Result:** A valid JSON file is written to `<filename>.opentake.tmp`, flushed to disk (`fsync`), and atomically renamed to `<filename>.opentake`. The file contains `schema_version: 1`, source file path, source file size/checksum, detection parameters, and retained ranges array.
- **Error Behavior:** If disk write fails (e.g., read-only filesystem or out of disk space), user is alerted with an explicit error dialog; existing `.opentake` file is left untouched.
- **Verification Method:** Unit test saving project, asserting JSON schema validity, simulating disk write failure, and verifying atomic safety.
- **Release Blocker:** YES.

### REQ-PROJ-002: Project Reloading & State Restoration
- **Description:** Loading an existing `.opentake` file restores all edit decisions faithfully.
- **Preconditions:** An existing `.opentake` project file saved on disk.
- **Action:** User clicks "Open Project" and selects the file.
- **Expected Result:** OpenTake parses the JSON file, locates the referenced media, verifies source compatibility, and restores all retained ranges, waveform markers, and detection parameters.
- **Error Behavior:** If schema version is newer than supported by the application, alert user: "This project was created by a newer version of OpenTake. Please update your application."
- **Verification Method:** Roundtrip serialization test: `Project -> Save -> Load -> Assert Equality`.
- **Release Blocker:** YES.

### REQ-PROJ-003: Missing Source Media Relink
- **Description:** Graceful handling when raw media source has moved or been renamed.
- **Preconditions:** Project references a media file that has been renamed or moved.
- **Action:** User opens the `.opentake` project file.
- **Expected Result:** System detects missing file and displays a "Media Offline / Relink Media" dialog showing original path. User selects the new location; OpenTake validates stream duration and updates the project reference.
- **Error Behavior:** If user cancels relinking, project is not modified on disk.
- **Verification Method:** Integration test moving source file to a new subdirectory and triggering relink flow.
- **Release Blocker:** YES.

---

## 8. Export Pipeline & Verification

### REQ-EXPORT-001: Timeline Consistency & Retained Range Integrity
- **Description:** Exported video must represent the exact ordered retained ranges specified in the EDL, within documented frame tolerances.
- **Preconditions:** Project with 5 non-contiguous retained ranges totaling exactly 62.5 seconds of content.
- **Action:** User exports the project to MP4.
- **Expected Result:** The resulting MP4 file has a total duration of $62.5\text{ s} \pm 1\text{ video frame}$ ($41.6\text{ ms}$ at 24 fps). Visual content matches the retained ranges with zero dropped or reordered scenes.
- **Error Behavior:** If total exported duration deviates by more than 2 video frames from calculated EDL duration, export verification fails with a diagnostic alert.
- **Verification Method:** Automated test using synthetic media with burnt-in visual timecodes and audio tones, inspecting output stream duration via FFprobe.
- **Release Blocker:** YES (CRITICAL).

### REQ-EXPORT-002: Standard Encoding Presets (1080p / 720p / H.264 / AAC)
- **Description:** Video must be encoded using universally compatible H.264 (libx264) and AAC audio.
- **Preconditions:** Export dialog opened.
- **Action:** User selects "1080p Preset" (or "720p Preset") and clicks "Export".
- **Expected Result:** Output file is valid MP4 (`isom` / `mp42` major brand). Video stream is `h264 (High)` profile; audio stream is `aac (LC)`. File plays back correctly in standard players (VLC, mpv, browser, QuickTime).
- **Error Behavior:** If encoding fails due to unsupported resolution or encoder crash, temporary `.tmp` file is deleted and user is presented with FFmpeg error details.
- **Verification Method:** Automated probe verification checking `ffprobe -show_streams export.mp4`.
- **Release Blocker:** YES.

### REQ-EXPORT-003: Audio / Video Synchronization Boundary Tolerances
- **Description:** Audio and video streams must remain strictly in sync across all cut boundaries throughout the exported video.
- **Preconditions:** Project with 30+ silence cuts distributed across a 15-minute video.
- **Action:** Export project to MP4.
- **Expected Result:** Audio/Video drift across the entire exported file must not exceed $\pm 25\text{ ms}$ (less than 1 frame at 30 fps). There must be no audible pops, clicks, or visual tearing at cut splice points.
- **Error Behavior:** Any progressive drift exceeding $50\text{ ms}$ is a release-blocking failure.
- **Verification Method:** Test using synthetic AV sync flash/beep calibration video, comparing audio burst onset to video flash frame timestamp via automated script.
- **Release Blocker:** YES (CRITICAL).

### REQ-CANCEL-001: Safe Export and Analysis Cancellation
- **Description:** Cancelling active operations must immediately halt processing and leave no orphaned files or processes.
- **Preconditions:** Export or silence analysis is actively running at 40% progress.
- **Action:** User clicks "Cancel".
- **Expected Result:**
  - FFmpeg child process is terminated within 1,500 ms.
  - Partial `.tmp` export files are deleted from the disk.
  - Rust Tokio worker tasks exit cleanly.
  - UI returns to ready state within 200 ms.
  - Application memory and CPU usage return to baseline.
- **Error Behavior:** Child process must not hang or become an orphaned zombie process.
- **Verification Method:** Automated test initiating export, issuing cancellation via IPC after 500 ms, checking process table (`ps aux`) for orphaned FFmpeg processes, and asserting `.tmp` file absence.
- **Release Blocker:** YES.

---

## 9. Security & System Safety

### REQ-SEC-001: Path Traversal & Filesystem Security
- **Description:** All paths passed to native commands must be sanitized against path traversal and symlink hijacking.
- **Preconditions:** Malicious or malformed path strings passed to backend IPC commands (e.g., `../../../../etc/passwd` or null-byte paths).
- **Action:** Rust backend validates and canonicalizes paths.
- **Expected Result:** Path validation rejects relative traversal components and paths outside allowed project roots. An `Err(SecurityError::InvalidPath)` is returned.
- **Error Behavior:** File access is denied; application never accesses unauthorized system files.
- **Verification Method:** Fuzz testing IPC endpoints with malicious path vectors.
- **Release Blocker:** YES.

### REQ-SEC-002: Injection-Proof Process Spawning
- **Description:** All child processes must be spawned using structured argument lists.
- **Preconditions:** Media filenames containing special shell characters (e.g., `test; rm -rf / ;.mp4`, `$(`echo evil`)`, quotes, spaces, unicode).
- **Action:** OpenTake invokes FFprobe and FFmpeg on the file.
- **Expected Result:** Commands are executed strictly via `tokio::process::Command::new("ffmpeg").args(["-i", path_str, ...])`. Filename is passed as a single isolated argv element. No shell interpolation occurs. Operation succeeds normally.
- **Error Behavior:** Filename containing quotes or semicolons must never execute auxiliary commands.
- **Verification Method:** Automated integration test creating files with malicious shell filenames and verifying safe FFprobe execution.
- **Release Blocker:** YES (CRITICAL).
