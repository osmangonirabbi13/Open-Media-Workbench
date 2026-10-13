# OpenTake — Future AI Integration Boundary (Post-Beta P1 Specification)

**Document Version:** 1.0.0  
**Target Release:** Deferred to Post-Beta (P1 — v0.2.0+)  
**Status:** Architectural Interface Boundary Specification  
**Author:** OpenTake AI/ML Systems Architect  
**Date:** 2026-10-13  

---

## 1. Strict Boundary Notice: AI is P1, NOT v0.1.0 Beta

> [!CRITICAL]
> **Scope Invariant:** OpenTake v0.1.0 Beta does **NOT** bundle, download, or execute neural network models. Core silence detection and timeline editing are 100% functional using deterministic digital signal processing (DSP) without any AI dependency.
> 
> This document specifies the **future architectural boundary** for Post-Beta (P1) local transcription integration with `whisper.cpp` to prevent architectural lock-in while preserving v0.1.0 focus.

---

## 2. Conceptual Future Transcription Service Interface

In post-beta v0.2.0, speech-to-text will be introduced behind a strictly isolated Rust trait in `crates/opentake-core/src/ai/`:

```rust
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start_us: i64,
    pub end_us: i64,
    pub confidence: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscriptSegment {
    pub id: Uuid,
    pub start_us: i64,
    pub end_us: i64,
    pub text: String,
    pub words: Vec<WordTimestamp>,
}

#[derive(Debug, Clone)]
pub struct ModelDescriptor {
    pub model_id: String, // e.g., "whisper-tiny.en", "whisper-base.en"
    pub file_size_bytes: u64,
    pub sha256_checksum: String,
    pub license_spdx: String, // "MIT"
    pub local_path: Option<std::path::PathBuf>,
}

#[async_trait]
pub trait TranscriptionEngine: Send + Sync {
    /// Checks whether the model weights exist locally in application cache.
    async fn is_model_available(&self, model_id: &str) -> bool;

    /// Downloads model weights only after explicit user consent in UI.
    async fn download_model(
        &self,
        model_id: &str,
        progress_tx: tokio::sync::mpsc::Sender<f32>,
        cancel_token: CancellationToken,
    ) -> Result<std::path::PathBuf, CoreError>;

    /// Executes offline local speech-to-text inference on decoded 16kHz PCM audio.
    async fn transcribe(
        &self,
        pcm_samples: &[f32],
        model_id: &str,
        progress_tx: tokio::sync::mpsc::Sender<f32>,
        cancel_token: CancellationToken,
    ) -> Result<Vec<TranscriptSegment>, CoreError>;
}
```

---

## 3. Evaluation of `whisper.cpp` as Local Engine

| Dimension | `whisper.cpp` Assessment | Post-Beta Implementation Strategy |
|---|---|---|
| **Licensing** | **MIT License.** | Fully compatible with OpenTake's open-source architecture. |
| **Model Weights** | OpenAI Whisper original weights are **MIT licensed**. Quantized GGML weights inherit MIT terms. | Downloaded directly from verified HuggingFace/GitHub mirror with SHA-256 validation. |
| **Hardware Acceleration** | Supports CPU AVX2, Apple Metal, Linux Vulkan, and NVIDIA CUDA. | Build with CPU fallback first; enable Metal/Vulkan backends in subsequent releases. |
| **User Privacy** | 100% Offline inference. Audio never leaves RAM/VRAM. | Upholds OpenTake's core privacy principle. |

---

## 4. User Consent & Model Download Safeguards

When P1 is implemented, OpenTake will enforce strict ethical download policies:
1. **Zero Silent Downloads:** The application will **never** automatically download model files in the background.
2. **Explicit Consent Modal:** A modal dialog will display:
   - Model name and quantized parameter size (e.g., `ggml-base.en.bin` — $142\text{ MB}$).
   - Destination path (`~/.cache/opentake/models/`).
   - MIT License acknowledgment.
   - Live download progress with an instant "Cancel" button.
