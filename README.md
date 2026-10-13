# OpenTake

> **Free AI Video Editing Studio — Automatic Silence Removal & Rough-Cut Desktop Studio**

[![License](https://img.shields.io/badge/License-GPL_v3-blue.svg)](docs/decisions/ADR-0001.md)
[![Status](https://img.shields.io/badge/Status-STEP%201%20Baseline%20Lock-amber.svg)](docs/reports/STEP_01_REPORT.md)
[![Roadmap](https://img.shields.io/badge/Roadmap-32%20Steps%20Approved-success.svg)](docs/roadmap/DEVELOPMENT_ROADMAP.md)

OpenTake is a free, open-source, offline-first desktop application designed to streamline spoken-word video editing. It provides content creators, podcasters, educators, and video editors with a fast, privacy-preserving studio to automatically detect and eliminate dead air, review cuts on an interactive timeline, and export high-quality, watermark-free MP4 videos.

---

## ⚡ Current Project Status

> [!IMPORTANT]
> **Project State: STEP 1 Completed (Requirements, Scope Lock & Acceptance Baseline)**  
> The project is currently in the **foundation and architecture phase** of an approved 32-step engineering roadmap. Application implementation (Tauri/React scaffolding) begins in **STEP 3**. The editor is not yet functional.

- **Target Release:** v0.1.0 Beta
- **Development Schedule:** ~30 days (engineering target)
- **Primary Development Environment:** Ubuntu Linux x86_64
- **Primary Tech Stack:** Tauri 2, Rust (Tokio), React 19, TypeScript, Tailwind CSS, FFmpeg/FFprobe

---

## 🎯 Core Principles

- **100% Free Core:** No paywalls, artificial feature restrictions, or exported video limits.
- **Open Source:** Transparent, community-driven development.
- **Privacy-First & Offline:** All video and audio processing happens entirely on your local machine. Zero mandatory accounts, zero cloud uploads.
- **Zero Watermarks:** Your content belongs entirely to you.
- **Non-Destructive Editing:** Your original raw recordings are strictly read-only and never modified.
- **Focused Rough-Cut Workflow:** Built specifically to solve the repetitive, tedious rough-cut phase before final delivery or NLE finishing.

---

## 📚 Documentation Index

All architectural specifications, requirements, and roadmaps are tracked within the [`docs/`](file:///home/osmangoni/Downloads/OpenTake/docs) directory:

| Document | Description |
|---|---|
| [Product Requirements (PRD)](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/PRODUCT_REQUIREMENTS.md) | Vision, target user personas, jobs to be done, primary workflows, and scenarios. |
| [Release Scope & Feature Lock](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/RELEASE_SCOPE.md) | Locked P0 features for v0.1.0 Beta; deferred P1 and P2 scope boundaries. |
| [Technical Constraints](file:///home/osmangoni/Downloads/OpenTake/docs/architecture/TECHNICAL_CONSTRAINTS.md) | Rust/React responsibility split, non-destructive EDL model, timebase math, FFmpeg pipeline, and media policies. |
| [Acceptance Criteria](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/ACCEPTANCE_CRITERIA.md) | Testable, measurable criteria and verification methods for every P0 requirement. |
| [ADR-0001: Initial Architecture](file:///home/osmangoni/Downloads/OpenTake/docs/decisions/ADR-0001.md) | Architecture Decision Record evaluating Tauri 2, Rust, React, FFmpeg CLI, JSON persistence, and scope decisions. |
| [Risk Register](file:///home/osmangoni/Downloads/OpenTake/docs/requirements/RISK_REGISTER.md) | Comprehensive tracking of legal, security, technical, and operational risks. |
| [Development Roadmap](file:///home/osmangoni/Downloads/OpenTake/docs/roadmap/DEVELOPMENT_ROADMAP.md) | Complete 32-step engineering roadmap across 8 milestones with definitions of done and release gates. |
| [Step 1 Execution Report](file:///home/osmangoni/Downloads/OpenTake/docs/reports/STEP_01_REPORT.md) | Official execution audit and readiness report for Step 1. |

---

## 🗺️ 32-Step Development Roadmap at a Glance

```
Milestone 1 — Foundation (Days 1–4)
  ├── STEP 1: Requirements, Scope Lock & Acceptance Baseline  ◄ [COMPLETED]
  ├── STEP 2: Architecture & Licensing Decisions
  ├── STEP 3: Tauri + Rust + React Project Bootstrap
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

To follow progress or review current architectural proposals, check the [Step 1 Report](file:///home/osmangoni/Downloads/OpenTake/docs/reports/STEP_01_REPORT.md).
