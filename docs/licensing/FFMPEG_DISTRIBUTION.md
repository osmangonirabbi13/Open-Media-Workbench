# OpenTake — FFmpeg Licensing and Redistribution Evaluation

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Architecture Decision Package (Release Blocker)  
**Author:** OpenTake Software Licensing & Architecture Team  
**Date:** 2026-10-13  

> [!DISCLAIMER]
> **Not Legal Advice:** This document represents an engineering and architectural analysis of software licenses, patent pool policies, and packaging mechanisms. It does not constitute formal legal counsel.

---

## 1. Authoritative Citations & Legal Baseline

This evaluation is grounded in official documentation and licensing guidelines:
1. **FFmpeg Legal Guidelines:** [https://ffmpeg.org/legal.html](https://ffmpeg.org/legal.html) (Accessed 2026-10-13).
2. **FFmpeg Source License Matrix:** [https://ffmpeg.org/doxygen/8.1/md_LICENSE.html](https://ffmpeg.org/doxygen/8.1/md_LICENSE.html) (Accessed 2026-10-13).
3. **Tauri 2 Sidecar Architecture:** [https://tauri.app/develop/sidecar/](https://tauri.app/develop/sidecar/) (Accessed 2026-10-13).
4. **GNU General Public License v3 (GPLv3):** Free Software Foundation (FSF).

---

## 2. The Host Development Environment Reality

The developer host system reports the following verified FFmpeg build:
```text
ffmpeg version 8.0.1-3ubuntu2+esm4 Copyright (c) 2000-2025 the FFmpeg developers
built with gcc 15 (Ubuntu 15.2.0-16ubuntu1)
configuration: --enable-gpl --enable-libx264 --enable-libx265 ...
```

### Fact:
The inclusion of `--enable-gpl` and `--enable-libx264` permanently upgrades the effective license of this binary from LGPLv2.1+ to **GPLv2 / GPLv3**.

### Engineering Rule:
Redistributing this binary inside an OpenTake installer bundle directly triggers the reciprocal obligations of the GNU General Public License. Invoking FFmpeg as a separate process via `Command::new("ffmpeg")` isolates memory address spaces, but **the binary itself remains governed by the GPL**. If OpenTake redistributes a GPL FFmpeg binary, the distributor must provide the exact matching source code and build scripts for that FFmpeg build.

---

## 3. Evaluation of Four Redistribution Strategies

| Dimension | Option A: System FFmpeg (Zero Bundling) | Option B: Bundled GPL-Enabled FFmpeg Sidecar | Option C: Bundled LGPL-Only FFmpeg Sidecar | Option D: Platform-Native Hardware Backends |
|---|---|---|---|---|
| **Architectural Model** | OpenTake locates `ffmpeg` in user `$PATH` or prompts to install. | OpenTake bundles a hardened, static FFmpeg binary as a Tauri sidecar. | OpenTake bundles an FFmpeg build without `--enable-gpl` or `--enable-libx264`. | OpenTake delegates encoding to Media Foundation / VideoToolbox. |
| **Licensing Burden on OpenTake** | **Zero.** OpenTake redistributes zero third-party binaries. | **High but Manageable.** Must publish exact FFmpeg source code and build scripts on GitHub. | **Moderate.** Must provide relinking / object replacement mechanism. | Low directly; complex API agreements. |
| **H.264 Encoder Quality & Availability** | Excellent (`libx264` universally available on Linux/macOS). | **Best in Class (`libx264`).** Consistent high quality across all platforms. | **Severely Degraded.** Lacks `libx264`. Must use Cisco OpenH264 or native encoders. | Variable across hardware drivers. |
| **End-User Setup Experience** | **Poor.** Requires non-technical users to install FFmpeg via terminal or installer. | **Flawless.** Works out of the box with zero user configuration. | Good, but encoding quality is lower. | Good, but older PCs fail export. |
| **Linux Distro Compatibility** | High (native packages). | Requires bundling static GLIBC or Musl build. | Same as Option B. | Linux VAAPI setup is notoriously fragile. |
| **Patent Risk** | Shifted to host distribution / user. | Royalty considerations for commercial distributors. | Cisco covers MPEG LA royalties for OpenH264 binary downloads only. | Handled by OS vendor. |
| **30-Day Feasibility** | **High.** | **High.** | Moderate (codec hunting). | **BLOCKED (Too complex for v0.1.0).** |

---

## 4. Recommended Multi-Stage Architecture

### 4.1 Development-Time Strategy (STEP 3 through STEP 26)
- **Primary Strategy: Option A (System FFmpeg Discovery).**
- During development and local testing on Ubuntu Linux x86_64, OpenTake will dynamically discover and utilize the verified host FFmpeg 8.0.1 binary located in `/usr/bin/ffmpeg`.
- A fallback check in Rust (`media::probe::verify_ffmpeg_available()`) will confirm presence and version upon application launch.

### 4.2 Beta Distribution Strategy (STEP 27 & STEP 28)
- **Linux (`.deb` / AppImage):**
  - For `.deb`: Declare `ffmpeg (>= 4.4)` as a package dependency (`Depends: ffmpeg`). The package manager automatically installs the system FFmpeg.
  - For AppImage: Bundle an LGPL or GPL-compliant static build with explicit upstream build recipe.
- **Windows (`.msi` / NSIS) & macOS (`.dmg`):**
  - Adopt **Option B (Bundled GPL FFmpeg Sidecar)** using well-known, reproducible automated build pipelines (e.g., Gyan.dev or BtbN automated static builds).
  - OpenTake repository will host a dedicated `third_party/ffmpeg/` directory documenting the exact Git commit, configuration flags, license text, and download link for corresponding source tarballs.

---

## 5. Unresolved Legal & Patent Questions (Status Tracking)

| Issue ID | Legal / Licensing Question | Release Relevance | Status |
|---|---|---|---|
| **ISS-LIC-001** | Does distributing OpenTake under the GPL-3.0 license resolve all compatibility conflicts with bundled GPL FFmpeg sidecars? | Beta Blocker | **APPROVED (GPL-3.0 is 100% compatible).** |
| **ISS-LIC-002** | Does non-commercial open-source distribution of `libx264` qualify for MPEG LA H.264 royalty fee exemptions for free software under 100k annual distributions? | Pre-Beta | **REQUIRES REVIEW** before any commercial entity adopts the project. |
| **ISS-LIC-003** | Should macOS builds rely on native VideoToolbox or bundled FFmpeg sidecar to avoid Apple App Store GPL distribution conflicts? | Post-Beta | **DEFERRED (v0.1.0 Beta is distributed via GitHub Releases, not Apple App Store).** |
