# OpenTake — Project File Format and Persistence Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 12  
**Author:** OpenTake Core Architecture & Persistence Lead  
**Date:** 2026-10-13  

---

## 1. Project Format Principles

The `.opentake` project file format is designed to be **lightweight, human-readable, versioned, portable, and durable**:

- **Strictly Separated from Caches:** Derived artifacts (audio waveforms, decoded PCM, preview proxies) are **never** stored inside `.opentake` files. The project file stores only durable authoring decisions.
- **Portability:** Contains both relative and canonical absolute paths to raw source media, allowing projects to be moved across folders or shared across drives.
- **Human-Readable JSON:** Formatted with standard indented JSON for easy inspection and debugging.
- **Atomic Persistence:** Saved using the write-to-temp-then-rename pattern to eliminate the risk of file corruption on unexpected power loss or crashes.

---

## 2. Canonical JSON Schema (`schema_version: 1`)

```json
{
  "$schema": "https://opentake.org/schemas/v1/project.json",
  "schema_version": 1,
  "app_version": "0.1.0-beta",
  "project_id": "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d",
  "project_name": "Tutorial_Episode_01",
  "created_at_unix_secs": 1776100000,
  "updated_at_unix_secs": 1776100450,
  "source_media": {
    "source_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "canonical_path": "/home/creator/Videos/raw_recording.mp4",
    "relative_path": "./raw_recording.mp4",
    "file_size_bytes": 4294967296,
    "last_modified_unix_secs": 1776099000,
    "duration_us": 600000000,
    "fingerprint": {
      "file_size_bytes": 4294967296,
      "last_modified_unix_secs": 1776099000,
      "header_footer_hash": "a1b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890"
    }
  },
  "timeline": {
    "video_fps_num": 30000,
    "video_fps_den": 1001,
    "audio_sample_rate_hz": 48000,
    "retained_ranges": [
      {
        "id": "e8a939f4-1845-4e78-9e12-421b8a531f82",
        "source_start_us": 1250000,
        "source_end_us": 8400000
      },
      {
        "id": "2d8f9c0e-4a65-4f33-85b2-7c32e921d741",
        "source_start_us": 11100000,
        "source_end_us": 19850000
      }
    ]
  },
  "silence_analysis_settings": {
    "threshold_dbfs": -35.0,
    "min_duration_ms": 500,
    "pre_padding_ms": 150,
    "post_padding_ms": 150
  },
  "export_preferences": {
    "preset": "1080p",
    "quality": "high"
  },
  "view_state": {
    "playhead_position_us": 1250000,
    "timeline_zoom_level": 1.0
  }
}
```

---

## 3. Durability & Atomic Save Protocol

To guarantee that project files are never corrupted even if the operating system crashes or loses power mid-save:

```
[1. Serialize to In-Memory JSON Buffer]
                     │
                     ▼
[2. Write to `<path>.opentake.tmp`]
                     │
                     ▼
[3. Issue `fsync` / `sync_all`] ── Flushes disk write buffers to physical storage
                     │
                     ▼
[4. Create Backup `<path>.opentake.bak`] (Previous good version)
                     │
                     ▼
[5. Atomic Filesystem Rename] ── `<path>.opentake.tmp` atomically replaces `<path>.opentake`
```

---

## 4. Missing Media Resolution & Security

### 4.1 Relinking Algorithm:
1. Try `relative_path` resolved relative to the directory containing the `.opentake` file.
2. Try `canonical_path` directly.
3. If both fail, enter **Media Offline Mode** and prompt the user with a native file picker to locate the missing source.
4. When selected, the new file's duration, video stream, and audio sample rate are validated against the stored metadata. If compatible, the project is relinked seamlessly.

### 4.2 Security Against Malicious Project Files:
- All paths loaded from project JSON are strictly parsed and validated against path traversal (`..`).
- Number of retained ranges is capped at $50{,}000$ to prevent denial-of-service memory exhaustion.
- Deserialization rejects negative timestamps or ranges where `source_start_us >= source_end_us`.
