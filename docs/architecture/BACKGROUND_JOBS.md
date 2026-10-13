# OpenTake — Background Job System and Concurrency Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 5 & STEP 20  
**Author:** OpenTake Core Architecture & Systems Lead  
**Date:** 2026-10-13  

---

## 1. Job Lifecycle and State Machine

All long-running tasks in OpenTake (probing, waveform generation, silence analysis, proxy creation, and video export) execute as managed asynchronous jobs governed by a strict state machine:

```
                  ┌──────────────┐
                  │    QUEUED    │
                  └──────┬───────┘
                         │ Worker thread acquires task slot
                         ▼
                  ┌──────────────┐
                  │   RUNNING    │
                  └──┬───┬────┬──┘
      User clicks    │   │    │ Process exits
      "Cancel"       │   │    │ successfully
         ┌───────────┘   │    └───────────────────────┐
         ▼               ▼ Error / Non-zero exit      ▼
  ┌──────────────┐ ┌──────────────┐            ┌──────────────┐
  │  CANCELLING  │ │    FAILED    │            │  COMPLETED   │
  └──────┬───────┘ └──────────────┘            └──────────────┘
         │ Child process killed & temp files unlinked
         ▼
  ┌──────────────┐
  │  CANCELLED   │
  └──────────────┘
```

### Allowed State Transitions:
- `Queued` $\rightarrow$ `Running` | `Cancelled`
- `Running` $\rightarrow$ `Completed` | `Failed` | `Cancelling`
- `Cancelling` $\rightarrow$ `Cancelled` | `Failed`
- Terminal States: `Completed`, `Failed`, `Cancelled` (immutable once entered).

---

## 2. Job Definition & Concurrency Management

In `crates/opentake-core/src/jobs/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    Probe,
    Waveform,
    SilenceAnalysis,
    ProxyGeneration,
    Export,
}

pub struct ActiveJob {
    pub id: Uuid,
    pub job_type: JobType,
    pub state: JobState,
    pub cancellation_token: CancellationToken,
    pub child_pid: Option<u32>,
    pub temp_artifacts: Vec<std::path::PathBuf>,
}
```

### 2.1 Concurrency Limits (Resource Protection)
To prevent thrashing the CPU, exhausting disk I/O, or overloading the GPU/memory:
- **Export Jobs:** Strictly bounded to **maximum 1 concurrent export** via an asynchronous semaphore (`tokio::sync::Semaphore::new(1)`).
- **Audio Analysis Jobs:** Bounded to **maximum 1 concurrent analysis** pass.
- **Waveform Jobs:** Maximum 1 concurrent generation per media source.
- **Probe Jobs:** Lightweight; up to 4 concurrent probes permitted.

---

## 3. Process Tree Termination & Cleanup Protocol

> [!WARNING]
> **Orphan Process Hazard:** Calling `child.kill()` on a process spawned by a shell or complex launcher only kills the parent PID, leaving grandchild FFmpeg encoders running indefinitely in the background and locking files.

### 3.1 Process Tree Termination Strategy:
1. **Process Group Isolation (Unix / Linux / macOS):**
   When spawning child processes via `tokio::process::Command`, OpenTake sets the process group ID (`process_group(0)` on Unix).
   Upon cancellation:
   - Signals the entire process group with `SIGINT` (graceful stop) via `libc::killpg(pgid, libc::SIGINT)`.
   - Waits up to $1{,}500\text{ ms}$ for child exit.
   - If still running after timeout, issues a hard `SIGKILL` (`libc::killpg(pgid, libc::SIGKILL)`).
2. **Windows Process Tree Termination:**
   On Windows, child processes are assigned to a Win32 Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, or terminated using `taskkill.exe /F /T /PID <pid>`.

### 3.2 Temporary Artifact Cleanup Invariant
Every `ActiveJob` maintains a registered list of `temp_artifacts: Vec<PathBuf>`. Upon entering `Cancelled` or `Failed`, the job manager automatically unlinks and removes every registered temporary file.

---

## 4. Frontend Unmount and Shutdown Handling

1. **Window Close / App Exit:** When Tauri receives a close-requested event, the background job manager fires cancellation tokens for all active jobs, terminates child process trees, and flushes pending temp cleanup before permitting window destruction.
2. **Frontend Component Unmount:** The background job continues running in the Rust Tokio runtime. When the user navigates back to that view, the frontend queries `cmd_get_job_status(job_id)` to re-attach progress listeners.
