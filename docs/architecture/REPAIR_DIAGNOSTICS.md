# Repair Studio — Production-Grade Video Corruption Diagnostics Engine

**Document Version:** 1.0.0  
**Status:** Completed (STEP 12)  
**Target Release:** v0.1.0 Beta  
**Scope:** Forensic, deterministic, read-only corruption diagnostics for Repair Studio.  

---

## 1. Architectural Overview

The Repair Studio diagnostics engine replaces placeholder experiences with a production-grade, evidence-based diagnostic analyzer designed to inspect damaged, unfinalized, or corrupted media without altering source files.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Repair Studio (React 19)                        │
│   ├── DiagnosticsPanel (Mode Toggle, Live Progress, Filterable Cards)  │
│   ├── RecoveryWorkflowPanel (Multi-Step Guided Recovery Flow)          │
│   └── Export Functionality (Sanitized Native JSON Export)              │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Tauri 2 IPC Commands
                                    │ (run_media_diagnostics, cancel_*)
┌───────────────────────────────────┴────────────────────────────────────┐
│                    Tauri Desktop Security Layer                        │
│   ├── Resolves Authorized Asset ID via MediaLibrarySession             │
│   └── Verifies Path Authorization in MediaAuthorizationRegistry        │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Native Rust Engine Call
┌───────────────────────────────────┴────────────────────────────────────┐
│                     Rust Diagnostics Core (`repair`)                   │
│   ├── container.rs   (Magic byte inspection: MP4, MOV, MKV, AVI)      │
│   ├── iso_bmff.rs    (Bounded top-level atom scanner: ftyp, moov, mdat)│
│   ├── streams.rs     (FFprobe track inventory & probe error classifier)│
│   ├── timestamps.rs  (Bounded FFmpeg null-sink decode & DTS analysis)  │
│   ├── diagnostics.rs (Orchestrator & evidence recommendation engine)   │
│   └── report.rs      (Canonical, versioned DiagnosticReport schema)    │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Core Security & Media Authorization Invariants

1. **Zero-Byte Mutation Guarantee:**
   Original media files are opened strictly in read-only mode (`fs::File::open`). The diagnostics engine never modifies, truncates, renames, writes into, or deletes the source file or its parent directory.

2. **Asset ID Resolution via Registry:**
   Diagnostic IPC commands accept an authorized `asset_id: String` rather than arbitrary filesystem paths.
   The path is retrieved from `MediaLibrarySession::get_asset_by_id`, and its cryptographic/canonical authorization is verified against `MediaAuthorizationRegistry::verify_authorization`.

3. **No Shell Interpolation or Broad Permissions:**
   All external tools (FFmpeg, FFprobe) are spawned via `opentake_core::media::process::run_managed_process` using argument vectors (`Vec<String>`) with an explicit `--` delimiter to prevent option injection.

4. **Cooperative Cancellation:**
   Long-running deep scans track an active `tokio_util::sync::CancellationToken` mapped per `asset_id`. If cancelled, running child processes are terminated immediately via POSIX process group isolation.

---

## 3. Container & Bitstream Inspection Domains

### 3.1 Container Signature Identification (`repair::container`)
- Reads the leading 64 bytes of candidate files.
- Inspects ISO BMFF signatures (`ftyp`, `moov`, `mdat`, `wide`, `free`, `skip`).
- Matches Matroska/WebM EBML headers (`0x1A 0x45 0xDF 0xA3`).
- Detects RIFF AVI forms (`RIFF .... AVI `).
- Flags filename extension mismatches (e.g. an MP4 renamed to `.mkv`) and truncated headers (< 12 bytes).

### 3.2 Bounded ISO BMFF Atom Scanner (`repair::iso_bmff`)
- Memory-bounded: seeks over box payloads (`mdat`) without allocating sample buffers in RAM.
- Parses 32-bit box lengths, 64-bit extended box lengths (`size == 1`), and size-zero boxes extending to EOF (`size == 0`).
- Prevents infinite loops on damaged files using `MAX_TOP_LEVEL_BOXES = 5000` and checked 64-bit addition (`checked_add`).
- Detects:
  - `MP4_MOOV_MISSING`: Intact `mdat` payload but missing `moov` atom (crash during recording).
  - `MP4_BOX_TRUNCATED_BEFORE_END`: Box declared length exceeds remaining file length.
  - `MP4_MOOV_AT_END_OF_FILE`: Explains standard camera recording layout.
  - `MP4_FRAGMENTED_CONTAINER`: Identifies movie fragment (`moof`) progressive recording.

### 3.3 Stream Analysis & Probe Resilience (`repair::streams`)
- Reuses the managed FFprobe runner.
- Probes audio/video track counts, codecs, resolutions, sample rates, and rational timebases.
- Resilient: if FFprobe exits with an error (e.g., `moov atom not found`), the error is captured and translated into a high-value structured finding (`PROBE_MOOV_ATOM_NOT_FOUND`, `Critical`), enabling honest partial reporting rather than an application crash.

### 3.4 Bounded Timestamp & Null-Sink Decode (`repair::timestamps`)
- Executed exclusively in `AnalysisMode::Deep`.
- Runs `ffmpeg -v warning -xerror -t 30 -i <file> -f null -` with a 45-second timeout and bounded stderr buffer (256 KiB).
- Analyzes stderr for:
  - Non-monotonic DTS backward jumps (`TIMESTAMP_NON_MONOTONIC_DTS`).
  - Bitstream slice corruption or missing SPS/PPS (`DECODE_BITSTREAM_NAL_CORRUPT`).
  - Macroblock concealment triggers (`DECODE_MACROBLOCK_CONCEALMENT`).
  - Broken GOP structures and missing reference pictures (`DECODE_MISSING_REFERENCE_FRAMES`).

---

## 4. Scan Modes: Quick vs Deep

| Attribute | Quick Mode | Deep Mode |
| :--- | :--- | :--- |
| **Execution Time** | < 100 ms | 1–3 seconds (bounded to 45s max) |
| **Signature Check** | Yes (first 64 bytes) | Yes |
| **Top-Level Atom Scan** | Yes (bounded seek) | Yes |
| **FFprobe Metadata** | Yes | Yes |
| **Bitstream Decode** | Skipped | Sampled first 30 seconds via `-f null -` |
| **Timestamp Continuity** | Metadata-based | Packet-level DTS/PTS disorder detection |
| **Resource Cost** | Minimal I/O, negligible CPU | Single bounded FFmpeg process |

---

## 5. Finding Severity & Evidence Semantics

The diagnostic engine strictly separates states:
- **Verified Healthy:** Explicitly observed compliant headers and clean 0-exit decode logs.
- **Confirmed Defect:** Concrete evidence observed at byte offset or timecode.
- **Incomplete / Partial:** Inspection halted or bounded to sample window.
- **Unknown:** File structure could not be interpreted by known parsers.

### Severity Classifications:
1. `Critical`: Prevents container demuxing or direct playback (e.g. missing `moov`, zero dimensions, empty file).
2. `Error`: Bitstream decoding errors, truncated payload boundaries, or non-monotonic timestamps.
3. `Warning`: Filename extension mismatch, variable frame rate (VFR), or broken GOP references.
4. `Info`: Informational structural observations (`moov` located after `mdat`, fragmented MP4).

---

## 6. Evidence-Based Recovery Recommendation Rules

The recommendation engine evaluates all findings before suggesting future actions:

1. **Reference-Based MP4/MOV Reconstruction (STEP 14):**
   - *Trigger:* `MP4_MOOV_MISSING` or `PROBE_MOOV_ATOM_NOT_FOUND` AND `mdat` payload detected.
   - *Recommendation:* Match donor file recorded on the same camera/settings to reconstruct `moov`.
   - *Status:* Scheduled for STEP 14.

2. **Lossless Container Remux (STEP 13):**
   - *Trigger:* Valid elementary streams detected with timestamp disorder or trailing box truncation, but headers present.
   - *Recommendation:* Losslessly repackage bitstreams into a clean MP4/MKV wrapper.
   - *Status:* Scheduled for STEP 13.

3. **Raw Bitstream Salvage (STEP 13):**
   - *Trigger:* Corrupted container header or unknown format with file size > 1 MB.
   - *Recommendation:* Carve raw H.264/H.265 NAL units or ADTS frames directly from binary payload.
   - *Status:* Scheduled for STEP 13.

4. **File Integrity Verification:**
   - *Trigger:* File size is 0 bytes or completely unreadable.
   - *Recommendation:* Re-acquire source from camera storage card.

---

## 7. Report Export & Sanitization

- Reports are exported as standard JSON complying with `DiagnosticReport` Schema Version 1.
- Native OS save dialogs ensure files are written only to user-chosen directories.
- Internal home paths (`/home/username`) are sanitized to `~` prior to serialization.
