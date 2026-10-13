# OpenTake — Tauri 2 IPC and Security Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 3  
**Author:** OpenTake Core Architecture & Desktop Lead  
**Date:** 2026-10-13  

---

## 1. IPC Architecture Overview

Communication between the React/TypeScript frontend and the native Rust backend follows strict **typed remote procedure calls (RPC)** for state mutations and **throttled asynchronous events** for background operational updates.

```
┌────────────────────────────────────────────────────────────────────────┐
│                   FRONTEND (React 19 / TypeScript)                     │
│                                                                        │
│   invoke("command_name", payload)  ─────────┐ (Typed Request)          │
│                                             │                          │
│   listen("event_name", callback)   ◄────────┼─────────┐ (Async Event)  │
└─────────────────────────────────────────────┼─────────┼────────────────┘
                                              │         │
                       TAURI 2 IPC BOUNDARY   │         │
                                              ▼         │
┌───────────────────────────────────────────────────────┼────────────────┐
│                   BACKEND (src-tauri Commands)        │                │
│                                                       │                │
│   1. Validate payload & permissions                   │                │
│   2. Delegate to `opentake-core` domain methods       │                │
│   3. Mutate authoritative AppState                    │                │
│   4. Emit throttled events (max 10 Hz) ───────────────┘                │
│   5. Return typed Response / Result<T, CoreError>                      │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Typed IPC Command Specifications

### 2.1 Media Ingestion & Probing

#### `cmd_probe_media`
- **Purpose:** Ingests a local file path, validates accessibility, runs FFprobe, and returns structured container/stream metadata.
- **Input Payload:**
  ```typescript
  interface ProbeMediaRequest {
    filePath: string; // Absolute canonical path to media file
  }
  ```
- **Output Payload:**
  ```typescript
  interface ProbeMediaResponse {
    mediaId: string; // UUID v4 assigned to this media reference
    filePath: string;
    containerFormat: string;
    durationUs: number; // Integer microseconds (i64)
    fileSizeBytes: number;
    videoStream?: {
      codec: string;
      width: number;
      height: number;
      fpsNumerator: number;
      fpsDenominator: number;
      isVfr: boolean;
      pixelFormat: string;
    };
    audioStream?: {
      codec: string;
      sampleRateHz: number;
      channels: number;
    };
    previewSupportedDirectly: boolean; // False if WebView cannot decode natively
  }
  ```
- **Validation Rules:** File must exist, be readable, and have size $> 0$. Traversal elements (`..`) are rejected.
- **Error Behavior:** Returns `CoreError::ProbeFailed` or `CoreError::UnsupportedFormat`.
- **Security:** Path must be canonicalized and checked against allowed directories or user file-picker selections.

---

### 2.2 Timeline & Manual Editing

#### `cmd_split_segment`
- **Purpose:** Splits the active segment at a specified timeline timecode into two distinct segments.
- **Input Payload:**
  ```typescript
  interface SplitSegmentRequest {
    splitTimeUs: number; // Integer microsecond coordinate (i64)
  }
  ```
- **Output Payload:**
  ```typescript
  interface TimelineStateResponse {
    projectId: string;
    totalDurationUs: number;
    retainedRanges: Array<{
      id: string; // UUID v4
      sourceStartUs: number;
      sourceEndUs: number;
    }>;
    canUndo: boolean;
    canRedo: boolean;
  }
  ```
- **Authoritative State:** Rust owns the canonical `EditDecisionList`. The frontend updates its UI view from this response.

#### `cmd_delete_segment`
- **Purpose:** Excises a segment from the retained ranges list.
- **Input:** `{ rangeId: string }`
- **Output:** `TimelineStateResponse`
- **Validation:** Cannot delete if only one frame would remain without explicit user confirmation.

#### `cmd_trim_range`
- **Purpose:** Adjusts start or end boundary of an existing retained range.
- **Input:** `{ rangeId: string, newStartUs?: number, newEndUs?: number }`
- **Output:** `TimelineStateResponse`
- **Validation:** Invariant enforced: `sourceStartUs < sourceEndUs`. Range cannot extend beyond raw source duration.

#### `cmd_undo` / `cmd_redo`
- **Purpose:** Steps backward or forward in the command-pattern history stack.
- **Input:** `{}`
- **Output:** `TimelineStateResponse`

---

### 2.3 Audio Waveform & Silence Analysis

#### `cmd_start_waveform_generation`
- **Purpose:** Launches background audio decoding and multi-resolution LOD peak extraction.
- **Input:** `{ mediaId: string, targetBucketsPerSecond?: number }`
- **Output:** `{ jobId: string }`
- **Background Behavior:** Emits `job_progress` events; finishes with `job_completed` containing waveform cache key.

#### `cmd_start_silence_analysis`
- **Purpose:** Spawns a background streaming DSP task to detect silent intervals.
- **Input Payload:**
  ```typescript
  interface SilenceAnalysisRequest {
    mediaId: string;
    thresholdDbfs: number; // e.g., -35.0 (clamped between -60.0 and -20.0)
    minDurationMs: number; // e.g., 500 (clamped between 200 and 2000)
    prePaddingMs: number;  // e.g., 150 (clamped between 50 and 500)
    postPaddingMs: number; // e.g., 150 (clamped between 50 and 500)
  }
  ```
- **Output:** `{ jobId: string }`
- **Authoritative State:** Does NOT mutate the EDL automatically. Produces candidate suggestions for user review.

#### `cmd_get_silence_suggestions`
- **Purpose:** Retrieves the candidate cut regions produced by a completed analysis job.
- **Input:** `{ jobId: string }`
- **Output Payload:**
  ```typescript
  interface SilenceSuggestionsResponse {
    candidates: Array<{
      id: string; // UUID v4
      startUs: number;
      endUs: number;
      durationUs: number;
      averageEnergyDbfs: number;
    }>;
    estimatedSavedTimeUs: number;
  }
  ```

#### `cmd_apply_silence_suggestions`
- **Purpose:** Converts confirmed candidate cuts into EDL exclusions.
- **Input:** `{ acceptedCandidateIds: string[] }`
- **Output:** `TimelineStateResponse`

---

### 2.4 Project Persistence

#### `cmd_save_project`
- **Purpose:** Serializes the project to a versioned `.opentake` JSON file using atomic write-and-rename.
- **Input:** `{ filePath: string }`
- **Output:** `{ savedAt: string, fileSizeBytes: number }`
- **Validation:** File path must be writeable. Atomic `.tmp` file swapped upon sync.

#### `cmd_open_project`
- **Purpose:** Loads a project file, validates schema version, and checks source media availability.
- **Input:** `{ filePath: string }`
- **Output Payload:**
  ```typescript
  interface OpenProjectResponse {
    project: ProjectData;
    sourceMediaStatus: "available" | "missing";
    missingSourcePath?: string;
  }
  ```

---

### 2.5 Export Engine & Background Job Management

#### `cmd_start_export`
- **Purpose:** Compiles the canonical timeline into an FFmpeg filtergraph and begins rendering.
- **Input Payload:**
  ```typescript
  interface StartExportRequest {
    destinationPath: string;
    preset: "1080p" | "720p" | "match_source";
    quality: "high" | "balanced" | "fast_draft";
  }
  ```
- **Output:** `{ jobId: string }`
- **Pre-flight Checks:** Disk space check, destination file overwrite confirmation, source presence check.

#### `cmd_cancel_job`
- **Purpose:** Immediately aborts an active background task (analysis or export).
- **Input:** `{ jobId: string }`
- **Output:** `{ cancelled: boolean }`
- **Behavior:** Fires `CancellationToken`, terminates child process tree, unlinks partial files.

---

## 3. Asynchronous Event Specification

All asynchronous background notifications use typed Tauri events. To prevent IPC event bus saturation, **progress events are throttled in Rust to a maximum rate of 10 Hz (100 ms interval)**.

```typescript
// Event: "job:progress"
interface JobProgressEvent {
  jobId: string;
  jobType: "waveform" | "silence_analysis" | "export" | "proxy";
  percentage: number; // 0.0 to 100.0
  elapsedSeconds: number;
  estimatedRemainingSeconds?: number;
  currentStage: string;
}

// Event: "job:completed"
interface JobCompletedEvent {
  jobId: string;
  jobType: string;
  durationMs: number;
  resultPayload?: any;
}

// Event: "job:failed"
interface JobFailedEvent {
  jobId: string;
  jobType: string;
  errorMessage: string;
}

// Event: "job:cancelled"
interface JobCancelledEvent {
  jobId: string;
  jobType: string;
}
```

### Race Condition & Stale Event Protections
1. **Job ID Correlation:** The frontend ignores any progress or completion events whose `jobId` does not match the currently active job for that subsystem.
2. **Post-Cancellation Suppression:** Once `cmd_cancel_job` is acknowledged by Rust, any lingering stdout progress lines buffered in the pipe are discarded; no late progress events are emitted.
3. **Mount / Remount Safety:** When React components unmount or remount, they issue `cmd_get_job_status` to reconcile state rather than relying solely on missed event streams.

---

## 4. Tauri 2 Capability & Permission Model

OpenTake enforces the **Principle of Least Privilege** using Tauri 2's granular capability configuration (`src-tauri/capabilities/default.json`).

### 4.1 Permitted Capabilities
- `core:default`: Basic window and app lifecycle APIs.
- `dialog:open` / `dialog:save`: System file picker dialogs for media selection and project saves.
- `fs:read` / `fs:write`: Restricted strictly to user-selected paths via file dialogs, project files (`*.opentake`), and application cache (`~/.cache/opentake/*`).

### 4.2 Explicitly Denied Capabilities
- **Unrestricted Shell Execution (`shell:all`):** **DENIED.** The frontend is strictly forbidden from executing arbitrary shell commands or spawning processes directly. All process spawning is encapsulated inside Rust behind audited typed commands.
- **Arbitrary Filesystem Access:** **DENIED.** Access to arbitrary root or system directories (`/etc`, `C:\Windows`) is blocked.
- **External Web Traffic:** **DENIED.** Webview Content Security Policy (CSP) blocks external network connections.

### 4.3 Content Security Policy (CSP) Blueprint
```text
default-src 'self';
img-src 'self' asset: data: blob:;
media-src 'self' asset: data: blob:;
style-src 'self' 'unsafe-inline';
script-src 'self';
connect-src 'self';
```
This guarantees that the webview cannot make outbound network calls, exfiltrate user data, or inject remote scripts.
