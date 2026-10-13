# OpenTake — Video Playback and Preview Strategy

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 7 & STEP 13  
**Author:** OpenTake Core Architecture & Desktop Lead  
**Date:** 2026-10-13  

---

## 1. Evaluation of Playback Architectures

Providing responsive, frame-accurate preview in a cross-platform desktop app without shipping a heavy custom video engine is a classic challenge. We evaluate three primary approaches:

| Criterion | Option A: Native HTML5 `<video>` via Asset Protocol | Option B: HTML5 Video via Transcoded Preview Proxy | Option C: Custom Native Video Surface (WGPU / libmpv) |
|---|---|---|---|
| **Platform Compatibility** | **High on Windows & macOS.** Variable on Linux (depends on system GStreamer decoders). | **100% Universal.** Transcoded to standard 720p H.264 baseline playable everywhere. | High across all platforms, but requires embedding native OS window handles into Tauri. |
| **Codec Support** | Limited to platform media engines (H.264 is universal; HEVC/ProRes/Opus varies). | Universal. Transcodes any input format to lightweight H.264. | Plays almost any format known to FFmpeg. |
| **Seeking Latency** | Very Fast ($< 80\text{ ms}$) on keyframe-rich H.264 files via HTTP 206 byte ranges. | **Instant ($< 30\text{ ms}$).** Fast GOP structure and small proxy file size. | Fast, but complex synchronization with web DOM. |
| **VFR Behavior** | Browser decoders handle VFR natively, but seeking to exact frame indices can have slight jitter. | Proxy is normalized to Constant Frame Rate (CFR), eliminating VFR jitter. | Handled via custom PTS queues. |
| **Memory / CPU Overhead** | **Very Low.** Uses hardware decoding via OS graphics pipeline. | Low during playback; requires one-time CPU transcode pass ($< 15\text{ s}$ per 10 min). | High. Custom texture uploads and memory management in Rust. |
| **Implementation Complexity** | **Very Low.** Standard React video component with Tauri asset streaming bridge. | **Low.** Standard FFmpeg transcode command into temp directory. | **Extremely High.** Multi-platform native window embedding breaks Tauri 2 abstractions. |
| **30-Day Feasibility** | **High.** | **High.** | **BLOCKED (Unrealistic for v0.1.0 timeline).** |

---

## 2. Recommended Strategy for v0.1.0 Beta

OpenTake adopts a **Hybrid Two-Tier Preview Pipeline**:

### Tier 1 (Primary Route): Direct Native HTML5 Video
For all standard MP4 and MOV files with H.264 video and AAC audio, OpenTake serves the raw media file directly to the `<video>` element via Tauri's registered custom protocol (`asset://` or streaming bridge with HTTP 206 Partial Content support). This delivers **instant playback with zero waiting time and zero disk overhead**.

### Tier 2 (Automatic Fallback Route): Lightweight Preview Proxy
If the media prober detects an unplayable container/codec combination (e.g., MKV with Opus on Linux, ProRes 422, or 10-bit HEVC), or if the `<video>` element throws a decode error, OpenTake offers a **Fast Preview Proxy**:
- **Format:** 720p ($1280 \times 720$), H.264 baseline profile, ultrafast preset, stereo AAC, Constant Frame Rate (30 fps).
- **Command:**
  ```text
  ffmpeg -y -i input.mkv -vf "scale=-2:720" -c:v libx264 -preset ultrafast -crf 26 -c:a aac -b:a 128k proxy.mp4
  ```
- **Storage:** Stored in `~/.cache/opentake/proxies/<source_uuid>_720p.mp4`.
- **Architectural Rule:** The proxy is used **strictly for visual presentation in the viewport**. All timeline cut decisions, timecode calculations, and final exports continue to reference the **raw source file**.

---

## 3. Playback Synchronization & Cut-Skipping Engine

During playback with the **"Skip Silence / Preview Cuts"** mode enabled:

```
[Playhead at 00:05.00] ──► Playing Segment 1
         │
         ▼
[Playhead reaches 00:08.50] (End of Segment 1; Cut gap between 08.50 and 12.00)
         │
         ▼
[Time Monitor Hook intercepts] ──► `<video>.currentTime = 12.00` (Start of Segment 2)
         │
         ▼
[Playhead resumes at 00:12.00] ──► Playing Segment 2 seamlessly
```

### Technical Synchronization Mechanism:
1. A lightweight `requestAnimationFrame` loop monitors `<video>.currentTime` in the React layer.
2. The current playback timestamp is matched against the active EDL retained ranges.
3. When the playhead reaches within $30\text{ ms}$ of `source_end_us` of range $k$, the player immediately executes:
   ```javascript
   videoElement.currentTime = nextRange.source_start_us / 1_000_000;
   ```
4. This produces a fluid rough-cut auditioning experience without requiring intermediate rendering.

---

## 4. Differentiating Interactive Preview from Final Export

| Characteristic | Interactive Desktop Preview | Final Export Engine |
|---|---|---|
| **Precision** | Approximate ($\pm 1\text{ video frame}$ display jitter due to browser decode scheduling). | **Exact.** Mathematically deterministic frame-by-frame filtergraph rendering. |
| **Audio Splicing** | Browser audio stream seeks instantaneously (may have minor audio crossfade smoothing). | **Frame-Accurate.** Sample-aligned `atrim` and `asetpts` concatenations. |
| **Resolution** | Scaled to preview viewport (or 720p proxy). | Full user-configured resolution (1080p, 4K, or source match). |
| **Speed** | 1.0x Real-time playback. | Multi-threaded offline render as fast as hardware allows. |
