# Open Media Workbench

> **Free, open-source, offline-first desktop media recovery and AI localization studio.**

[![Status](https://img.shields.io/badge/Status-STEP%2021%20Subtitle%20Styling%20%26%20Burn--In%20Export%20Complete-success.svg)](docs/reports/STEP_21_REPORT.md)
[![Architecture](https://img.shields.io/badge/Architecture-Dual%20Workspace-indigo.svg)](docs/architecture/OPEN_MEDIA_WORKBENCH.md)


Open Media Workbench is a free, open-source, offline-first desktop application designed for media recovery and local AI localization. Built with Tauri 2, Rust, and React 19, the workbench provides content creators, archivists, and video professionals with an uncompromising local-first studio that processes all media strictly on-device without cloud telemetry or mandatory accounts.

---

## 🏛️ Product Architecture: Dual Studio Model

Open Media Workbench unifies two specialized studio workspaces under a single high-performance desktop shell, sharing a common Rust media processing core:

1. **Repair Studio** *(Active Development — STEPS 12, 13, 14 & 15 Complete)*
   - **Video Corruption Diagnostics (STEP 12):**
     - **Container Identification:** Fast magic byte validation for MP4/MOV, MKV (EBML), and AVI (RIFF) with extension mismatch detection.
     - **ISO BMFF Structural Inspection:** Bounded, memory-safe atom parser (`ftyp`, `moov`, `mdat`, `moof`, `free/skip`) handling 32-bit, 64-bit extended sizes, and size-0 atoms without loading sample payloads into memory.
     - **Resilient Stream Probing:** FFprobe metadata inventory that handles unfinalized or missing headers gracefully without crashing.
     - **Quick vs Deep Scan Modes:** Instant structural inspection (<100ms) or bounded 30-second decode scan (`-f null -`) identifying non-monotonic DTS jumps, macroblock concealment, and corrupt NAL slices.
     - **Evidence-Based Findings:** Structured codes (`MP4_MOOV_MISSING`, `TIMESTAMP_NON_MONOTONIC_DTS`) with concrete observed evidence strings and confidence scores.
     - **Sanitized Diagnostic JSON Export:** Privacy-preserving report export with native save destination (scrubs internal user directory paths).
     - **Cooperative Cancellation:** Responsive background job management with leak-free child process termination.
   - **Video Recovery Pipeline (STEP 13 Implemented):**
     - **Lossless Container Remux:** Pure stream copy (`-c copy`) of readable elementary streams into freshly muxed, compliant containers (MP4, MKV, MOV, AVI) with `-movflags +faststart`.
     - **Recoverable Stream Salvage:** Conservative isolation of intact video or audio tracks when sister streams suffer from corruption or unreadable data (Video-Only, Audio-Only, or All Readable Tracks).
     - **Conservative Timestamp Rebuild:** Presentation timestamp regeneration (`+genpts`, `avoid_negative_ts make_zero`) resolving non-monotonic DTS/PTS jumps and audio/video desynchronization.
     - **Post-Recovery Bounded Validation:** Three-stage output verification checking non-empty file size, container recognition, stream loss detection, and bounded FFmpeg null-sink decode test.
     - **Secure Staging & Safe Finalization:** Recovery operations execute strictly in isolated, application-managed staging directories. Final output export enforces atomic no-clobber protection (preventing silent overwrites) and defends against symlink redirection attacks.
     - **Zero-Byte Source Mutation:** Original source media files are strictly read-only and never modified, renamed, or truncated under any circumstances.
     - **Recovered Asset Preview Integration:** Validated outputs are registered into the active media library as new authorized assets and immediately playable in the desktop preview player.
     - **Versioned Recovery Reports:** Structured JSON reports detailing processing status, stream inventories, observed errors, duration deltas, and known limitations.
   - **Interrupted MP4/MOV Reconstruction Engine (STEP 14 Implemented):**
     - **Reference-Based Reconstruction:** Recovers unfinalized or interrupted recordings missing the index atom (`moov`) by pairing the damaged file with an authorized healthy reference recording from the same device and configuration.
     - **Donor Compatibility Analysis:** Pre-flight inspection comparing container family, video codec, profile, resolution, frame rate, timebase, and audio channels before attempting reconstruction.
     - **Dual Independent Authorization:** Both damaged and reference media files must be independently authorized via native file dialogs and verified through `MediaAuthorizationRegistry`. Arbitrary filesystem paths from untrusted IPC callers are strictly rejected.
     - **Built-in Safe Rust Reconstructor:** Out-of-the-box native reconstruction engine that extracts donor SPS/PPS parameter sets, scans damaged `mdat` payloads for length-prefixed and Annex-B NAL units, synthesizes sample tables, and produces compliant, playable ISO BMFF containers without requiring external C++ tools or third-party binary downloads.
     - **Pluggable Engine Adapter:** Extensible architecture supporting both the native safe Rust engine and external `untrunc` CLI tools if discovered on system `$PATH`.
     - **Zero-Byte Source & Reference Immutability:** Pre- and post-recovery SHA-256 hashes confirm both the damaged source and reference recording remain 100% byte-identical.
     - **Bounded Output Validation:** Reconstructed candidates are inspected for container recognition, stream inventory, and bounded decode playback testing (`-f null -`).
   - **Professional Recovery Preview, Validation & Reports (STEP 15 Implemented):**
     - **Secure Staged Candidate Preview:** Preview recovered media safely prior to final export using `SourceProvenance::StagedRecoveryCandidate` and opaque custom protocol tokens (`opentake-media://`) without exposing staging filesystem paths or expanding Tauri capabilities.
     - **Multi-Mode Before/After Comparison Player:** Side-by-side synchronized comparison (`Original | Recovered`) with linked or independent seek modes, as well as single recovered-only and original-only views. Unplayable originals are handled honestly: metadata and diagnostic findings remain visible while recovered candidate plays independently.
     - **Advanced Multi-Tier Output Validation:** Distinguishes `MetadataOnly`, `SampledDecode` (with explicit disclosure of sample coverage), and user-initiated on-demand `FullDecode` (verifying 100% of bitstream frames without unbounded automatic execution).
     - **Timeline Difference & Integrity Review:** Compares stream durations and start offsets using verified timestamp evidence; missing nominal durations are never fabricated.
     - **Persistent Local Recovery History:** Crash-safe, atomic versioned JSON ledger tracking up to 50 previous recovery attempts with status, validation levels, timestamps, and manual history clear action.
     - **Sanitized JSON & Standalone Offline HTML Reports:** User-initiated report export featuring stable machine-readable finding codes, strict Content-Security-Policy, offline embedded dark CSS, XSS escaping, and path privacy sanitization.

2. **Localization Studio** *(Active Development — STEPS 16, 17, 19 & 20 Implemented)*
   - **Canonical Subtitle Domain Model (STEP 16 Implemented):**
     - **Strict Microsecond Timestamps:** Microsecond-accurate integer representation (`MediaTimeUs`), preventing floating-point drift across playback ticks.
     - **Robust SubRip (.srt) Support:** Parsing and serialization supporting UTF-8 BOM, CRLF/LF line endings, multiline cues, and tolerant numbering.
     - **Standards-Aware WebVTT (.vtt) Support:** Parses WebVTT signatures, skips `NOTE` comments, inspects cue settings (`line:`, `position:`, `align:`, `size:`, `vertical:`), and extracts `<v Speaker>` voice tags into structured speaker attributes.
     - **Full Unicode & Bengali (বাংলা) Preservation:** Zero byte corruption or entity truncation when processing non-Latin alphabets and complex Unicode scripts.
     - **Automated Validation Diagnostic Engine:** Inspects tracks and documents for empty cues, inverted intervals, duplicate IDs, out-of-order cues, duration anomalies, and media bounds violations.
     - **Safe Local Import & Provenance:** Native `rfd` file picker enforcing `SourceProvenance::SubtitleFileDialog`, 10MB file size ceiling, and 20,000 cue caps without arbitrary filesystem writes or media mutation.
     - **Live Video Preview Overlay & Synchronized Seek:** Clean plain-text subtitle overlay rendered over the HTML5 preview player with Closed Captions (CC) toggle; click-to-seek cue list navigation powered by $O(\log N)$ binary search lookup.
     - **Safe Atomic Export & Project Persistence:** Export to compliant `.srt` and `.vtt` formats with temporary file rename guarantees; versioned `.omwsub` project document format.
   - **Professional Subtitle Timeline Editor (STEP 17 Implemented):**
     - **Visual Timeline Cue Blocks:** Interactive cue blocks on a dedicated `S1 Subtitle` track lane precisely aligned with the audio waveform canvas and horizontal time ruler.
     - **Interactive Timeline Manipulation:** Drag cue bodies to translate timing; drag left and right boundary handles to resize start and end times in integer microseconds.
     - **Multi-Target Snapping Engine:** Snaps cue boundaries to current playhead, neighboring cue start/end boundaries, and timeline grid with visual amber snap indicators and threshold controls.
     - **Split & Merge Workflows:** Split cues at playhead with clean Unicode/Bengali word and newline division; merge adjacent cues with deterministic text concatenation and metadata preservation.
     - **Precise Timing Inspector:** Dedicated start/end timecode inputs (`HH:MM:SS.mmm`), duration display, configurable nudge buttons (-100ms, +100ms, -500ms, +500ms), and playhead sync buttons ("Start @ Playhead", "End @ Playhead").
     - **Inline Dialogue Editor:** Multiline text editor with live character counts, lines count, reading speed (chars/sec), and non-blocking readability warnings (>42 chars/line).
     - **Multi-Cue Batch Editing:** Ctrl/Cmd-click and Shift-click range selection, batch nudge timing shifts with atomic negative-time rejection, and batch deletion.
     - **Multi-Level Undo/Redo:** 50-step deterministic history stack with `Ctrl/Cmd+Z`, `Ctrl/Cmd+Shift+Z`, and `Ctrl/Cmd+Y` shortcuts, designed to never intercept native text input undo events.
     - **Autosave & Crash Recovery:** Dirty state tracking with debounced snapshots and automatic recovery prompts on launch protecting against abrupt crashes or power loss.
   - **Offline Whisper Transcription Engine (STEP 19 Implemented):**
     - **Genuine Local Speech-to-Text:** Real on-device transcription utilizing official `whisper.cpp` inference backends through secure managed subprocess isolation. Zero cloud APIs, paid speech recognition tokens, telemetry, or mandatory accounts.
     - **Shared AI Model Integration:** Reuses installed GGML models (`whisper-tiny`, `whisper-base`) directly from the shared STEP 18 AI Model Manager repository. Zero redundant model downloads.
     - **Audio Preprocessing Pipeline:** Secure non-shell FFmpeg transformation staging 16 kHz mono signed 16-bit PCM WAV with automatic cleanup and cooperative cancellation.
     - **Multilingual & English Model Discrimination:** Supports automatic language detection, English, Bengali (বাংলা), and 90+ Whisper languages on multilingual models; automatically disallows non-English transcription on English-only (`.en`) models.
     - **Canonical Subtitle Integration:** Parses structured runtime outputs into validated microsecond `SubtitleCue` segments, with non-destructive track addition to existing `.omwsub` project documents.
     - **Measured On-Device Speed:** Measured 770ms execution on CPU for 11-second audio test fixture (~14x realtime speed with Whisper Tiny).
   - **Word-Level Timestamp Alignment, Smart Segmentation & Karaoke Foundation (STEP 20 Implemented):**
     - **Genuine Word-Level Timing Extraction:** Extracts measured token offsets and acoustic probabilities directly from `whisper.cpp` structured output (`--output-json-full`) without external Python runtimes or cloud aligners.
     - **Explicit Provenance & Timing Quality:** Classifies timings as `WhisperNativeEstimate`, `ForcedAlignment`, `UserAdjusted`, or `Unaligned`, coupled with document-level `TimingQuality` ratings (`Aligned`, `Estimated`, `PartiallyAligned`, `Unavailable`).
     - **Smart Subtitle Segmentation:** Intelligent heuristic segmentation partitioning aligned words into broadcast-compliant subtitle cues guided by configurable policies: maximum characters per line (e.g. 42), max lines per cue (e.g. 2), maximum and minimum cue duration bounds (1.0s – 7.0s), reading speed (CPS), natural silence pause splits (e.g. 400ms), and sentence punctuation breaks.
     - **Unicode-Safe Bengali Segmentation:** Complete preservation of Bengali (বাংলা) grapheme clusters, combining marks, nukta, virama, and Dari sentence punctuation (`।`, `॥`) without erroneous mid-word boundary cuts.
     - **Evidence-Based Timing Refinement:** Reversible boundary snapping matching cue start and end times to contained first and last word timestamps with minimum duration padding.
     - **Live Word-Level Karaoke Preview:** Real-time visual word highlighting inside the video preview subtitle overlay following speech timestamps without `dangerouslySetInnerHTML`.
     - **Waveform Timeline Word Markers:** Scalable, zoom-responsive word boundary markers on the subtitle timeline track lane when zoomed in (`>= 5%` cue width).
     - **Interactive Alignment Review & Manual Correction:** Dedicated `WordAlignmentPanel` for inspecting words, adjusting millisecond timestamps, seeking audio to word boundaries, and non-destructively applying formatted tracks to the project.
     - **Linear Performance Scaling:** Measured 100 words in 0.275ms, 1,000 words in 1.714ms, and 10,000 words in 20.024ms.
     - **Preservation & Export Compatibility:** Losslessly persists word timing metadata in `.omwsub` project format, migrates legacy schemas without data loss, and exports clean, compliant `.srt` and `.vtt` caption files.
   - **Professional Subtitle Styling & Video Burn-In Export (STEP 21 Implemented):**
     - **Canonical Subtitle Style Domain:** Fully typed style configurations controlling typography (font family, size, weight, italic), colors (text, outline, shadow, background box, word highlight), outline thickness, shadow offset, background opacity, frame-relative positioning (BottomCenter, Center, TopCenter, etc.), and animation presets.
     - **8 Professional Built-in Presets:** Clean Minimal, Cinematic, Modern Outline, Social Bold, Karaoke Highlight, Documentary, Accessible High Contrast, and Soft Background Box, alongside user-defined custom presets persisted locally.
     - **Local System Font Discovery & Bengali Support:** Native font discovery (`fc-list`) with fallback defaults and explicit Bengali font detection (`Noto Sans Bengali`, `Noto Serif Bengali`).
     - **Secure ASS Subtitle Generation:** Advanced SubStation Alpha generation with centisecond timing rounding, safe brace and character escaping, dialogue formatting, and animation tags (`\fad`, `\fscx`).
     - **Real Word-Level Karaoke:** Timing-accurate `\kf` karaoke tags powered by genuine word timing from STEP 20.
     - **Live React Subtitle Preview:** Instant dynamic styling updates on the video overlay adhering to canonical styles with live karaoke word highlighting.
     - **Production FFmpeg Burn-In Video Pipeline:** Real video burn-in re-encoding via system FFmpeg with libass filter integration (`ass=` filter), structured non-shell arguments, resolution options, CRF quality presets, and audio policy handling.
     - **Multi-Stage Output Validation:** ffprobe-based container, stream, and frame decode verification ensuring captions are visibly burned in and not merely accompanied by metadata.
     - **Safe Staging & Source Immutability:** Rendering executes strictly into app-owned staging directories; finalized export requires native save destination; source media is byte-for-byte immutable (verified via SHA-256).
   - **On-Device Multilingual Translation** *(Planned STEP 22–23)*
   - **Synchronized Neural Speech Synthesis (Dubbing)** *(Planned STEP 24–28)*
   - **Multiplexed Soft Subtitle Export** *(Planned STEP 30)*

3. **Shared Local AI Model Manager** *(Infrastructure Foundation — STEP 18 Implemented)*
   - **Unified Workspace Model Repository:** Shared local model cache (`~/.local/share/open-media-workbench/ai-models`) serving both Localization Studio and future AI Shorts Studio without duplicating multi-gigabyte files.
   - **Small Installer Strategy:** Zero bundled AI model weights. Clean release desktop binaries (~24 MB) with zero automatic downloads on application startup.
   - **Curated & Cryptographically Verified Catalog:** Pinned upstream Git LFS models with strict SHA-256 verification (Whisper Tiny/Base for speech-to-text, Piper Lessac for speech synthesis, and Qwen 2.5 0.5B for local text generation).
   - **Explicit User Consent & Transparency:** Clear confirmation modal disclosing exact artifact sizes, available disk space, model licenses, and official sources before initiating any download.
   - **Resilient Staging Pipeline:** Direct streaming network-to-disk writes in 64 KiB chunks with running SHA-256 computation, download cancellation cleanup, and size-overflow guardrails.
   - **Storage Safety & Anti-Traversal:** Model ID path sanitization, symlink attack rejection, atomic registry transactions (`registry.json`), startup orphan cleanup, and in-use locking preventing accidental deletion during active jobs.
   - **Hardware & Capability Discovery:** Safe non-privileged Linux hardware probing via `/proc/meminfo` (RAM) and POSIX `statvfs` (disk storage) with honest reporting of unverified GPU acceleration states.
   - **100% Offline Resilience:** Core Repair Studio, video playback, and subtitle editing operate with zero AI dependencies; previously installed models and catalog remain fully accessible offline.

---

## ⚠️ Important Recovery & Safety Guarantees

> [!IMPORTANT]
> **Zero-Byte Source Mutation Guarantee**
> All recovery, reconstruction, and subtitle operations execute strictly on read-only file descriptors. Output files are generated in isolated temporary staging locations or atomic swap paths. Source media files are **never** modified, overwritten, truncated, or moved.

> [!NOTE]
> **Lossless Stream Copy vs Re-encoding**
> The recovery and reconstruction engines prioritize lossless stream copy (`-c copy`) without generational loss. Damaged bitstream packets are preserved or isolated; content is never silently transcoded into degraded quality.

> [!TIP]
> **Reference Video Requirements**
> For reference-based reconstruction, use a working, finalized video recorded on the exact same camera or software (e.g. same smartphone, OBS profile, drone, or action camera) with matching resolution and frame rate. The donor file provides the missing SPS/PPS codec configuration and timescale required to rebuild the movie index.

> [!WARNING]
> **Unsupported Recovery Cases (Best-Effort Policy)**
> Open Media Workbench makes no universal recovery promises. Files with empty `mdat` payloads (0 bytes of media data), encrypted or DRM-protected content, or proprietary camera-specific raw bitstreams cannot be reconstructed.

> [!TIP]
> **Strict No-Clobber Export Safety**
> When saving recovered media or subtitle files to user storage, Open Media Workbench requires native OS file dialog confirmation, forbids writing over the original source or reference files, and rejects attempts to overwrite existing files without explicit user confirmation.

---

## ⚡ Current Project Status

> [!IMPORTANT]
> **Project State: STEP 19 Completed (Offline Whisper Transcription Engine: Real Local Speech-to-Text & Automatic Subtitle Generation)**
> The dual-workspace desktop UI (STEP 11), corruption diagnostics engine (STEP 12), safe remux/salvage pipeline (STEP 13), reference-based reconstruction engine (STEP 14), recovery review/reporting pipeline (STEP 15), subtitle foundation (STEP 16), professional subtitle editor (STEP 17), shared AI model manager (STEP 18), and offline Whisper transcription engine (STEP 19) are fully implemented and verified with **260 automated Rust tests** and **71 TypeScript unit tests**. Next milestone: **STEP 20**.


- **Target Release:** v0.1.0 Beta
- **Development Schedule:** ~30 days (engineering target)
- **Primary Development Environment:** Ubuntu Linux x86_64
- **Primary Tech Stack:** Tauri 2, Rust (Tokio), React 19, TypeScript, Tailwind CSS v4, FFmpeg/FFprobe

---

## 🚀 Developer Quickstart

### Prerequisites (Ubuntu / Debian Linux)
Ensure standard native build tools, FFmpeg, and WebKitGTK libraries are installed:
```bash
sudo apt update
sudo apt install -y build-essential curl pkg-config libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev ffmpeg
```

### Installation & Local Run
```bash
# 1. Install Node.js frontend dependencies
npm install

# 2. Run the native desktop application in development mode
npm run tauri dev

# 3. Run frontend standalone preview (browser only)
npm run dev
```

### Verification & Quality Checks
```bash
# Frontend typecheck, lint, and unit tests
npm run typecheck
npm run lint
npm test
npm run build

# Rust workspace compilation, unit tests, clippy, and binary build
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build -p opentake-desktop
```

---

## 📜 Licensing & Distribution

Open Media Workbench is distributed under the **GNU General Public License v3.0 or later (GPL-3.0-or-later)**.
- See [LICENSE](LICENSE) for the full license text.
- See [docs/architecture/LICENSING_AND_DISTRIBUTION.md](docs/architecture/LICENSING_AND_DISTRIBUTION.md) for licensing architecture and compliance invariants.
- See [docs/architecture/MP4_RECONSTRUCTION_ENGINE.md](docs/architecture/MP4_RECONSTRUCTION_ENGINE.md) for reconstruction engine architecture and licensing evidence.
