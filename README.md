# OpenTake

> **Free AI Video Editing Studio — Automatic Silence Removal & Rough-Cut Desktop Studio**

[![License](https://img.shields.io/badge/License-GPL_v3-blue.svg)](docs/decisions/ADR-0001.md)
[![Status](https://img.shields.io/badge/Status-STEP%202%20Architecture%20Complete-success.svg)](docs/reports/STEP_02_REPORT.md)
[![Roadmap](https://img.shields.io/badge/Roadmap-32%20Steps%20Approved-success.svg)](docs/roadmap/DEVELOPMENT_ROADMAP.md)

OpenTake is a free, open-source, offline-first desktop application designed to streamline spoken-word video editing. It provides content creators, podcasters, educators, and video editors with a fast, privacy-preserving studio to automatically detect and eliminate dead air, review cuts on an interactive timeline, and export high-quality, watermark-free MP4 videos.

---

## ⚡ Current Project Status

> [!IMPORTANT]
> **Project State: STEP 2 Completed (Architecture & Licensing Decisions)**  
> The project has completed its comprehensive technical architecture, IPC contracts, licensing analysis, and test design. All engineering blueprints are locked for **STEP 3 (Tauri + Rust + React Project Bootstrap)**. The application implementation has not yet begun.

- **Target Release:** v0.1.0 Beta
- **Development Schedule:** ~30 days (engineering target)
- **Primary Development Environment:** Ubuntu Linux x86_64
- **Primary Tech Stack:** Tauri 2, Rust (Tokio), React 19, TypeScript, Tailwind CSS, FFmpeg/FFprobe

---

## 🎯 Core Principles

- **100% Free Core:** No paywalls, artificial feature restrictions, or exported video limits.
- **Open Source:** Transparent, community-driven development under proposed GPL-3.0.
- **Privacy-First & Offline:** All video and audio processing happens entirely on your local machine. Zero mandatory accounts, zero cloud uploads.
- **Zero Watermarks:** Your content belongs entirely to you.
- **Non-Destructive Editing:** Your original raw recordings are strictly read-only and never modified.
- **Focused Rough-Cut Workflow:** Built specifically to solve the repetitive, tedious rough-cut phase before final delivery or NLE finishing.

---

## 📚 Documentation Index

All architectural specifications, requirements, and roadmaps are tracked within the [`docs/`](file:///home/osmangoni/Downloads/OpenTake/docs) directory:

### Requirements & Governance
| Document | Description |
|---|---|
| [Product Requirements (PRD)](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/PRODUCT_REQUIREMENTS.md) | Vision, target user personas, jobs to be done, primary workflows, and scenarios. |
| [Release Scope & Feature Lock](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/RELEASE_SCOPE.md) | Locked P0 features for v0.1.0 Beta; deferred P1 and P2 scope boundaries. |
| [Acceptance Criteria](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/ACCEPTANCE_CRITERIA.md) | Testable, measurable criteria and verification methods for every P0 requirement. |
| [Risk Register](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/RISK_REGISTER.md) | Tracking of legal, security, technical, and operational risks. |
| [Development Roadmap](file:///home/osmangoni/Downloads/OpenTake/docs/roadmap/DEVELOPMENT_ROADMAP.md) | Complete 32-step engineering roadmap across 8 milestones with release gates. |

### Technical Architecture Specifications
| Document | Description |
|---|---|
| [Rust Workspace Architecture](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/RUST_WORKSPACE.md) | Two-tier workspace blueprint (`crates/opentake-core` + `src-tauri`), modules, error propagation. |
| [Tauri 2 IPC & Security Contracts](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/TAURI_IPC_CONTRACTS.md) | Strongly typed request/response commands, 10 Hz throttled events, least-privilege capability model. |
| [Media Ingestion & Probing](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/MEDIA_PIPELINE.md) | 10-stage ingestion flow, FFprobe normalization, fast header/footer Blake3 fingerprinting. |
| [Timeline & EDL Model](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/TIMELINE_EDL.md) | Integer microsecond timebase (`i64`), frame/sample boundary snapping, interval algebra. |
| [Video Playback Strategy](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/PLAYBACK_STRATEGY.md) | Hybrid two-tier preview: direct HTML5 video via asset bridge + 720p H.264 proxy fallback. |
| [Audio Analysis & Waveforms](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/AUDIO_ANALYSIS.md) | Streaming 16kHz PCM decoding, multi-resolution LOD buckets, hysteresis silence DSP. |
| [Export Engine Pipeline](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/EXPORT_PIPELINE.md) | 14-stage export flow, FFmpeg filtergraph compiler, CFR normalization, provisional ±25 ms sync gate. |
| [Background Job Management](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/BACKGROUND_JOBS.md) | Async Tokio job state machine, process tree killing (PGID), cancellation cleanup. |
| [Project Format Specification](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/PROJECT_FORMAT.md) | Versioned `.opentake` JSON schema (v1), atomic save with fsync, missing media relinking. |
| [Cross-Platform Distribution](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/DISTRIBUTION_STRATEGY.md) | Packaging on Linux (AppImage/deb), Windows (NSIS), macOS (DMG), 10-point qualification matrix. |
| [Future AI Extension Boundary](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/AI_EXTENSION_BOUNDARY.md) | Post-beta P1 whisper.cpp trait interface, explicit consent downloads, weights licensing. |
| [Performance Budget](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/PERFORMANCE_BUDGET.md) | Provisional engineering thresholds vs measured results, memory caps (< 500 MB). |
| [Testing Strategy](file:///home/osmangoni/Downloads/OpenTake/docs/testing/TEST_STRATEGY.md) | Multi-tier test pyramid, synthetic fixture specifications, AV sync measurement harness. |

### Licensing & Supply Chain
| Document | Description |
|---|---|
| [FFmpeg Licensing & Distribution](file:///home/osmangoni/Downloads/OpenTake/docs/licensing/FFMPEG_DISTRIBUTION.md) | Rigorous analysis of `--enable-gpl` and `libx264` obligations; sidecar packaging strategy. |
| [Open-Source Project License](file:///home/osmangoni/Downloads/OpenTake/docs/licensing/OPEN_SOURCE_LICENSE.md) | Evaluation of MIT vs Apache vs GPL-3.0; recommendation of GPL-3.0-or-later. |
| [Dependency Supply Chain Policy](file:///home/osmangoni/Downloads/OpenTake/docs/security/DEPENDENCY_POLICY.md) | License allowlist, cargo/npm audit gates, committed lockfiles, CI token hardening. |

### Architecture Decision Records (ADRs)
| ADR | Title & Link | Status |
|---|---|---|
| ADR-0001 | [Initial Desktop Architecture and Product Scope](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0001.md) | Proposed |
| ADR-0002 | [Rust Workspace and Module Boundaries](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0002.md) | Proposed |
| ADR-0003 | [Tauri IPC and Security Model](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0003.md) | Proposed |
| ADR-0004 | [Timeline Timebase and EDL](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0004.md) | Proposed |
| ADR-0005 | [Playback and Proxy Strategy](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0005.md) | Proposed |
| ADR-0006 | [FFmpeg Process and Export Pipeline](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0006.md) | Proposed |
| ADR-0007 | [Project Serialization and Recovery](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0007.md) | Proposed |
| ADR-0008 | [FFmpeg Licensing and Distribution Strategy](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0008.md) | Proposed |
| ADR-0009 | [Background Jobs and Cancellation Protocol](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0009.md) | Proposed |
| ADR-0010 | [Cross-Platform Release Strategy](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0010.md) | Proposed |
| ADR-0011 | [Open-Source Licensing and Dependency Policy](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0011.md) | Proposed |

### Milestone Execution Reports
- [Step 1 Execution Report](file:///home/osmangoni/Downloads/OpenTake/docs/reports/STEP_01_REPORT.md)
- [Step 2 Execution Report](file:///home/osmangoni/Downloads/OpenTake/docs/reports/STEP_02_REPORT.md)

---

## 🗺️ 32-Step Development Roadmap at a Glance

```
Milestone 1 — Foundation (Days 1–4)
  ├── STEP 1: Requirements, Scope Lock & Acceptance Baseline  ◄ [COMPLETED]
  ├── STEP 2: Architecture & Licensing Decisions              ◄ [COMPLETED]
  ├── STEP 3: Tauri + Rust + React Project Bootstrap          ◄ [NEXT]
  └── STEP 4: App Shell, Navigation & Design System

Milestone 2 — Media Foundation (Days 5–8)
  ├── STEP 5: FFmpeg/FFprobe Integration
  ├── STEP 6: File Import & Metadata Inspection
  ├── STEP 7: Playback & Seeking Foundation
  └── STEP 8: Audio Waveform Generation

Milestone 3 — Editing Engine (Days 9–13)
  ├── STEP 9: Non-Destructive Editing Data Model
  ├── STEP 10: Visual Timeline & Manual Editing
  ├── STEP 11: Undo/Redo & Keyboard Shortcuts
  ├── STEP 12: Project Persistence & Reopen
  └── STEP 13: Preview-to-Timeline Synchronization

Milestone 4 — Automatic Editing (Days 14–17)
  ├── STEP 14: Audio Analysis Pipeline
  ├── STEP 15: Silence Detection Algorithm
  ├── STEP 16: Smart Cut Generation
  └── STEP 17: Automatic Edit Review UX

Milestone 5 — Export Engine (Days 18–21)
  ├── STEP 18: Export Timeline Compiler
  ├── STEP 19: MP4 Export & Quality Presets
  ├── STEP 20: Export Progress & Cancellation
  └── STEP 21: Audio/Video Sync Validation

Milestone 6 — Polish & Reliability (Days 22–25)
  ├── STEP 22: Onboarding, Settings & UX Polish
  ├── STEP 23: Performance, Memory & Large-File Safety
  ├── STEP 24: Security & Dependency Audit
  └── STEP 25: Automated Regression Tests

Milestone 7 — Open Source & Distribution (Days 26–28)
  ├── STEP 26: Open-Source Repository Documentation
  ├── STEP 27: Cross-Platform CI & Packaging
  └── STEP 28: Installer/Release Verification

Milestone 8 — Beta Launch (Days 29–30)
  ├── STEP 29: External Beta Testing & Critical Fixes
  ├── STEP 30: Launch Materials & Showcase
  ├── STEP 31: GitHub Beta Release
  └── STEP 32: Feedback & Post-Release Roadmap
```

---

## 🛠️ Verified Development Environment (Host)

The primary development environment is Linux x86_64:
- **Rust Toolchain:** rustc 1.99.0 / cargo 1.99.0
- **Node.js Environment:** Node.js v24.19.0 / npm 11.17.0
- **Multimedia Engine:** FFmpeg 8.0.1 / FFprobe 8.0.1
- **GUI Libraries:** GTK 3 (3.24.52), WebKitGTK 4.1 (2.52.6)

---

## 🤝 Community & Contributing

OpenTake welcomes contributors from the global open-source community. Full contributing guides, issue templates, and coding standards will be formally published in **STEP 26**.

To inspect the architecture and licensing decisions, check the [Step 2 Report](file:///home/osmangoni/Downloads/OpenTake/docs/reports/STEP_02_REPORT.md).
