# OpenTake — Performance and Memory Budget Specification

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Engineering Baseline & Calibration Blueprint  
**Author:** OpenTake Systems Performance Architect  
**Date:** 2026-10-13  

---

## 1. Performance Metric Categories

To maintain scientific rigor and avoid fabricated claims, all performance specifications are strictly divided into two distinct categories:

1. **[PROVISIONAL TARGET]:** Architectural performance thresholds required to deliver a responsive, production-quality user experience. These serve as engineering constraints during development.
2. **[MEASURED RESULT]:** Empirically verified benchmarks measured using automated scripts on defined hardware fixtures. *(Populated in STEP 21, STEP 23, and STEP 25).*

---

## 2. Performance & Resource Budget Matrix

| Operational Metric | Provisional Engineering Target | Verified Measured Result | Measurement Plan & Calibration Procedure |
|---|---|---|---|
| **Cold Application Startup** | $< 1.5\text{ seconds}$ to interactive desktop ready state. | *Pending STEP 3 Bootstrap* | Measured from launch command to Tauri window `DOMContentLoaded` event. |
| **Media Probing (1080p MP4, 10 GB)** | $< 1.0\text{ second}$ to extract full stream metadata. | *Pending STEP 6 Ingestion* | Wall-clock execution time of `ffprobe` process runner. |
| **Waveform Generation (10 min media)** | $< 3.0\text{ seconds}$ background scan time. | *Pending STEP 8 Waveform* | Timed from PCM stream spawn to completion of LOD Level 1 bucket generation. |
| **Waveform Generation (2 hour media)** | $< 25.0\text{ seconds}$ total background scan. | *Pending STEP 8 Waveform* | Tested with synthetic 2-hour audio fixture; progress rendered at 10 Hz. |
| **Silence DSP Scan Speed** | $> 15\times\text{ real-time speed}$ (1 hour analyzed in $< 4\text{ min}$). | *Pending STEP 14 & 15* | Measured in Rust DSP loop using sliding-window RMS algorithm. |
| **Timeline Scrub Latency** | $\le 150\text{ ms}$ from playhead move to `<video>` frame update. | *Pending STEP 7 Playback* | Time delta from input drag event to HTML5 video `seeked` event. |
| **Editing Operation Latency (Split/Trim)**| $\le 16\text{ ms}$ (within a single 60 FPS animation frame). | *Pending STEP 10 Timeline* | Pure Rust interval partition time + React state update. |
| **Undo / Redo Latency** | $\le 16\text{ ms}$ to restore previous state snapshot. | *Pending STEP 11 History* | History stack pop/push and EDL reconciliation time. |
| **Peak Memory Consumption (Idle)** | $< 100\text{ MB}$ Resident Set Size (RSS). | *Pending STEP 3 Bootstrap* | Measured via Linux `/proc/[pid]/statm` or `ps -o rss`. |
| **Peak Memory (2-hour video editing)** | $< 500\text{ MB}$ Resident Set Size (RSS). | *Pending STEP 23 Safety* | Memory profiling during simultaneous playback and waveform render. |
| **Peak Memory (Export in Progress)** | $< 650\text{ MB}$ Resident Set Size (RSS). | *Pending STEP 19 Export* | Combined memory of Tauri desktop process and child FFmpeg process. |
| **Cancellation Response Time** | $\le 1{,}500\text{ ms}$ to kill FFmpeg and unlink temp files. | *Pending STEP 20 Cancel* | Elapsed time from `cmd_cancel_job` to child process termination confirmation. |
| **Export Rendering Speed (1080p CFR)** | $\ge 1.0\times\text{ real-time}$ on modern quad-core x86_64 CPU. | *Pending STEP 19 Export* | Benchmark of `libx264` preset `fast` on reference 1080p test video. |

---

## 3. Reference Test Media & Calibration Environment

When benchmarking begins in Milestone 2 and Milestone 5, all measurements will be executed against standardized fixtures and record host hardware specifications:

```text
Calibration Host Environment Record Template:
- CPU: AMD/Intel model name and core/thread count
- RAM: Total installed RAM (GB) and speed
- OS: Ubuntu Linux kernel version / Windows build / macOS version
- Disk: NVMe SSD / SATA SSD / HDD model and write throughput
- Graphics / Drivers: Mesa / NVIDIA driver version
- FFmpeg Build: Version and enabled compiler flags
```

### Standardized Fixture Specifications (Documented in `TEST_STRATEGY.md`):
- `fixture_short_1080p.mp4`: 2 minutes, 1080p, 29.97 fps CFR, 48 kHz stereo AAC.
- `fixture_long_1080p.mp4`: 120 minutes, 1080p, 30 fps CFR, 48 kHz stereo AAC.
- `fixture_vfr_screencap.mp4`: 15 minutes, OBS screen recording with variable frame rate.
