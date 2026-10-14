# Open Media Workbench

> **Free, open-source, offline-first desktop media recovery and AI localization studio.**

[![License](https://img.shields.io/badge/License-GPL_v3-blue.svg)](docs/decisions/ADR-0001.md)
[![Status](https://img.shields.io/badge/Status-STEP%2013%20Recovery%20Pipeline%20Complete-success.svg)](docs/reports/STEP_13_REPORT.md)
[![Architecture](https://img.shields.io/badge/Architecture-Dual%20Workspace-indigo.svg)](docs/architecture/OPEN_MEDIA_WORKBENCH.md)

Open Media Workbench is a free, open-source, offline-first desktop application designed for media recovery and local AI localization. Built with Tauri 2, Rust, and React 19, the workbench provides content creators, archivists, and video professionals with an uncompromising local-first studio that processes all media strictly on-device without cloud telemetry or mandatory accounts.

---

## 🏛️ Product Architecture: Dual Studio Model

Open Media Workbench unifies two specialized studio workspaces under a single high-performance desktop shell, sharing a common Rust media processing core:

1. **Repair Studio** *(Active Development — STEPS 12 & 13 Complete)*
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
   - **Future Reference Reconstruction (Scheduled for STEP 14):**
     - Reference-based MOOV atom reconstruction for unfinalized or interrupted recordings missing required sample tables.

2. **Localization Studio** *(Planned — STEP 11 Desktop UI Ready)*
   - Interactive subtitle editor and timing adjustment
   - Local, privacy-first AI speech-to-text transcription (Whisper)
   - On-device multilingual translation
   - Synchronized neural speech synthesis (voice dubbing)
   - Multi-track preview and dubbed video/subtitle export

---

## ⚠️ Important Recovery & Safety Guarantees

> [!IMPORTANT]
> **Zero-Byte Source Mutation Guarantee**
> All recovery operations execute on read-only file descriptors. Output files are generated in an isolated temporary staging location. Source media files are **never** modified, overwritten, truncated, or moved.

> [!NOTE]
> **Lossless Stream Copy vs Re-encoding**
> The STEP 13 recovery pipeline strictly copies valid compressed packets (`-c copy`) without lossy re-encoding or transcoding. If an elementary stream is unreadable or corrupted at the packet level, it is not silently transcoded into degraded quality.

> [!WARNING]
> **Missing MOOV Atom Boundary (STEP 14 Required)**
> MP4 and MOV files that lack a `moov` atom (e.g. power-loss during recording) cannot be demuxed or remuxed by standard FFmpeg commands because sample chunk offsets are absent. The recovery engine honestly flags this condition as requiring **STEP 14 Reference-Based Reconstruction** using a donor file recorded on the same device.

> [!TIP]
> **Strict No-Clobber Export Safety**
> When saving recovered media to user storage, Open Media Workbench requires native OS file dialog confirmation, forbids writing over the original source file, and rejects attempts to overwrite existing files or write to symlinks.

---

## ⚡ Current Project Status

> [!IMPORTANT]
> **Project State: STEP 13 Completed (Safe Media Remux & Stream Salvage Pipeline)**
> The dual-workspace desktop UI (STEP 11), video corruption diagnostics engine (STEP 12), and safe recovery pipeline (STEP 13) are fully implemented and verified with **145 automated Rust tests** and **30 TypeScript unit tests**. Next milestone: **STEP 14 (Reference-Based MP4/MOV moov Reconstruction)**.

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
