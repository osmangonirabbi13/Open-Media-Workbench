# Architecture Document: Repair Review, Preview, Validation & Reporting

## 1. Executive Summary

STEP 15 completes the recovery user journey in Open Media Workbench by delivering an evidence-backed, professional review and validation pipeline. Rather than stopping at file generation or making unsubstantiated claims of "100% restoration", the system provides:

1. **Secure Candidate Preview:** Isolated streaming of staged recovery candidates without exposing filesystem paths or granting unrestricted disk permissions.
2. **Multi-Mode Before/After Player:** Synchronized side-by-side comparison, single candidate inspection, and honest handling of unplayable damaged originals.
3. **Multi-Tier Output Validation:** Explicit differentiation between `MetadataOnly`, `SampledDecode`, and on-demand `FullDecode`.
4. **Timeline Difference Evidence:** Real measurement of nominal durations, stream start offsets, and A/V drift without fabricated percentages.
5. **Persistent Local Recovery History:** Crash-safe, atomic versioned JSON ledger tracking previous attempts without cloud telemetry.
6. **Privacy-Sanitized Reports:** Exportable machine-readable JSON and standalone, offline HTML reports protected by strict Content Security Policies (CSP) and automated path redaction.

---

## 2. Architecture & Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Repair Studio Review Flow                         │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                         Recovery Operation Finished
                     (Remux, Salvage, or Reconstruction)
                                      │
                                      ▼
               ┌──────────────────────────────────────────────┐
               │    Staged Candidate File in Isolated Temp    │
               └──────────────────────────────────────────────┘
                                      │
                   register_recovery_candidate_preview()
                                      │
                                      ▼
               ┌──────────────────────────────────────────────┐
               │  Authorized Media Asset (Staged Provenance)  │
               │   - Tokenized in MediaLibrarySession         │
               │   - Zero raw filesystem path leakage         │
               │   - Streamable via opentake-media://         │
               └──────────────────────────────────────────────┘
                                      │
                     ┌────────────────┴────────────────┐
                     ▼                                 ▼
        ┌──────────────────────────┐     ┌──────────────────────────┐
        │   Before/After Preview   │     │   Validation Inspector   │
        │ - Side-by-Side (A/B)     │     │ - Container checks       │
        │ - Recovered Only         │     │ - Stream inventory       │
        │ - Damaged Source Only    │     │ - Sampled vs Full decode │
        │ - Linked/Independent seek│     │ - Timeline differences   │
        └──────────────────────────┘     └──────────────────────────┘
                     │                                 │
                     └────────────────┬────────────────┘
                                      │
                     ┌────────────────┴────────────────┐
                     ▼                                 ▼
        ┌──────────────────────────┐     ┌──────────────────────────┐
        │ Persistent Local History │     │ Report Export Engine     │
        │ - Up to 50 attempts      │     │ - Offline Standalone HTML│
        │ - Atomic JSON file write │     │ - Machine-readable JSON  │
        │ - Zero cloud telemetry   │     │ - Privacy Path Redaction │
        └──────────────────────────┘     └──────────────────────────┘
```

---

## 3. Recovery Review Domain Model

The core domain model is defined in `crates/opentake-core/src/repair/recovery_review.rs` and mirrored in `src/types/repair.ts`:

### 3.1 `RecoveryReview`
```rust
pub struct RecoveryReview {
    pub schema_version: u32,
    pub review_id: String,
    pub source_asset_id: String,
    pub recovery_job_id: String,
    pub recovery_method: String,
    pub recovery_status: RecoveryStatus,
    pub output_asset_id: Option<String>,
    pub output_candidate_id: Option<String>,
    pub source_summary: AssetMediaSummary,
    pub output_summary: Option<AssetMediaSummary>,
    pub validation_summary: ValidationSummary,
    pub timeline_evidence: Option<TimelineEvidence>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
    pub created_at_epoch_ms: u64,
}
```

### 3.2 Core Distinctions
The engine strictly differentiates three distinct claims:
1. **Completed Processing Operation:** The FFmpeg or reconstruction process exited cleanly with code 0.
2. **Playable Recovered Output:** The resulting file has valid container headers and playable streams.
3. **Full Content Restoration:** The recovered media matches original recorded content without missing frames or omitted tracks. Playability does **not** imply full restoration.

---

## 4. Preview & Candidate Authorization

### 4.1 Security Boundaries
- **No Path Leakage:** Staging directory paths (`/tmp/.../opentake_recovery_staging/...`) are never sent across the IPC boundary to the webview.
- **Opaque Protocol Identifiers:** The candidate file is registered with `SourceProvenance::StagedRecoveryCandidate` into `MediaLibrarySession`. The frontend only receives an opaque UUID asset ID.
- **Strict Custom Protocol Handler:** The custom protocol `opentake-media://` streams media directly by resolving the authorized asset ID inside Rust, maintaining Tauri sandboxing and rejecting path traversal attacks.

### 4.2 Candidate vs Finalized Media
- **Staged Recovery Candidate:** Temporary file created in an application scratch folder during recovery. Subject to cleanup when the review session terminates.
- **Finalized Exported Media:** Saved to an authorized user directory only upon explicit user export confirmation via native file dialog.
- **Imported Original Source:** Strictly read-only input media file.

---

## 5. Before / After Comparison Player

The desktop UI provides three preview modes:
1. **Side-by-Side (`Original | Recovered`):** Both players rendered in parallel. Supports linked play/pause, linked seeking, and independent seeking when timebases diverge.
2. **Recovered Candidate Only:** Focus on inspecting audio/video quality of the salvaged output.
3. **Original Source Only:** Inspect damaged source footage and playback anomalies.

### Unplayable Original Fallback
When a source file is severely damaged (e.g. missing `moov` index, corrupted container header, or demuxer failure):
- The original video player gracefully transitions to an honest fallback state.
- Damaged file metadata and diagnostic findings remain clearly displayed.
- The player does not attempt or pretend to perform synchronized playback against an unplayable container.

---

## 6. Multi-Tier Output Validation

Validation is categorized into three explicit depth levels:

| Level | Scope | Execution Trigger | Evidence Provided |
| :--- | :--- | :--- | :--- |
| **`MetadataOnly`** | Fast container structure & stream inventory check | Automatic on recovery completion | File existence, non-zero byte size, stream inventory, codec profiles |
| **`SampledDecode`** | Bounded decode scan of initial & representative frames (`-f null -`) | Automatic post-processing default | First 8–30s decode verification, PTS continuity, macroblock error check |
| **`FullDecode`** | Complete bitstream decode of entire output file from 0:00 to EOF | User-initiated on-demand action | 100% frame decode proof; surfaces any truncated or corrupt GOPs |

### Disclosed Coverage Invariant
Sampled decoding never claims that an entire 60-minute video is healthy based on 8 seconds of clean frames. The review summary explicitly discloses:
- Analyzed duration window
- Uninspected remaining regions
- Exact finding codes (`DECODE_SAMPLED_PASSED`, `DECODE_FULL_PASSED`)

---

## 7. Timeline Difference & Integrity Analysis

Duration and timestamp comparisons adhere to strict evidentiary guidelines:
- **Missing MOOV Durations:** If the damaged original lacks a header duration, the engine reports source duration as `Unknown (missing container header)` rather than inventing a duration or computing a fake recovery percentage from file sizes.
- **Stream Start Offsets:** Evaluates audio vs video start timestamps (`videoStartOffsetSeconds`, `audioStartOffsetSeconds`).
- **Intentional Omissions:** When the user selects Video-Only stream salvage, the absence of an audio track is reported as `AUDIO_INTENTIONALLY_OMITTED` (Passed) rather than an error or corruption failure.

---

## 8. Persistent Recovery History

Persisted locally at `<AppLocalData>/recovery_history.json`:
- **Crash Safety:** Writes to a `.tmp` file and executes an atomic filesystem rename.
- **Bounded Storage:** Automatically capped at 50 most recent recovery attempts.
- **Resilience:** Malformed or obsolete JSON structures fall back gracefully to a clean ledger without crashing.
- **Privacy:** Contains only metadata and sanitized filenames. No authorization tokens, plaintext passwords, or raw file paths are persisted.
- **Manual Clear:** The user can clear all history records at any time.

---

## 9. Report Generation & Privacy Sanitization

### 9.1 Machine-Readable JSON
- Serialized from canonical `RecoveryReview` schema version 1.
- Fully structured finding codes, stream inventories, and limitation strings.

### 9.2 Standalone Offline HTML Report
- **Zero External Dependencies:** Built with inline CSS styled to match the dark workbench aesthetic. Requires no external fonts, scripts, CDNs, or internet connectivity.
- **Content Security Policy:** Embedded with `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">`.
- **XSS Prevention:** All dynamic strings, filenames, codec names, and error messages are filtered through `html_escape()`, escaping `&`, `<`, `>`, `"`, and `'`.
- **Cross-Platform Privacy Sanitization:** File paths across Linux (`/home/user`), macOS (`/Users/user`), and Windows (`C:\Users\user`) are redacted to `~` prior to export. Staging scratch paths are masked to `[Staged Output File]`.
