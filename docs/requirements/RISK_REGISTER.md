# OpenTake — Licensing, Security & Engineering Risk Register

**Document Version:** 1.0.0  
**Release Target:** v0.1.0 Beta  
**Status:** Active Tracking  
**Author:** OpenTake Security & Legal Risk Architecture Team  
**Date:** 2026-10-13  

---

## 1. Risk Management Framework

Risks are categorized across four critical domains:
- **LEGAL:** Licensing, redistribution compliance, patent considerations.
- **SECURITY:** Process safety, untrusted media parsing, filesystem sandboxing.
- **TECHNICAL:** Media synchronization, memory limits, platform playback parity.
- **OPERATIONAL:** Single-developer 30-day timeline, distribution signing, packaging.

### Risk Scoring Matrix
- **Probability:** Low (L), Medium (M), High (H)
- **Severity:** Low (L), Medium (M), High (H), Critical (C)
- **Current Status:** Open, In Review, Mitigated

---

## 2. Comprehensive Risk Register

| Risk ID | Category | Description | Prob | Sev | Mitigation Strategy | Owner | Release Relevance | Verification Requirement | Status |
|---|---|---|---|---|---|---|---|---|---|
| **RSK-LEG-001** | LEGAL | **FFmpeg GPL/LGPL Redistribution Obligations.** Distributing FFmpeg binaries compiled with `--enable-gpl` subjects the distributed bundle to GNU GPL v2/v3 terms. If OpenTake links or bundles GPL binaries, source code disclosure and licensing compliance must be fulfilled. | High | High | Keep clean process separation (invoking FFmpeg CLI via child process, not in-process C linking). License OpenTake under a compatible open-source license (GPL-3.0-or-later proposed in ADR-0011). Provide automated source attribution scripts in STEP 27. | Lead Maintainer | Beta Blocker | Formal legal license matrix review completed in STEP 2 (`FFMPEG_DISTRIBUTION.md`). | Mitigated by Architecture |
| **RSK-LEG-002** | LEGAL | **libx264 and libx265 Encoder Licensing.** libx264 is licensed under GPL v2+ with commercial patent implications. Compiling it into distributed binaries enforces GPL on the binary distributor. | High | High | For v0.1.0 Beta, rely on system/user-provided FFmpeg on Linux, and vetted GPL-compliant sidecar builds with clear upstream source attribution for Windows/macOS (ADR-0008). | Lead Maintainer | Beta Blocker | Verified FFmpeg build flags and licensing declarations in STEP 2 (`FFMPEG_DISTRIBUTION.md`). | Mitigated by Architecture |
| **RSK-LEG-003** | LEGAL | **H.264 & AAC Patent Pool Considerations (MPEG LA / Via Licensing).** Distributing commercial H.264 encoders/decoders can trigger royalty claims under patent pool terms. Free, non-commercial open-source distribution generally qualifies for exemptions or falls below minimum thresholds, but requires disclosure. | Low | High | Clear open-source disclaimer. Rely on system codecs and un-monetized open-source distribution. No commercial license fees charged to users. | Legal / Maintainer | Pre-Beta | Legal audit check of open-source packaging precedents (OBS Studio, HandBrake). | In Review |
| **RSK-LEG-004** | LEGAL | **Future AI Model Licensing (Post-Beta Whisper).** OpenAI Whisper model weights have specific usage terms (MIT), while certain fine-tuned models carry non-commercial restrictions. | Low | Medium | Restrict future P1 model downloads strictly to official MIT-licensed OpenAI GGML models (whisper.cpp). Do not bundle weights in the installer. | AI Engineer | Post-Beta | Verify model license files before enabling P1 downloader. | Open |
| **RSK-SEC-001** | SECURITY | **Untrusted Media Parsing & Parser Vulnerabilities.** Malformed or malicious video files can exploit buffer overflows in FFmpeg/FFprobe decoders (CVEs). | Medium | High | 1. Isolate parsing within separate unprivileged child processes. 2. Regularly update bundled FFmpeg binaries to latest patch releases. 3. Run probe processes with strict timeouts (5 seconds). 4. Use memory limits (`ulimit` / resource control). | Security Lead | Beta Blocker | Fuzzing with malformed media fixtures in STEP 24. | In Review |
| **RSK-SEC-002** | SECURITY | **Unsafe Child Process Invocation / Command Injection.** Constructing shell execution strings (`sh -c "ffmpeg -i $path"`) allows arbitrary command execution via crafted filenames. | High | Critical | **Absolute ban on shell string concatenation.** All process invocations must use typed argument arrays (`std::process::Command::arg()`). | Senior Rust Eng | Beta Blocker | Static grep audit forbidding `sh -c`, `bash -c`, or format strings in command builders. | Mitigated by Design |
| **RSK-SEC-003** | SECURITY | **Path Traversal & Filesystem Boundary Violation.** Manipulated project files or relative paths accessing arbitrary filesystem locations (e.g., `/etc/shadow` or `C:\Windows`). | Medium | High | Canonicalize all paths via `std::fs::canonicalize()`. Reject relative path components (`..`). Enforce allowed directories (user-selected media and explicit cache paths). | Security Lead | Beta Blocker | Automated unit tests fuzzing path inputs with traversal sequences. | In Review |
| **RSK-SEC-004** | SECURITY | **Resource Exhaustion Attacks (DoS).** Extremely large or maliciously crafted files (e.g., zip-bomb style media or 8K 120fps files) causing out-of-memory panics. | Medium | High | Streaming audio processing in bounded chunks (64 KB). Bounded multi-resolution waveform caches. Disk space pre-flight check before initiating export. | Senior Rust Eng | Beta Blocker | Memory profiling with 10 GB+ long video fixtures. | In Review |
| **RSK-SEC-005** | SECURITY | **FFmpeg Security Patch Cadence.** Host or bundled FFmpeg binaries becoming outdated with unpatched vulnerabilities. | Medium | Medium | Document supported FFmpeg baseline version (FFmpeg 6.1+ or 7.x/8.x). In release builds, verify minimum version check during application startup. | Security Lead | Post-Beta | Version check test during app initialization. | Open |
| **RSK-TEC-001** | TECHNICAL | **Audio/Video Desynchronization Across Spliced Edits.** Cumulative drift between video frames and audio samples across dozens of silence cuts. | High | Critical | Enforce integer microsecond timebase (`i64`). Use exact PTS normalization filtergraph (`setpts=PTS-STARTPTS`, `asetpts=PTS-STARTPTS`). Snap cuts to audio sample boundaries. | QA Lead / Rust Eng | Beta Blocker | Automated AV sync flash/beep verification test in STEP 21. | In Review |
| **RSK-TEC-002** | TECHNICAL | **Platform-Specific WebView Playback Codec Discrepancies.** Linux WebKitGTK requires system GStreamer plugins (`gstreamer1.0-libav`), whereas Windows WebView2 and macOS WKWebView use OS media frameworks. | High | High | Implement automatic preview proxy fallback pipeline (fast 720p H.264 generation) when native playback fails or codecs are missing. | Tauri Architect | Beta Blocker | Multi-platform manual playback smoke tests in STEP 7. | In Review |
| **RSK-TEC-003** | TECHNICAL | **Large File Temporary Scratch Storage Exhaustion.** Long multi-hour video exports filling up system temp partition (`/tmp` or `C:\Users\...\Temp`). | Medium | High | Pre-flight disk space calculation (estimated export size + 500 MB buffer). Allow user to configure custom temporary working directory in Settings. Automatic cleanup on exit or cancel. | Senior Rust Eng | Beta Blocker | Low-disk-space simulation test in STEP 23. | In Review |
| **RSK-TEC-004** | TECHNICAL | **Unverified GPU Acceleration Instability.** Relying on hardware encoders (NVENC, VAAPI, VideoToolbox) that fail or produce artifacts across diverse driver setups. | High | Medium | Default strictly to proven software encoding (`libx264`, `aac`) for v0.1.0 Beta. Defer hardware acceleration to P1. | Senior Rust Eng | Beta Blocker | Benchmark CPU encoding performance baseline in STEP 19. | Mitigated by Scope Lock |
| **RSK-OPS-001** | OPERATIONAL | **Single-Developer 30-Day Development Schedule Risk.** Complex scope expansions causing missed deadlines or half-baked features. | High | High | Strict P0 scope lock. No P1/P2 features permitted in beta. Modular 32-step roadmap with clear definitions of done at every step. Pre-authorized scope reductions if delayed. | Product Eng | Beta Blocker | Daily milestone progress tracking against the 32-step roadmap. | In Review |
| **RSK-OPS-002** | OPERATIONAL | **macOS Code Signing & Notarization Barriers.** macOS Gatekeeper blocks unsigned apps; requires paid Apple Developer Account ($99/year) and notarization pipeline. | High | Medium | For v0.1.0 Beta: Primary development & validation on Ubuntu Linux x86_64. Document self-signing / `xattr -cr` bypass for macOS community beta testers until formal Apple developer account is funded. | Open-Source Lead | Post-Beta | Documented unsigned installation guide in repository README. | In Review |
| **RSK-OPS-003** | OPERATIONAL | **Windows SmartScreen Warnings on Unsigned Installers.** Windows Defender SmartScreen shows scary "Unknown Publisher" prompt for installers lacking EV code signing certificate. | High | Low | Document bypass ("More info -> Run anyway") for early beta testers. Explore open-source code signing options (SignPath.io for open source) in STEP 27. | Open-Source Lead | Post-Beta | Test NSIS installer on clean Windows 11 VM in STEP 28. | Open |
| **RSK-OPS-004** | OPERATIONAL | **Bundled vs System-Provided FFmpeg Packaging.** Packaging 80 MB+ FFmpeg binaries inside desktop installers increases download size and cross-compilation complexity. | Medium | Medium | On Linux, detect and utilize system FFmpeg or bundle an AppImage runtime. On Windows/macOS, provide clear instructions or validated sidecar binaries. | Tauri Architect | Beta Blocker | Cross-platform build pipeline validation in STEP 27. | In Review |

---

## 3. High-Priority Action Items for Milestone 1 & 2

1. **Formalize License Choice in STEP 2:** Finalize whether OpenTake is licensed under GPL-3.0-or-later or AGPL-3.0 to align cleanly with GPL FFmpeg / libx264 binaries.
2. **Hardcode Injection Defense in STEP 5:** Create a centralized Rust `CommandRunner` wrapper that accepts typed slices `&[&str]` and forbids raw strings.
3. **Verify Host Toolchain in STEP 3:** Confirm WebKitGTK and GStreamer codec playback on Ubuntu 24.04/26.04 development host.
