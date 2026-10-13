# OpenTake — Product Requirements Document (PRD)

**Document Version:** 1.0.0-draft  
**Release Target:** v0.1.0 Beta  
**Status:** In Review / Baseline Lock  
**Author:** OpenTake Core Architecture Team  
**Date:** 2026-10-13  

---

## 1. Product Vision & Problem Statement

### 1.1 What is OpenTake?
**OpenTake** is a free, open-source, privacy-first desktop application designed for automated and semi-automated video editing. It provides content creators with an accessible studio environment to ingest spoken-word video recordings, automatically detect and excise dead air and silence, preview and fine-tune cut decisions on a responsive timeline, and export crisp, synchronized MP4 video files.

OpenTake is engineered as an **offline-first desktop application** built on **Tauri 2**, **Rust**, and **React/TypeScript**, utilizing **FFmpeg/FFprobe** for reliable, hardware-capable media decoding, inspection, and rendering.

### 1.2 Why Does OpenTake Exist?
In the modern creator economy, spoken-word video production (tutorials, podcasts, educational lectures, video essays, and social content) involves a tedious, repetitive initial stage: **the rough cut**. Creators spend hours scrubbing through raw camera takes simply to cut out pauses, breathing breaks, hesitations, and silent gaps between spoken sentences.

Existing proprietary solutions (such as Gling AI and TimeBolt) have validated this workflow, but present severe friction points:
1. **Mandatory Subscriptions and Artificial Paywalls:** Users face recurring monthly fees ($15–$35/month) or strict limits on exported hours and video resolutions.
2. **Cloud Exfiltration and Privacy Invasion:** Several commercial tools require uploading raw, unedited footage to remote cloud servers for processing, which violates privacy policies for internal corporate recordings, unreleased IP, and privacy-conscious creators.
3. **Vendor Lock-in and Watermarking:** Free tiers penalize creators with intrusive watermarks, reduced bitrates, or restricted project formats.
4. **Bloat and Instability:** Heavy Electron-based or web-wrapped architectures lead to sluggish playback, high memory consumption, and platform inconsistencies.

### 1.3 Core Product Principles
OpenTake is governed by fourteen non-negotiable architectural and product tenets:
- **Free Core Functionality:** The core editing and export features will remain free of financial barriers.
- **Open Source:** Public codebase distributed under a transparent open-source license.
- **No Mandatory Account:** Zero registration, authentication tokens, or email requirements.
- **No Subscriptions:** No recurring monetization or license key validations.
- **No Watermarks:** Video outputs are clean, bit-for-bit creator content.
- **No Artificial Processing Quotas:** Unlimited local processing, bounded only by the user's host hardware, storage, and CPU/GPU capability.
- **No Mandatory Cloud Processing:** 100% offline functionality. Network disconnection must not degrade core capabilities.
- **Privacy-First Data Handling:** Media files never leave the local machine. Telemetry is disabled by default.
- **Preservation of Original Media:** Raw source media files are strictly treated as read-only.
- **Non-Destructive Editing:** All user edits are stored in an Edit Decision List (EDL); raw clips are never overwritten or altered.
- **Beginner-Friendly Interface:** Focused user experience prioritizing the rapid rough-cut workflow over multi-panel NLE complexity.
- **Modular Architecture:** Clear boundary between the high-performance Rust core (DSP, FFmpeg orchestration, state persistence) and the React UI.
- **Reliable Export Behavior:** Audio/video synchronization must be mathematically guaranteed and verified.
- **Community-Friendly Development:** Clean documentation, predictable development roadmap, reproducible builds, and clear contributor guidelines.

### 1.4 Market Positioning & Scope Boundary
OpenTake v0.1.0 Beta is laser-focused on **silence elimination and rapid rough-cutting for single-source spoken-word video**.

- **What OpenTake IS in v0.1.0:** A lightweight, rapid, offline alternative to TimeBolt and Gling AI for detecting dead air, reviewing suggested cuts, trimming segments, and producing clean MP4 exports.
- **What OpenTake IS NOT in v0.1.0:** A general-purpose Nonlinear Editor (NLE). It does not attempt to replace DaVinci Resolve, Adobe Premiere Pro, Final Cut Pro, or Kdenlive. It does not provide multi-track video compositing, color grading, title generation, audio mixing consoles, or complex transition effects.

---

## 2. Target Users & Personas

| Persona | Primary Content Type | Key Pain Points | Core Need from OpenTake |
|---|---|---|---|
| **The Solo YouTuber** | Talking-head tutorials, tech reviews, commentary. | Spends 40% of editing time cutting silent gaps; frustrated by cloud upload times for large 1080p/4K files. | Instant local silence detection, fast timeline review, high-quality MP4 export with zero watermarks. |
| **The Podcaster / Interviewer** | Video podcasts, guest discussions, monologue episodes. | Long recordings (60–120 minutes) take forever to scrub; paid cloud tools charge per hour processed. | Memory-bounded processing for 2+ hour recordings; threshold sensitivity to isolate silent gaps without clipping trailing syllables. |
| **Online Educator / Course Creator** | Screen-recording tutorials, slide presentations with webcam inset. | Constant retakes and pauses while checking notes; needs polished, gap-free lessons quickly. | Keyboard-driven cut review (Space to play, J/K/L navigation, accept/reject cuts), reliable project saves. |
| **Freelance Video Editor** | Pre-editing client footage before final grading in Premiere/DaVinci. | Repetitive rough-cutting eats into billable hours; client NDAs prohibit uploading footage to third-party AI clouds. | Offline security, quick rough-cut export, accurate cut decisions. |
| **Small Marketing Team** | Product demos, founder updates, internal training videos. | Limited video editing expertise; Premiere Pro has an intimidating learning curve. | Dead-simple single-window workflow: Drag, Analyze, Adjust, Export. |
| **Beginner Content Creator** | Gaming commentary, vlog intros, casual videos. | Cannot afford monthly SaaS subscriptions ($20+/mo); gets intimidated by complicated multi-track software. | Free, zero-setup, intuitive visual waveform with clear cut highlights. |

---

## 3. Primary Jobs to Be Done (JTBD)

### JTBD-01: Rapid Silence Elimination
> *When I finish recording a raw spoken video take with multiple natural pauses, hesitations, and silence,*  
> *I want the software to automatically scan the audio and identify all dead-air intervals above a customizable threshold,*  
> *So that I can eliminate dead air in seconds rather than manually splitting and rippling clips for hours.*

### JTBD-02: Granular Review & False-Positive Correction
> *When the software proposes cut points,*  
> *I want to visually inspect the highlighted silent regions on a synchronized waveform and preview the edited result,*  
> *So that I can reject cuts that inadvertently remove intentional dramatic pauses or trailing word endings.*

### JTBD-03: Safe Non-Destructive Manual Editing
> *When reviewing the timeline,*  
> *I want to manually split, trim, delete, or restore regions using keyboard shortcuts and mouse clicks,*  
> *So that I can fine-tune edits with the certainty that my original raw recording remains completely untouched.*

### JTBD-04: Resumable Project Persistence
> *When working on a long video project that I cannot finish in one sitting,*  
> *I want to save my project to a lightweight file and resume later,*  
> *So that my customized thresholds, manual splits, and cut decisions are restored precisely.*

### JTBD-05: Private, Watermark-Free Export
> *When I am satisfied with the edited timeline,*  
> *I want to export a crisp, properly synchronized MP4 video locally on my machine,*  
> *So that my video is ready for distribution without external watermarks, quality degradation, or exposure of private media to the cloud.*

---

## 4. Primary User Workflow

```
[1. Launch OpenTake]
         │
         ▼
[2. Ingest Media] ────────── Drag & Drop or System File Picker
         │
         ▼
[3. Inspect & Validate] ──── FFprobe metadata inspection (Duration, Codec, FPS, Audio)
         │
         ▼
[4. Preview & Waveform] ──── Instant HTML5 preview & background audio waveform render
         │
         ▼
[5. Analyze Silence] ─────── User triggers DSP pass (Threshold dB, Min Duration ms, Padding ms)
         │
         ▼
[6. Review Suggestions] ──── Suggested cuts highlighted over waveform; jump between cuts
         │
         ▼
[7. Refine Edits] ────────── Accept/Reject cuts, manual split, trim, undo/redo
         │
         ▼
[8. Preview Cut Timeline] ── Seamless playback skipping excised regions
         │
         ▼
[9. Save Project] ────────── Save lightweight `.opentake` JSON project descriptor
         │
         ▼
[10. Export MP4] ─────────── Select preset (1080p/720p/Source), render via FFmpeg, atomic swap
         │
         ▼
[11. Verification] ───────── Confirm original file untouched; verify export A/V sync
```

### Detailed Workflow Step Breakdown:
1. **Launch OpenTake:** Application opens into a modern, uncluttered dark-theme desktop workspace with an active drop zone and quick-action buttons.
2. **Select or Drag Media:** User drops an MP4, MOV, or MKV file into the window or clicks "Open Video".
3. **Inspect Media Metadata:** Backend verifies container validity via FFprobe, extracting duration, container type, video stream resolution/codec/framerate, and audio sample rate/channels. Metadata is displayed in the media info header.
4. **Preview Original Video:** Source video loads in the preview viewport, with synchronized transport controls (Play/Pause, Scrub, Timecode display).
5. **Analyze Audio for Silence:** User initiates silence detection (or auto-triggers on import per user preference). Configurable parameters:
   - *Silence Threshold:* -60 dB to -20 dB (default: -35 dB).
   - *Minimum Silence Duration:* 200 ms to 2000 ms (default: 500 ms).
   - *Padding Buffer (Pre/Post):* 50 ms to 500 ms (default: 150 ms) to prevent clipping word boundaries.
6. **Display Suggested Cut Regions:** Cut regions are highlighted on the waveform track with distinct visual markers (e.g., translucent amber/red striped zones).
7. **Review & Adjust Suggestions:** User cycles through cut points using shortcut keys (e.g., `[` and `]`), auditions each transition, and toggles individual cuts between accepted and ignored.
8. **Manual Timeline Editing:** User uses the Playhead and Razor tool (`S` to Split, `Backspace` to Delete segment, `Z` to Undo) to perform manual cuts.
9. **Audition Edited Timeline:** When playing with "Skip Silence" enabled, the playhead skips excised regions in real time, letting the creator hear the natural cadence of the edit.
10. **Save Project:** User presses `Ctrl+S` to save an atomic `.opentake` project file storing source media reference and the EDL array.
11. **Export MP4 Video:** User selects an export preset (1080p, 720p, or Match Source), sets output destination, and clicks "Export Video". Background FFmpeg process renders the output with live percentage progress, ETA, and a responsive "Cancel" button.
12. **Verify Output:** Export finishes, displaying a "Show in Folder" action. The original input file hash is re-verified to confirm zero mutation.

---

## 5. Comprehensive User Scenarios & Edge Cases

### Scenario 5.1: First-Time User Experience
- **Context:** User launches OpenTake for the very first time.
- **Expected Behavior:** An intuitive zero-state dashboard greets the user with an illustrative drag-and-drop target, an "Open Media File" button, and a concise 3-step visual guide ("Import → Detect Silence → Export"). No account login, no telemetry opt-in prompts, and no internet connection required.

### Scenario 5.2: Returning User Opening an Existing Project
- **Context:** User selects "Open Project" or opens a `.opentake` file from disk.
- **Expected Behavior:** System validates schema version, reads relative or absolute path of source media, verifies source file presence and checksum. All retained ranges, cut segments, silence detection settings, and cursor positions are restored exactly as saved.

### Scenario 5.3: Long Recording Ingestion (2–3 Hours)
- **Context:** A podcaster imports a 2.5-hour 1080p recording (~12 GB).
- **Expected Behavior:**
  - File is probed in under 500 ms without loading the full video into RAM.
  - Audio extraction and DSP analysis runs in chunks / streaming mode via Rust background threads.
  - Progress indicator displays audio scan percentage.
  - Waveform data is downsampled and cached in memory using multi-resolution buckets (LOD) to prevent UI thread lag or out-of-memory crashes. Memory footprint remains bounded under 500 MB.

### Scenario 5.4: Media Without Audio Stream
- **Context:** User imports a screen recording or timelapse video that has no audio tracks.
- **Expected Behavior:** FFprobe probing identifies zero audio streams. The UI gracefully informs the user: *"No audio track detected in this media file. Silence detection is disabled. You can still preview and perform manual cuts."* Silence detection controls are cleanly disabled with an informative tooltip; application does not crash.

### Scenario 5.5: Unsupported Codec or Corrupted Container
- **Context:** User imports an unsupported format (e.g., proprietary RAW or truncated container).
- **Expected Behavior:** Validation pipeline catches probe failure. A user-friendly error modal appears: *"Unsupported Media Format: OpenTake could not read video stream from [filename]. Supported baseline formats for v0.1.0 Beta are MP4, MOV, and MKV with H.264/AAC."* The invalid file is rejected, and the UI returns to the clean ready state without hanging.

### Scenario 5.6: User Rejects All Automatic Suggestions
- **Context:** The user analyzes silence on a video with soft ambient background music, resulting in false-positive detections.
- **Expected Behavior:** User clicks "Reject All Suggestions" or adjusts the threshold slider. The suggested cuts are cleared from the EDL in a single action, reverting the timeline to an uncut state. The user can then proceed with manual cuts.

### Scenario 5.7: User Restores an Accidentally Cut Section
- **Context:** While manually trimming or accepting automatic cuts, the user cuts out an important spoken phrase.
- **Expected Behavior:**
  - *Immediate action:* Pressing `Ctrl+Z` (Undo) immediately restores the segment.
  - *Timeline action:* The user can click on any excluded segment in the timeline and click "Restore Segment", turning it back into an active retained range. Because the underlying model is non-destructive, all source media frames remain accessible.

### Scenario 5.8: User Cancels Silence Analysis Midway
- **Context:** User begins silence analysis on a 3-hour file, then realizes they selected the wrong threshold setting and clicks "Cancel".
- **Expected Behavior:** The Rust backend signals the Tokio cancellation token, cleanly terminating the FFmpeg audio pipe process. The progress modal dismisses immediately, temporary partial PCM buffers are discarded, and the UI returns to an interactive state within 200 ms.

### Scenario 5.9: User Cancels Video Export Midway
- **Context:** User starts a 1080p export, progress reaches 45%, and the user clicks "Cancel Export".
- **Expected Behavior:** Export cancellation token is triggered. The active FFmpeg export process is gracefully stopped (SIGINT/SIGTERM or process kill if unresponsive within 2 seconds). Partial export `.tmp` files are deleted from the destination directory to prevent clutter. No corrupted video files are left behind. UI returns to timeline editing state.

### Scenario 5.10: Reopening Project Whose Media Moved or Was Renamed
- **Context:** User moves raw footage to an external drive and reopens the `.opentake` project.
- **Expected Behavior:** The application detects that the referenced media path does not exist. A "Media Offline / Missing Media" dialog appears, displaying the original path and filename, and prompts: *"Locate Missing Media File"*. Upon locating the file, OpenTake verifies duration and stream compatibility, updates the relative/absolute path in project state, and restores the editing session seamlessly.

### Scenario 5.11: Insufficient Storage Space During Export
- **Context:** Target disk has 150 MB free, but estimated export size is 1.2 GB.
- **Expected Behavior:** Pre-flight export check queries available disk space on the target partition. If available space is less than estimated required space + 500 MB buffer, the application halts before launching FFmpeg with an explicit alert: *"Insufficient Disk Space: Estimated export size is 1.2 GB, but only 150 MB is available on target drive. Please choose another location or free up space."*

### Scenario 5.12: Unexpected Processing Failure During Encoding
- **Context:** FFmpeg encounters an unrecoverable corrupted packet at frame 42,000 and exits with a non-zero exit code.
- **Expected Behavior:** The Rust backend captures the stderr output from FFmpeg, logs the error internally, suppresses raw panic stack traces from the user, and presents a diagnostic dialog: *"Export Failed: Video encoder encountered an error at timecode 00:23:15. Temporary files have been cleaned up. Please verify your source media."* Application state remains intact and stable.

---

## 6. Non-Functional Requirements (NFRs)

### 6.1 Reliability & Data Integrity
- **NFR-REL-01 (Zero Source Mutation):** Under no circumstance shall OpenTake write to, rename, truncate, or touch the raw media file. Verified via SHA-256 hash checks.
- **NFR-REL-02 (Atomic File Persistence):** Project saves and final video exports must write to a temporary file (`.tmp`) and perform an atomic filesystem rename upon verified completion.
- **NFR-REL-03 (Graceful Process Termination):** All spawned child processes (FFmpeg/FFprobe) must be managed with lifetime guards. Orphan processes must be prevented even if the Tauri window is abruptly closed.

### 6.2 Performance & Responsiveness
- **NFR-PERF-01 (UI Thread Independence):** All media probing, audio decoding, DSP silence detection, and video encoding must execute on background threads via Tokio. The React UI thread frame rate must remain ≥ 55 FPS during background operations.
- **NFR-PERF-02 (Bounded Memory Utilization):** Peak application memory usage during silence analysis or playback of a 2-hour video must not exceed 600 MB on x86_64 Linux.
- **NFR-PERF-03 (Rapid Probe Time):** Media metadata extraction must complete within 1.0 second for local files under 10 GB.
- **NFR-PERF-04 (Low-Latency Scrubbing):** Seeking the playhead to any arbitrary position on the timeline must update the video frame within 150 ms for native formats.

### 6.3 Security & Privacy
- **NFR-SEC-01 (Air-Gapped Operation):** Zero outbound HTTP/HTTPS requests during all standard editing and export workflows.
- **NFR-SEC-02 (Safe Process Spawning):** FFmpeg/FFprobe invocations must strictly pass argument arrays to `std::process::Command` / `tokio::process::Command`. Direct shell evaluation (`sh -c` or `cmd.exe /C`) and shell string concatenation are strictly prohibited.
- **NFR-SEC-03 (Path Sanitation):** All input paths, project file paths, and export destinations must be normalized and canonicalized to protect against path traversal attacks.

### 6.4 Usability & Accessibility
- **NFR-UX-01 (Color Contrast & Ergonomics):** Dark-mode interface designed with WCAG AA compliance (minimum 4.5:1 contrast ratio for body text and interactive icons).
- **NFR-UX-02 (Keyboard First):** All primary editing actions (Play/Pause, Split, Delete, Restore, Undo, Redo, Jump Next/Prev Cut) must have discoverable, standard keyboard shortcut bindings.
