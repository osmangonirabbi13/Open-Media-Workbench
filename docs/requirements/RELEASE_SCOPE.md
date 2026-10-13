# OpenTake — Release Scope & Feature Lock (v0.1.0 Beta)

**Document Version:** 1.0.0-locked  
**Release Target:** v0.1.0 Beta  
**Scope Status:** LOCKED — Strict Boundary Enforcement  
**Author:** OpenTake Core Architecture Team  
**Date:** 2026-10-13  

---

## 1. Scope Governance & Release Philosophy

To guarantee the delivery of a robust, high-performance, and verifiable desktop beta within the target development timeline (~30 days), OpenTake strictly enforces a **three-tier feature classification model**:

- **P0 (Release Blocker / Mandatory for v0.1.0 Beta):** Features without which OpenTake cannot deliver its core promise of offline silence removal and rough-cutting. Every P0 feature must be fully implemented, covered by automated or documented manual verification, and meet all acceptance criteria before tagging `v0.1.0-beta`.
- **P1 (Post-Beta Fast Follow):** High-value enhancements scheduled for v0.2.0 through v0.4.0. Work on P1 features during the initial 30-day development window is strictly prohibited until all P0 gates pass.
- **P2 (Advanced Roadmap / Long-Term Vision):** Complex multi-month architectural expansions (e.g., multi-track compositing, AI smart zooms, plugin marketplaces) targeted for v1.0.0 and beyond.

> [!CRITICAL]
> **Scope Invariant:** Under no circumstances shall P1 or P2 features be silently moved into the v0.1.0 Beta scope. Any proposed addition to P0 requires a formal architectural amendment and explicit timeline recalculation.

---

## 2. P0 — Required for v0.1.0 Beta

### 2.1 Media Import & Inspection
- [x] **Local Drag-and-Drop Ingestion:** Direct file dropping onto the application window or designated drop zone.
- [x] **Native File Picker:** Native system dialog (via Tauri dialog plugin) for selecting local video files.
- [x] **Validated Container Support:**
  - *MP4 Container:* Primary reference format.
  - *MOV Container:* Supported where internal video/audio streams are compatible with the pipeline.
  - *MKV Container:* Supported where internal video/audio streams are compatible with the pipeline.
- [x] **Stream Metadata Extraction:** Extraction of duration, container format, video resolution (width × height), display aspect ratio, video codec, frame rate (CFR/VFR detection), audio sample rate, and audio channel count via FFprobe.
- [x] **Explicit Unsupported-Media Messaging:** Comprehensive, human-readable error dialogs when encountering unreadable, corrupted, or unsupported media codecs (e.g., AV1 on unaccelerated systems, Apple ProRes without decoder, or corrupted headers).

### 2.2 Playback Engine & Transport
- [x] **Preview Viewport:** Responsive video preview surface integrated into the desktop layout.
- [x] **Transport Controls:** Play, Pause, Toggle Playback (`Space`), Step Forward/Backward by single frame (`Left`/`Right` arrow keys).
- [x] **Timeline Playhead Synchronization:** High-precision scrubbing; moving the timeline playhead immediately updates the preview frame.
- [x] **Timecode Displays:** Source timecode and current timeline position displayed in `HH:MM:SS:FF` or `MM:SS.mmm`.
- [x] **Documented Fallback for Unsupported Native Playback:** If WebView2/WebKitGTK cannot natively decode a supported container/codec stream, the system generates a fast local preview proxy or utilizes a local stream bridge.

### 2.3 Non-Destructive Manual Editing
- [x] **Single-Source, Single-Track Model:** Focused linear edit decision list (EDL) operating on one raw source video file.
- [x] **Razor / Split Tool:** Split active video segment at the current playhead position (`S` key).
- [x] **Trim In / Trim Out:** Adjust start and end boundaries of any segment via timeline handles.
- [x] **Remove / Delete Segment:** Mark a segment as excluded from the exported output (`Delete` / `Backspace`).
- [x] **Restore Segment:** Re-activate any previously excised segment without re-importing the source file.
- [x] **Full Undo / Redo:** Robust command-pattern history stack (`Ctrl+Z` / `Ctrl+Y` or `Ctrl+Shift+Z`) tracking all split, trim, delete, and silence-acceptance operations.
- [x] **Keyboard Shortcut Engine:** Essential, discoverable editing shortcuts (`Space`, `S`, `J`/`K`/`L`, `Delete`, `Z`).
- [x] **Non-Destructive State:** Editing state represented entirely by an ordered list of retained source time ranges `[start_pts, end_pts]`. Raw media file remains read-only.

### 2.4 Audio Waveform Visualization
- [x] **Audio Extraction & Decoded PCM:** Background extraction of audio track to raw PCM for analysis.
- [x] **Peak/RMS Waveform Generation:** Computation of min/max decibel amplitude points downsampled for rendering.
- [x] **Timeline Visualization:** High-performance canvas-based waveform rendered synchronously beneath the video timeline ruler.
- [x] **Memory-Bounded Waveform Buffer:** Multi-resolution pyramid/LOD (Level of Detail) data structure to keep RAM consumption < 50 MB even for 3-hour files.
- [x] **Local Waveform Cache:** Derived waveform data cached in memory / temporary session cache to eliminate redundant audio scans.

### 2.5 Automatic Silence Detection & Review
- [x] **Rust DSP Silence Analysis:** Fast audio scan computing RMS/peak energy across configurable time windows.
- [x] **Customizable Detection Parameters:**
  - *Silence Threshold:* Range -60 dBFS to -20 dBFS (slider control, default: -35 dBFS).
  - *Minimum Silence Duration:* Range 200 ms to 2000 ms (default: 500 ms).
  - *Pre/Post Cut Padding:* Range 50 ms to 500 ms (default: 150 ms) to protect natural speech cadence and plosives.
- [x] **Suggested Removable Regions:** Visual representation of candidate silence cuts highlighted as distinct overlays on the waveform.
- [x] **Audition & Review Workflow:** Ability to navigate between detected cuts (`Next Cut` / `Previous Cut`), audition the transition, and accept or reject individual cuts.
- [x] **Batch Operations:** "Accept All Suggestions" and "Reject All Suggestions" buttons.
- [x] **Real-Time Preview with Silence Skipped:** Option to play the timeline with proposed cuts dynamically jumped in real-time.
- [x] **Ethical Technical Communication:** Explicitly documented as *deterministic DSP energy-based silence analysis*, not deceptive marketing as "Advanced AI Neural Speech Understanding".

### 2.6 Project Persistence & Portability
- [x] **Create Project:** Initialize a clean project state upon importing media.
- [x] **Save Project (`Ctrl+S`):** Save editing state to a lightweight, human-readable `.opentake` JSON file.
- [x] **Open Project (`Ctrl+O`):** Reload project state, restoring source references, retained ranges, detection parameters, and timeline zoom/scroll.
- [x] **Versioned Project Schema:** Explicit `schema_version: 1` field in project JSON to ensure backwards-compatible schema migrations in future releases.
- [x] **Missing Media Detection & Relink:** Friendly warning and file relocation dialog if the source video has been moved, renamed, or unmounted.
- [x] **Original Source Preservation:** Source file checksum / byte size recorded in project metadata to alert user if source file was modified externally.

### 2.7 Export Engine & Pipeline
- [x] **MP4 Container Export:** Universal MP4 container output.
- [x] **Standard H.264 / AAC Encoding Path:** Standard libx264 video encoding paired with AAC audio encoding for 100% player/platform compatibility.
- [x] **Export Presets:**
  - *1080p Full HD:* 1920×1080, scaled if source exceeds or matches, target bitrate / CRF 20.
  - *720p HD:* 1280×720, faster encode, smaller file footprint.
  - *Match Source Resolution:* Maintains source aspect ratio and pixel dimensions.
- [x] **Quality Presets:** Configurable encoding speed/quality trade-off (e.g., CRF 18 for High Quality, CRF 23 for Balanced, CRF 28 for Fast Draft).
- [x] **Progress Reporting:** Live percentage completion, elapsed time, and estimated time remaining (ETA) rendered via Tauri events.
- [x] **Safe Cancellation:** Responsive "Cancel" button that immediately halts FFmpeg, reaps child processes, and purges partial `.tmp` files.
- [x] **A/V Synchronization Guarantee:** Precise timestamp normalization (`setpts=PTS-STARTPTS`, `asetpts=PTS-STARTPTS`) to prevent audio/video drift across multiple concatenated cuts.
- [x] **Zero Watermarks:** Unaltered, pure user content output.

### 2.8 User Experience & Desktop Shell
- [x] **Modern Desktop Ergonomics:** Polished, native-feeling desktop layout designed with Tailwind CSS and Radix UI.
- [x] **Curated Dark Theme:** Professional dark workspace tailored for media editing, reducing visual fatigue.
- [x] **Beginner-Friendly Layout:** Linear, uncluttered 3-pane structure: Top Bar (Project/Media info), Center Viewport (Video Preview + Controls), Bottom Deck (Waveform & Timeline).
- [x] **Responsive Progress Feedback:** Non-blocking modal sheets and status indicators for audio analysis, waveform generation, and export rendering.
- [x] **Informative Error Messages:** Human-readable explanations for all error conditions with clear recommended remedies.

### 2.9 Reliability & Security Hardening
- [x] **Read-Only Media Ingestion:** Strict avoidance of write handles on imported files.
- [x] **Untrusted Media Handling:** Defenses against malformed files; isolation of parsing within FFprobe child processes.
- [x] **Injection-Proof Process Spawning:** Absolute ban on shell string concatenation (`sh -c`); all FFmpeg/FFprobe commands built using strict, typed vector arguments (`Command::arg()`).
- [x] **Atomic Project Saving:** Projects saved via write-to-temp-then-rename pattern to eliminate file corruption risks on crash or power loss.
- [x] **Resource Bounding:** Strict memory caps on PCM buffers; audio processed via streaming pipes rather than in-memory monolithic loading.

---

## 3. P1 — Post-Beta Roadmap (v0.2.0 – v0.4.0)

The following capabilities are officially deferred until after v0.1.0 Beta validation is complete:

1. **Local AI Transcription (whisper.cpp):** Integration of bundled `whisper.cpp` engine with quantized GGML models (tiny.en, base.en) for fully local, offline speech-to-text.
2. **Interactive Text Transcript Editor:** Interactive text pane displaying timestamped words synchronized with the timeline playhead.
3. **Transcript-Based Text Cutting:** Deleting a word or sentence in the transcript automatically ripples out the corresponding video segment.
4. **Filler-Word Detection:** Automatic identification and suggested removal of filler words ("um", "uh", "like", "you know", "so").
5. **SRT / VTT Subtitle Export:** Generating synchronized subtitle files derived from local transcription.
6. **Multilingual Transcription:** Expanding Whisper models to support non-English language packs.
7. **Hardware-Accelerated Encoding (NVENC, VAAPI, VideoToolbox):** Auto-detection and utilization of GPU hardware encoders for 5x–10x faster export rendering.
8. **Light Theme & System Theme Auto-Switching:** Supporting high-contrast light theme for varying workspace ambient lighting.
9. **Project Auto-Recovery & Crash Journaling:** Periodic state snapshots to automatically restore un-saved edits following unexpected OS shutdowns.
10. **Social Media Video Presets:** Aspect ratio reframing presets (9:16 vertical for Shorts/TikTok/Reels, 1:1 square, 4:5 portrait).
11. **4K High-Bitrate Optimizations:** GPU-assisted proxy workflow specifically tuned for 4K 60fps / 120fps scrubbing.
12. **NLE Interoperability Export:** Exporting timeline cut lists to Final Cut Pro XML (`.fcpxml`), DaVinci Resolve EDL (`.edl`), or Premiere Pro project markers.

---

## 4. P2 — Advanced Future Vision (v1.0.0+)

The following capabilities represent long-term architectural expansions:

1. **Repeated-Take & Bad-Take Detection:** Multi-modal analysis identifying repeated phrases where the speaker stumbles and re-reads a sentence, automatically recommending the best take.
2. **AI Smart Auto-Zoom:** Dynamic virtual camera zooms and pans punched in on active speaker gestures or emphasis words.
3. **Multi-Track Audio & Video Editing:** Compositing B-roll video tracks, background audio music beds, and sound effect tracks.
4. **Advanced Neural Noise Suppression:** Offline deep-learning audio cleaning (e.g., DeepFilterNet) to remove HVAC hum, room reverb, and computer fan noise.
5. **Plugin Runtime & Extension SDK:** Isolated WebAssembly (Wasm) or gRPC plugin architecture allowing community developers to author custom audio/video filters.
6. **Community Extension Marketplace:** Secure, curated registry for discovering and installing user-created workflows and title templates.
7. **Animated On-Screen Captions:** Kinetic, word-by-word highlighted captions rendered with custom fonts, colors, and motion graphics.
8. **Complex Video Transitions:** GPU-rendered cross-dissolves, light leaks, and dynamic motion transitions between cuts.

---

## 5. Scope Reduction & Contingency Gates

If unexpected technical roadblocks or platform incompatibilities arise during the 30-day development window, the following **controlled scope reductions** are pre-authorized by the architecture team:

| Potential Bottleneck | Permitted Scope Reduction | Prohibited Reduction (Never Allowed) |
|---|---|---|
| Native WebView playback has codec stutter on older Linux distros. | Fall back to automatically rendering a lightweight 720p H.264 preview proxy on import. | Never ship with broken, out-of-sync, or freezing preview playback. |
| Multi-format export matrix (MOV/MKV output) requires complex muxing logic. | Restrict export strictly to universal MP4 (H.264/AAC) for v0.1.0 Beta. | Never ship an export engine that outputs out-of-sync audio or corrupted video streams. |
| Waveform generation on 2+ hour files takes > 15 seconds. | Display progressive chunked waveform loading while allowing instant video playback. | Never block the UI thread or cause an out-of-memory crash. |
| Advanced keyboard shortcut remapping takes too long to build. | Hardcode clean, industry-standard shortcut defaults (`Space`, `S`, `Delete`, `Z`, `Ctrl+S`). | Never remove the ability to edit via keyboard shortcuts. |

> [!WARNING]
> **Non-Negotiable Baseline:** The 30-day timeline target must NEVER be preserved by compromising data safety (e.g., touching source files), skipping export verification, or bypassing error handling. If critical bugs remain in P0 features at Day 30, the beta release gate stays closed until fixed.
