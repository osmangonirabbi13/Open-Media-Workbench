# Repair Studio: Safe Video Recovery Architecture

**Module:** `crates/opentake-core/src/repair/`
**Document Version:** 1.0.0 (STEP 13)
**Status:** Implemented & Fully Validated

---

## 1. Architectural Overview & Design Principles

The Repair Studio Recovery Pipeline provides deterministic, non-destructive, and evidence-based video salvage for damaged or corrupted media files.

### Core Principles
1. **Zero-Byte Source Mutation:**
   Source media files are treated as immutable forensic evidence. They are opened strictly with read-only permissions and are never modified, overwritten, truncated, or moved.
2. **Pure Stream Copy (No Unwanted Transcoding):**
   Recovery strategies prioritize stream copy (`-c copy`) without lossy re-encoding to preserve original compressed packets and avoid introducing generational compression artifacts.
3. **Isolated Application Staging:**
   All intermediate processing takes place in a dedicated, application-owned staging directory (`temp_dir/opentake_recovery_staging/<job_id>/`). No output is written to user directories until explicit finalization.
4. **Strict No-Clobber Output Placement:**
   Destination file paths chosen by the user are validated using atomic checks. If a file with the destination name already exists, the write is refused to prevent accidental data loss. Symlink destination paths are strictly rejected to defeat redirection attacks.
5. **Bounded Post-Recovery Validation:**
   FFmpeg process exit code `0` is never accepted as sufficient proof of recovery. Every generated media candidate undergoes a three-stage validation check: container recognition, FFprobe stream inventory, and bounded decode playback testing (`-f null -`).
6. **Honest Reporting (No Fabricated Success):**
   Media files missing crucial structural atoms (such as unfinalized MP4 recordings lacking a `moov` atom) cannot be salvaged with simple remuxing. The engine reports failure honestly, documents exact technical limitations, and recommends reference-based reconstruction (STEP 14).

---

## 2. Recovery Pipeline Domain Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Recovery Pipeline Flow                           │
└───────────────────────────────────┬─────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. Authorization & Source Verification                                      │
│    - Validate asset_id against MediaAuthorizationRegistry                   │
│    - Resolve canonical path and verify file size integrity                  │
│    - Ensure source is not inside staging directory                          │
└───────────────────────────────────┬─────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. Pre-Flight Inspection & Moov Check                                       │
│    - Detect container signature (ISO BMFF, MKV EBML, AVI RIFF)              │
│    - Scan for missing 'moov' atom in MP4/MOV:                               │
│      └─ If missing: immediately report STEP 14 required; abort safely       │
│    - Probe input streams and duration                                       │
└───────────────────────────────────┬─────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 3. Staged Strategy Execution (`media::process`)                             │
│    ├── A. Lossless Container Remux (Stream Copy, -movflags +faststart)      │
│    ├── B. Stream Salvage (Video-Only, Audio-Only, or All Readable)          │
│    └── C. Timestamp Rebuild (+genpts, avoid_negative_ts make_zero)          │
│    * Executed in isolated staging: opentake_recovery_staging/<job_id>/      │
│    * Isolated POSIX process group for clean cancellation cleanup            │
└───────────────────────────────────┬─────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 4. Three-Stage Output Validation (`repair::validation`)                     │
│    - Stage 1: File existence and non-zero size check (>0 bytes)             │
│    - Stage 2: Container signature recognition & FFprobe stream matching     │
│    - Stage 3: Bounded null-sink decode test (-t 8 -f null -)                │
│    - Classify: Completed (Passed) | Partial (Playable w/ warnings) | Failed │
└───────────────────────────────────┬─────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 5. Safe Finalization & Library Ingestion (`repair::recovery`)               │
│    - Explicit user destination confirmation (native file save dialog)       │
│    - Refuse destination overwrite (no-clobber) and reject symlinks          │
│    - Refuse source/destination identity collisions                          │
│    - Register new asset into MediaLibrarySession with NativeFileDialog prov │
│    - Cleanup staged directory                                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Supported Recovery Operations

| Strategy | When Offered | FFmpeg Implementation Details | Limitations & Caveats |
| :--- | :--- | :--- | :--- |
| **Lossless Remux** (`losslessRemux`) | Elementary streams readable; container headers damaged or trailing truncation observed | `-map 0:v? -map 0:a? -c copy` with `-movflags +faststart` for MP4/MOV | Cannot resolve broken bitstream packets or missing sample index tables (`moov`) |
| **Stream Salvage** (`streamSalvage`) | One or more streams severely damaged while other tracks remain readable | `-map 0:v:0 -c:v copy -an` (Video Only)<br>`-map 0:a:0 -c:a copy -vn` (Audio Only)<br>`-map 0:v? -map 0:a? -c copy` (All Readable) | Discards omitted or unreadable streams; user must review stream loss notification |
| **Timestamp Rebuild** (`timestampRebuild`) | Non-monotonic DTS jumps, out-of-order PTS, or negative start offsets | `-fflags +genpts -i ... -avoid_negative_ts make_zero -c copy` | Does not fix missing reference frames or arbitrary bitstream corruptions |

---

## 4. Security & Safety Boundaries

1. **Authorization Registry Enforcement:**
   Recovery IPC commands take an authorized `asset_id` and re-verify path provenance with `MediaAuthorizationRegistry`. Arbitrary file paths provided over IPC are rejected.
2. **Path Collision Defense:**
   The canonical source path and destination path are strictly compared. Any attempt to write output to the same path as the source file is immediately rejected with an error.
3. **Symlink Redirection Defense:**
   Target destinations that are symlinks are rejected before writing. The engine refuses to follow attacker-controlled symlinks.
4. **No-Clobber Finalization:**
   Output finalization uses atomic existence checks: if the destination exists, the operation halts and requests a distinct filename from the user.
5. **Process Group Isolation & Cancellation:**
   FFmpeg processes are spawned in isolated process groups (`cmd.process_group(0)` on Unix). When a user clicks "Cancel Recovery", the entire process group is reaped, and all temporary staging files are purged.

---

## 5. Output Validation Semantics

| Classification | Validation Criteria | User Interface Representation |
| :--- | :--- | :--- |
| **Completed** | File > 0 bytes; container recognized; expected streams matched; bounded decode test passed with zero errors. | Green badge: *"Valid & Playable Stream"* |
| **Partial** | File > 0 bytes; playable media generated; but non-fatal decode warnings occurred or intentional stream loss occurred (e.g., salvage video-only). | Amber badge: *"Partial Recovery: Playable with Warnings"* |
| **Failed** | File is 0 bytes; FFprobe detected 0 streams; fatal decode failure; or missing required container atom (`moov`). | Red badge: *"Recovery Failed"* |
| **Cancelled** | Operation cancelled by user before or during decode validation; staged files deleted. | Neutral badge: *"Recovery Cancelled"* |

---

## 6. Relationship Between Operations

- **Standard Remux & Stream Salvage (STEP 13):**
  Recovers media using standard FFmpeg demuxing, stream copy, conservative stream isolation, and timestamp reconstruction on files with demuxable headers.
- **Reference-Based Reconstruction (STEP 14 Implemented):**
  Addresses unfinalized recordings (power failure, camera crash, interrupted stream) where the crucial `moov` atom was never written to disk. STEP 14 inspects raw sample data in `mdat`, parses codec parameters (SPS/PPS, timescale) from a healthy reference file recorded on the same device, and synthesizes a compatible `moov` index and valid sample tables (`stbl`). See [docs/architecture/MP4_RECONSTRUCTION_ENGINE.md](MP4_RECONSTRUCTION_ENGINE.md) for full specification.
- **Unified Review, Validation & Reporting (STEP 15 Implemented):**
  Synthesizes output from all recovery methods into a cohesive `RecoveryReview`. Supports secure candidate preview under `SourceProvenance::StagedRecoveryCandidate`, multi-mode before/after playback, on-demand full decode validation, persistent recovery attempt history, and standalone offline HTML/JSON report export. See [docs/architecture/REPAIR_REVIEW_AND_REPORTING.md](REPAIR_REVIEW_AND_REPORTING.md).

