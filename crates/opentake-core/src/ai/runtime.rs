//! Provider-neutral Runtime Capability Contracts for future AI inference engines.

use super::models::{AiHardwareInfo, AiModelCapability, AiModelManifest};

/// Result of evaluating model compatibility against host hardware and runtime.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelCompatibility {
    pub is_compatible: bool,
    pub runtime_available: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Abstract contract for AI inference engine providers.
pub trait AiRuntime: Send + Sync {
    /// Human-readable identifier of the runtime backend (e.g. "whisper.cpp", "llama.cpp").
    fn backend_name(&self) -> &str;

    /// Supported model file format (e.g. "ggml", "gguf", "onnx").
    fn supported_format(&self) -> &str;

    /// Primary model capability serviced by this runtime.
    fn primary_capability(&self) -> AiModelCapability;

    /// Checks if the runtime engine is actually installed and operational.
    fn is_available(&self) -> bool;

    /// Evaluates if the given model manifest can execute on the target hardware.
    fn check_compatibility(
        &self,
        manifest: &AiModelManifest,
        hardware: &AiHardwareInfo,
    ) -> AiModelCompatibility;
}

/// Baseline mock/foundation runtime provider representing the STEP 18 offline state.
/// All inference engines are currently planned for subsequent implementation steps.
pub struct UnavailableRuntime {
    backend: String,
    format: String,
    capability: AiModelCapability,
}

impl UnavailableRuntime {
    pub fn new(backend: &str, format: &str, capability: AiModelCapability) -> Self {
        Self {
            backend: backend.to_string(),
            format: format.to_string(),
            capability,
        }
    }
}

impl AiRuntime for UnavailableRuntime {
    fn backend_name(&self) -> &str {
        &self.backend
    }

    fn supported_format(&self) -> &str {
        &self.format
    }

    fn primary_capability(&self) -> AiModelCapability {
        self.capability
    }

    fn is_available(&self) -> bool {
        // In STEP 18, zero inference engines are compiled or bundled.
        false
    }

    fn check_compatibility(
        &self,
        manifest: &AiModelManifest,
        hardware: &AiHardwareInfo,
    ) -> AiModelCompatibility {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if !self.is_available() {
            errors.push(format!(
                "Inference runtime '{}' is planned for future integration and is not yet available.",
                self.backend
            ));
        }

        if let Some(total_ram) = hardware.total_ram_bytes {
            if total_ram < manifest.min_ram_bytes {
                warnings.push(format!(
                    "System RAM ({} MB) is below recommended minimum ({} MB).",
                    total_ram / (1024 * 1024),
                    manifest.min_ram_bytes / (1024 * 1024)
                ));
            }
        }

        AiModelCompatibility {
            is_compatible: errors.is_empty(),
            runtime_available: self.is_available(),
            warnings,
            errors,
        }
    }
}

/// Evaluates default compatibility for a model manifest against system hardware.
pub fn evaluate_model_compatibility(
    manifest: &AiModelManifest,
    hardware: &AiHardwareInfo,
) -> AiModelCompatibility {
    let runtime = UnavailableRuntime::new(
        &manifest.runtime_backend,
        &manifest.runtime_format,
        manifest.capability,
    );
    runtime.check_compatibility(manifest, hardware)
}
