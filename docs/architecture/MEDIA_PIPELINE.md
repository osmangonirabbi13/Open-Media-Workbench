# OpenTake — Media Ingestion and Probing Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 6  
**Author:** OpenTake Core Architecture & Media Pipeline Lead  
**Date:** 2026-10-13  

---

## 1. Media Source Lifecycle

When a creator imports a video file into OpenTake, the media moves through a deterministic 10-stage lifecycle designed to guarantee data integrity, accurate metadata, and safe playback:

```
[1. User File Ingestion] (Drag-and-Drop or Native File Picker)
         │
         ▼
[2. Filesystem Validation] (Permissions, non-zero size, path canonicalization)
         │
         ▼
[3. FFprobe Execution] (Structured JSON probe; 5-second execution timeout)
         │
         ▼
[4. Stream Enumeration] (Identify primary video and audio stream indexes)
         │
         ▼
[5. Compatibility Evaluation] (Verify container and codec support matrix)
         │
         ▼
[6. Metadata Normalization] (Map timestamps, frame rates, DAR, rotation)
         │
         ▼
[7. Source Identity Fingerprinting] (Fast non-blocking identity generation)
         │
         ▼
[8. Preview Mode Decision] (Direct WebView `<video>` vs Proxy trigger)
         │
         ▼
[9. Downstream Task Unlock] (Waveform extraction and silence analysis enabled)
         │
         ▼
[10. Original Media Immutable Guarantee] (Read-only lock verified; zero writes)
```

---

## 2. Media Metadata Data Structure

In `crates/opentake-core/src/media/metadata.rs`, media metadata is extracted via FFprobe JSON output into strongly typed Rust structs:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub source_id: uuid::Uuid,
    pub file_path: std::path::PathBuf,
    pub file_size_bytes: u64,
    pub last_modified_unix_secs: u64,
    pub fingerprint: SourceFingerprint,
    pub container_format: String,
    pub duration_us: i64, // Exact duration in microseconds
    pub start_time_us: i64, // Presentation start time offset
    pub video_stream: Option<VideoStreamMetadata>,
    pub audio_stream: Option<AudioStreamMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamMetadata {
    pub stream_index: usize,
    pub codec_name: String,
    pub profile: Option<String>,
    pub width: u32,
    pub height: u32,
    pub pixel_aspect_ratio: (u32, u32), // e.g., (1, 1)
    pub display_aspect_ratio: (u32, u32), // e.g., (16, 9)
    pub rotation_degrees: i32, // 0, 90, 180, 270 (from display matrix)
    pub avg_frame_rate: (u64, u64), // Rational numerator/denominator (e.g., 30000, 1001)
    pub r_frame_rate: (u64, u64),
    pub is_vfr: boolean, // Variable frame rate detected
    pub pixel_format: String, // e.g., "yuv420p"
    pub timebase: (u64, u64), // e.g., (1, 90000)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamMetadata {
    pub stream_index: usize,
    pub codec_name: String, // e.g., "aac", "mp3", "pcm_s16le"
    pub sample_rate_hz: u32, // e.g., 48000, 44100
    pub channels: u16, // e.g., 1 (mono), 2 (stereo)
    pub channel_layout: String, // e.g., "stereo", "mono"
    pub timebase: (u64, u64), // e.g., (1, 48000)
}
```

---

## 3. Comprehensive Format and Edge-Case Support Policy

| Media Ingestion Case | Technical Handling in v0.1.0 Beta | User-Facing Behavior |
|---|---|---|
| **MP4 (H.264 / AAC)** | **Primary Reference Format.** Direct playback via WebView; frame-accurate cuts during export. | Full functionality enabled immediately. |
| **MOV (H.264 / AAC or ProRes)** | Handled natively. If WebView fails to decode ProRes, fallback proxy is offered. | Ingests cleanly; proxy prompt if unplayable in preview. |
| **MKV (H.264 or H.265 / AAC or Opus)** | Ingests cleanly. Native WebKitGTK may lack MKV container parser; triggers preview proxy. | Ingests cleanly; automatically offers preview proxy. |
| **Variable Frame Rate (VFR)** | Detected via `r_frame_rate != avg_frame_rate`. Logged in metadata; normalized to CFR during final export. | Ingests normally; status badge indicates "VFR (Auto-Normalized on Export)". |
| **Missing Audio Stream** | Detected when audio stream count $= 0$. `audio_stream` set to `None`. | Ingests cleanly; silence detection disabled with banner: "No audio stream detected." Manual trimming works. |
| **Multiple Audio Streams** | Probing indexes all streams; defaults to Stream 0 (primary track). Multi-track mixer deferred to P1. | Primary audio track used for waveform and silence analysis. |
| **Rotated Video (e.g., Smartphone 90°)** | Rotation parsed from side data / display matrix (`rotation: 90`). Metadata preserved for export. | Preview and export respect orientation metadata. |
| **Non-Square Pixels (PAR $\ne$ 1:1)** | DAR parsed from container (`display_aspect_ratio`); SAR/PAR passed to export scale filter. | Correct display aspect ratio maintained in preview. |
| **Unsupported Codec (e.g., AV1 / Indeo)** | Prober detects unsupported codec string or missing decoder in host FFmpeg. | Ingestion rejected with clear dialog explaining supported formats (MP4/MOV/MKV H.264). |
| **Corrupted Media Container** | FFprobe exits with code $\ne 0$ or times out after 5 seconds. | Intercepted in Rust. Modal alert: "Corrupted or unreadable media file." Zero crash. |
| **Large Files (50+ GB / 4K 60fps)** | Probed via fast header inspection ($< 500\text{ ms}$). No full-file read or hashing. | Instant import; progress bar shown for background audio scan. |
| **Missing / Moved File on Project Load** | File existence check fails during `open_project`. | Project opens into "Media Offline" mode with "Relink Media" button. |

---

## 4. Source Identity & Change Detection Strategy

> [!IMPORTANT]
> **Performance Anti-Pattern:** Computing a full SHA-256 hash over a 20 GB raw video file on import causes 30–60 seconds of heavy disk I/O and freezes user excitement.

OpenTake implements a **fast two-stage source fingerprinting strategy**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceFingerprint {
    pub file_size_bytes: u64,
    pub last_modified_unix_secs: u64,
    /// Fast Blake3 hash over: first 64 KB (header) + last 64 KB (footer/moov atom)
    pub header_footer_hash: String,
}
```

### Fingerprinting Execution Rules:
1. **Import Time:** Reads the file size and modification timestamp, then hashes only the first 64 KB and last 64 KB of the file using `blake3`. This completes in **under 5 milliseconds**, even on 100 GB files over slow USB drives.
2. **Project Reload:** Checks file path, size, and the fast header/footer hash. If they match, the media is certified identical.
3. **Export Pre-Flight Check:** Verifies the fingerprint before initiating export to confirm the raw file was not swapped, modified, or truncated outside OpenTake.
4. **Full SHA-256 Hash:** Reserved exclusively for automated QA regression tests (`REQ-EDIT-001`) to empirically verify zero source file mutation.
