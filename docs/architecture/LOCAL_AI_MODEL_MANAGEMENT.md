# Shared Local AI Model Management Architecture

**Document Version:** 1.0.0  
**Status:** Canonical Architecture Specification (STEP 18)  
**Date:** 2026-10-15  
**Workspaces:** Localization Studio & AI Shorts Studio  

---

## 1. Executive Summary & Design Principles

Open Media Workbench introduces a **shared, offline-first local AI Model Manager** serving both current and future workspaces:
- **Localization Studio:** Speech-to-Text (Whisper), neural machine translation, voice synthesis (TTS), and audio dubbing.
- **AI Shorts Studio:** Highlight detection, transcription, speech alignment, and social metadata generation.

To preserve the application's lightweight distribution footprint, privacy guarantees, and offline reliability, the model management architecture adheres to five inviolable principles:

1. **Small Installer / Zero Bundled Weights:** The application installer contains zero multi-gigabyte neural network weights. Base app installation and startup remain lightweight (<100MB target for Linux binaries).
2. **Explicit User Consent:** No model file is ever downloaded automatically on startup or in the background. Every download requires explicit user action, displaying the model purpose, artifact size, license, and source URL.
3. **Cryptographic SHA-256 Integrity Verification:** Downloads stream into isolated `.part` staging files while calculating running SHA-256 digests. Only artifacts that match verified pinned checksums are committed to the installed repository.
4. **Shared, Single-Instance Model Storage:** All workspaces reference a single local model directory (`app_data/ai-models`). Models are never duplicated between workspaces.
5. **Offline-First & Honest Runtime Contracts:** Base features (media repair, playback, subtitle editing) have zero dependencies on AI models. Installed model weights are distinguished from active inference engines: installed models are marked `Runtime Pending` until native execution runtimes (whisper.cpp, llama.cpp) are integrated in subsequent steps.

---

## 2. System Architecture & Component Hierarchy

```
                             Open Media Workbench
                                      │
                        Shared AI Model Manager
                     (crates/opentake-core/src/ai)
                                      │
         ┌────────────────────────────┼───────────────────────────┐
         │                            │                           │
   Model Catalog                Model Storage               Hardware Info
   (Curated & Pinned)      (~/.local/share/.../ai-models)    (Linux procfs & statvfs)
         │                            │                           │
         └────────────────────────────┼───────────────────────────┘
                                      │
                           Tauri IPC Security Bridge
                           (src-tauri/src/commands/ai.rs)
                                      │
                           Runtime Capability Layer
                           (Provider-neutral traits)
                                      │
         ┌────────────────────────────┴───────────────────────────┐
         │                                                        │
  Localization Studio                                      AI Shorts Studio
  (Speech-to-Text / Dubbing)                               (Highlight Detection / Metadata)
```

---

## 3. Curated & Verified Model Catalog

The initial catalog is pinned in Rust and mirrored in TypeScript. Every installable entry is verified against an immutable upstream Git LFS pointer:

| Model ID | Family | Capability | Format | Expected Size | License | SHA-256 Digest | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `whisper-tiny` | `whisper` | Speech-to-Text | GGML | 77,691,713 B (~74 MB) | MIT | `be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21` | Verified (Installable) |
| `whisper-base` | `whisper` | Speech-to-Text | GGML | 147,951,465 B (~141 MB) | MIT | `60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe` | Verified (Installable) |
| `piper-en-lessac-medium` | `piper` | Text-to-Speech | ONNX | 63,201,294 B (~60 MB) | Public Domain | `5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f` | Verified (Installable) |
| `qwen2.5-0.5b-instruct` | `qwen` | Text Generation | GGUF Q4_K_M | 491,400,032 B (~468 MB) | Apache-2.0 | `74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db` | Verified (Installable) |
| `opus-mt-en-es` | `opus-mt` | Translation | ONNX | 312,000,000 B (~298 MB) | CC-BY-4.0 | None (Pending) | Catalog-Only (Disabled) |

### 3.1 Security & Trusted Host Policy
- **HTTPS Enforcement:** Artifact downloads require valid TLS certificate verification and reject plain HTTP or insecure schemes.
- **Trusted Host Allowlist:** By default, downloads are restricted to `huggingface.co` and its subdomains (`cdn-lfs.huggingface.co`). Untrusted remote hosts or arbitrary user-supplied URLs are strictly rejected.
- **Single-File Direct Artifacts:** Model entries use direct single-file binaries (GGML, GGUF, ONNX) rather than arbitrary multi-file archives, eliminating Zip Slip, symlink traversal, and decompression bomb risks.

---

## 4. Secure Streaming Download Pipeline

```
[User Clicks Install] 
         │
[Confirmation Modal] ──(User Consents)──► [Check Available Disk Space]
                                                   │
                                          [Generate Staging File]
                                          (staging/{id}_{job}.part)
                                                   │
                                          [Stream Chunks over HTTPS]
                                          (Continuous Sha256 update)
                                                   │
                       ┌───────────────────────────┴──────────────────────────┐
                       │                                                      │
               [User Cancels]                                       [Stream Completes]
                       │                                                      │
               [Delete .part file]                                   [Verify Byte Count]
               [Reset State]                                         [Verify SHA-256]
                                                                              │
                                                            ┌─────────────────┴─────────────────┐
                                                            │                                   │
                                                      [Hash Mismatch]                     [Hash Matches]
                                                            │                                   │
                                                    [Delete .part file]                 [Atomic Rename to]
                                                    [Report Error]                      [installed/{id}/...]
                                                                                        [Update registry.json]
```

### 4.1 Bounded Memory & Size Overflow Protection
- Streaming writes write directly from network socket to disk in 64 KiB buffers without loading full model files into system RAM.
- A hard buffer threshold (`expected_size + 1 MB`) prevents unbounded disk usage if a rogue server sends indefinite chunked data.

### 4.2 Crash Recovery & Orphan Cleanup
- If the application is terminated mid-download, all incomplete `.part` files in the `staging/` directory are automatically deleted during startup reconciliation (`reconcile_startup`).
- The persistent ledger (`registry.json`) is updated atomically using a temporary file (`registry.json.tmp`) and atomic file replacement.

---

## 5. Storage Safety & Path Traversal Protections

### 5.1 Directory Layout
```
~/.local/share/open-media-workbench/ai-models/
├── installed/
│   ├── whisper-tiny/
│   │   └── ggml-tiny.bin
│   └── piper-en-lessac-medium/
│       └── en_US-lessac-medium.onnx
├── staging/
│   └── (temporary .part files during active downloads)
└── registry.json
```

### 5.2 Path Traversal & Symlink Defense
- Model IDs are strictly validated using `validate_model_id()`: must contain only ASCII alphanumerics, hyphens, and underscores. Path separators (`/`, `\`), `..`, null bytes, and hidden dot files are rejected.
- Target paths are canonicalized and verified to ensure they reside strictly within the managed `installed/` repository.
- Symlinks are detected via `symlink_metadata()` and rejected to defend against symlink redirection attacks.

### 5.3 In-Use Locking & Safe Removal
- An in-memory lock set (`active_locks`) tracks active inference and export jobs.
- Attempting to uninstall or delete a model that is currently in use returns `AiError::ModelInUse` and prevents file deletion.

---

## 6. Hardware & Environment Detection

Best-effort hardware discovery provides creators with guidance without executing intrusive or privileged shell scripts:

1. **System & CPU:** `std::env::consts::OS`, `ARCH`, and `available_parallelism()` reporting logical cores.
2. **RAM (Linux):** Reads `/proc/meminfo` line-by-line for `MemTotal` and `MemAvailable` without third-party dependencies.
3. **Storage Available:** Uses POSIX `libc::statvfs` on the storage path to measure unprivileged free disk space (`f_bavail * f_frsize`).
4. **GPU Acceleration Detection:** Inspects safe Linux device nodes (`/sys/class/drm/` and `/proc/driver/nvidia/version`).
   - Distinguishes **GPU Detected** vs **GPU Runtime Usable**. In STEP 18, `gpu_backend_installed: false` and `gpu_runtime_usable: false` are reported accurately because inference runtimes are not yet integrated.

---

## 7. Tauri IPC Bridge & Security Boundaries

Nine narrowly-scoped Tauri commands expose model management to the frontend without granting broad filesystem access:

| Command | Purpose | Input Validation |
| :--- | :--- | :--- |
| `list_ai_models` | Lists catalog with dynamic installation status | None (read-only) |
| `get_ai_model_status` | Status and installation details for single model | Model ID validated |
| `get_ai_hardware_info` | Host hardware and memory report | None (read-only) |
| `start_ai_model_download` | Starts background streaming download | Catalog verification & disk space check |
| `get_ai_model_download_status` | Returns active download progress | Model ID validated |
| `cancel_ai_model_download` | Cancels in-progress download | Model ID validated |
| `verify_ai_model` | Re-verifies SHA-256 on installed model | Model ID validated |
| `remove_ai_model` | Safely removes model files and updates registry | In-use lock check |
| `get_ai_storage_usage` | Reports exact byte footprints on disk | None (read-only) |

---

## 8. Summary of STEP 18 Invariants

- **Zero Bundled Model Weights:** Default desktop package contains zero neural network weight files.
- **No Automatic Downloads:** No background network requests on application startup.
- **Verified Metadata Only:** Install buttons are enabled strictly for verified, pinned models with known SHA-256 checksums.
- **Uncompromised Core Workspaces:** Repair Studio and Subtitle Editor remain 100% operational with or without downloaded models.

---

## 9. Consumer Integration: STEP 19 Offline Whisper Transcription

The shared Model Manager is actively consumed by the STEP 19 Whisper transcription engine:
- **Zero Redundant Storage:** `whisper-tiny` and `whisper-base` installed via the Model Manager are shared directly by Localization Studio.
- **Pre-Flight Capability Verification:** Multilingual vs English-only flags in catalog manifests are strictly enforced. English-only models (`.en`) reject non-English or Auto-Detect requests before launching inference.
- **In-Use Concurrency Locking:** `storage.acquire_lock(model_id)` is acquired at the start of transcription and released upon job completion, cancellation, or failure. Attempts to delete an active model while transcription runs are rejected with `AiError::ModelInUse`.
- **Integrity Validation:** Model files are verified on disk using stored SHA-256 digests prior to invoking the `whisper.cpp` inference runtime.

---

## 10. Consumer Integration: STEP 22 Offline Subtitle Translation Engine

The shared Model Manager powers the offline subtitle translation pipeline implemented in STEP 22:
- **Curated Translation Model:** Utilizes `qwen2.5-0.5b-instruct` (468 MB GGUF Q4_K_M, Apache-2.0, single-file direct download) as the primary CPU-efficient instruction translation engine.
- **Strict Verification:** Pre-flight SHA-256 check (`74a4da8c9f...`) against catalog metadata before launching inference.
- **Concurrency & Lock Safety:** Translation calls `storage.acquire_lock("qwen2.5-0.5b-instruct")` prior to invoking `llama.cpp` and guarantees release via RAII/deferred execution across all terminal paths (completion, error, cancellation).
- **Zero Redundant Storage:** Translation references the existing `~/.local/share/opentake/models` directory, avoiding duplicate weight files.

