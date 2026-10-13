# OpenTake — Cross-Platform Distribution and Packaging Strategy

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Architecture Blueprint for STEP 27 & STEP 28  
**Author:** OpenTake Release Engineering Lead  
**Date:** 2026-10-13  

---

## 1. Platform Packaging Architecture

OpenTake targets the three major desktop operating systems via **Tauri 2 bundle generators**:

```
┌────────────────────────────────────────────────────────────────────────┐
│                      GITHUB ACTIONS CI MATRIX                          │
├───────────────────┬────────────────────┬───────────────────────────────┤
│   ubuntu-latest   │   windows-latest   │         macos-latest          │
├───────────────────┼────────────────────┼───────────────────────────────┤
│  AppImage & .deb  │    NSIS (.exe)     │  DMG (Universal / arm64+x64)  │
│  Tauri Bundler    │    Tauri Bundler   │  Tauri Bundler                │
└───────────────────┴────────────────────┴───────────────────────────────┘
```

---

## 2. Platform Delivery Matrix & Tooling

| Platform | Target Architecture | Primary Package Format | FFmpeg Resolution Strategy | Runtime Prerequisites | Code Signing Status (v0.1.0 Beta) |
|---|---|---|---|---|---|
| **Ubuntu / Linux** | `x86_64` | **AppImage** (standalone) + **`.deb`** (Debian/Ubuntu) | `.deb` declares `Depends: ffmpeg`. AppImage can utilize host or bundled static binary. | `libwebkit2gtk-4.1-0`, `gstreamer1.0-libav` | Standard Linux unsigned packaging (GPG signed release assets). |
| **Windows** | `x86_64` | **NSIS Installer (`.exe`)** + Portable `.zip` | Bundled FFmpeg static sidecar (`ffmpeg-x86_64-pc-windows-msvc.exe`). | WebView2 Runtime (pre-installed on Windows 10/11). | Unsigned early beta; instructions provided for Windows SmartScreen warning. |
| **macOS Apple Silicon** | `aarch64` (M1/M2/M3/M4) | **DMG (`.dmg`)** | Bundled static FFmpeg sidecar (`ffmpeg-aarch64-apple-darwin`). | macOS 11.0 (Big Sur) or newer. | Unsigned community beta (`xattr -cr` bypass documented) until Apple account funded. |
| **macOS Intel** | `x86_64` | **DMG (`.dmg`)** | Bundled static FFmpeg sidecar (`ffmpeg-x86_64-apple-darwin`). | macOS 10.15 (Catalina) or newer. | Same as Apple Silicon. |

---

## 3. Platform Qualification Matrix (QA Release Gate)

Before tagging a public release on GitHub, packages must pass the **10-Point Qualification Matrix**:

| Stage | Ubuntu Linux x86_64 (Local Host) | Windows 11 VM / CI | macOS Apple Silicon (Tester) |
|---|---|---|---|
| **1. Build** | Verified Locally | Verified in CI | Verified in CI |
| **2. Install** | Verified Locally | Required before launch | Required before launch |
| **3. Launch & Window Render** | Verified Locally | Required before launch | Required before launch |
| **4. Media Import (MP4)** | Verified Locally | Required before launch | Required before launch |
| **5. Video Preview & Scrub** | Verified Locally | Required before launch | Required before launch |
| **6. Silence DSP Analysis** | Verified Locally | Required before launch | Required before launch |
| **7. Manual Split & Trim** | Verified Locally | Required before launch | Required before launch |
| **8. Project Save & Reopen**| Verified Locally | Required before launch | Required before launch |
| **9. Final MP4 Export** | Verified Locally | Required before launch | Required before launch |
| **10. Clean Uninstall** | Verified Locally | Required before launch | Required before launch |

### Verification Boundary Note:
- **Locally Verifiable Now:** Ubuntu Linux x86_64 on the developer workstation.
- **Requires Automated CI & External Testers:** Windows and macOS builds will be generated via GitHub Actions in STEP 27 and smoke-tested by community beta testers in STEP 29.
