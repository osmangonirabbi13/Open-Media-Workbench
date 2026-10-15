//! Text-to-Speech Engine trait and runtime adapters.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::audio::validate_wav_file;
use super::errors::TtsError;
use super::models::{TtsRuntimeCapabilities, TtsSynthesisInput, WavAudioMetadata};

/// Contract for offline TTS runtime implementations.
pub trait TtsEngine: Send + Sync {
    /// Discovers system capabilities, binary path, and version.
    fn discover_capabilities(&self) -> Result<TtsRuntimeCapabilities, TtsError>;

    /// Verifies if the runtime is installed, executable, and functioning.
    fn verify_runtime(&self) -> Result<bool, TtsError>;

    /// Synthesizes speech from structured input text to a target WAV file.
    fn synthesize(
        &self,
        input: &TtsSynthesisInput,
        output_path: &Path,
        cancel_rx: Option<&tokio::sync::watch::Receiver<bool>>,
    ) -> Result<WavAudioMetadata, TtsError>;
}

/// Production TTS adapter invoking the local Piper CLI binary.
pub struct SystemPiperCliEngine {
    custom_binary_path: Option<PathBuf>,
}

impl SystemPiperCliEngine {
    pub fn new(custom_binary_path: Option<PathBuf>) -> Self {
        Self { custom_binary_path }
    }

    /// Resolves the piper executable path.
    pub fn resolve_binary_path(&self) -> Option<PathBuf> {
        if let Some(ref path) = self.custom_binary_path {
            if path.is_file() {
                return Some(path.clone());
            }
        }

        let mut candidates = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            let h = PathBuf::from(home);
            candidates.push(h.join(".local/share/opentake/bin/piper"));
            candidates.push(h.join(".local/bin/piper"));
            candidates.push(h.join(".local/share/open-media-workbench/bin/piper"));
        }
        candidates.push(PathBuf::from("/usr/local/bin/piper"));
        candidates.push(PathBuf::from("/usr/bin/piper"));

        candidates.into_iter().find(|candidate| candidate.is_file())
    }
}

impl TtsEngine for SystemPiperCliEngine {
    fn discover_capabilities(&self) -> Result<TtsRuntimeCapabilities, TtsError> {
        let binary_path = self.resolve_binary_path();
        let discovered = binary_path.is_some();

        let mut version = None;
        let mut error = None;

        if let Some(ref path) = binary_path {
            match Command::new(path).arg("--version").output() {
                Ok(output) if output.status.success() => {
                    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    version = Some(if ver.is_empty() {
                        "Piper (compatible)".to_string()
                    } else {
                        ver
                    });
                }
                Ok(output) => {
                    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    error = Some(format!("Piper returned error: {err}"));
                }
                Err(e) => {
                    error = Some(format!("Failed to execute Piper: {e}"));
                }
            }
        } else {
            error =
                Some("Piper executable not found in ~/.local/share/opentake/bin or PATH".into());
        }

        Ok(TtsRuntimeCapabilities {
            discovered,
            binary_path: binary_path.map(|p| p.to_string_lossy().to_string()),
            version,
            supported_formats: vec!["wav".into()],
            error,
        })
    }

    fn verify_runtime(&self) -> Result<bool, TtsError> {
        let caps = self.discover_capabilities()?;
        Ok(caps.discovered && caps.error.is_none())
    }

    fn synthesize(
        &self,
        input: &TtsSynthesisInput,
        output_path: &Path,
        cancel_rx: Option<&tokio::sync::watch::Receiver<bool>>,
    ) -> Result<WavAudioMetadata, TtsError> {
        if let Some(rx) = cancel_rx {
            if *rx.borrow() {
                return Err(TtsError::Cancellation);
            }
        }

        let binary_path = self.resolve_binary_path().ok_or_else(|| {
            TtsError::MissingRuntime("Piper executable is not available on this system".into())
        })?;

        if !input.model_path.exists() {
            return Err(TtsError::MissingModel(format!(
                "Model file not found: {}",
                input.model_path.display()
            )));
        }

        if !input.config_path.exists() {
            return Err(TtsError::MissingArtifact(format!(
                "Model config file not found: {}",
                input.config_path.display()
            )));
        }

        // Calculate length scale: length_scale = 1.0 / speed_scale
        // (In Piper: length_scale > 1.0 means slower speech, length_scale < 1.0 means faster speech)
        let length_scale = 1.0 / input.settings.speed_scale.max(0.25);

        let mut cmd = Command::new(&binary_path);
        cmd.arg("--model")
            .arg(&input.model_path)
            .arg("--config")
            .arg(&input.config_path)
            .arg("--output_file")
            .arg(output_path)
            .arg("--length_scale")
            .arg(format!("{:.3}", length_scale))
            .arg("--noise_scale")
            .arg(format!("{:.3}", input.settings.noise_scale))
            .arg("--noise_w")
            .arg(format!("{:.3}", input.settings.noise_w))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(speaker_id) = input.settings.speaker_id {
            cmd.arg("--speaker").arg(speaker_id.to_string());
        }

        // Set LD_LIBRARY_PATH if running from ~/.local/share/opentake/bin
        if let Some(parent) = binary_path.parent() {
            let current_ld = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
            let new_ld = if current_ld.is_empty() {
                parent.to_string_lossy().to_string()
            } else {
                format!("{}:{current_ld}", parent.display())
            };
            cmd.env("LD_LIBRARY_PATH", new_ld);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| TtsError::ProcessFailed(format!("Failed to spawn Piper process: {e}")))?;

        // Feed text via stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.text.as_bytes())
                .map_err(|e| TtsError::ProcessFailed(format!("Failed to write to stdin: {e}")))?;
            let _ = stdin.write_all(b"\n");
        }

        // Wait for child with cancellation check
        loop {
            if let Some(rx) = cancel_rx {
                if *rx.borrow() {
                    let _ = child.kill();
                    let _ = child.wait();
                    if output_path.exists() {
                        let _ = std::fs::remove_file(output_path);
                    }
                    return Err(TtsError::Cancellation);
                }
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        let stderr_output = child
                            .wait_with_output()
                            .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
                            .unwrap_or_else(|_| "Unknown process error".into());
                        return Err(TtsError::ProcessFailed(format!(
                            "Piper exited with non-zero status ({status}): {stderr_output}"
                        )));
                    }
                    break;
                }
                Ok(None) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(TtsError::ProcessFailed(format!(
                        "Error waiting on Piper child process: {e}"
                    )));
                }
            }
        }

        validate_wav_file(output_path)
    }
}

/// Deterministic mock TTS engine for unit and integration testing.
pub struct MockTtsEngine {
    sample_rate: u32,
}

impl Default for MockTtsEngine {
    fn default() -> Self {
        Self { sample_rate: 22050 }
    }
}

impl MockTtsEngine {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// Generates synthetic PCM audio formatted as a valid WAV file.
    pub fn generate_synthetic_wav(
        &self,
        text: &str,
        output_path: &Path,
    ) -> Result<WavAudioMetadata, TtsError> {
        let char_count = text.chars().count().max(1);
        // Duration: ~60ms per char, min 400ms, max 5s
        let duration_ms = ((char_count * 60) as u64).clamp(400, 5000);
        let sample_count = (duration_ms * self.sample_rate as u64) / 1000;

        let mut file = File::create(output_path)?;

        let num_channels = 1u16;
        let bits_per_sample = 16u16;
        let bytes_per_sample = 2u32;
        let byte_rate = self.sample_rate * (num_channels as u32) * bytes_per_sample;
        let block_align = num_channels * (bytes_per_sample as u16);
        let data_size = (sample_count * (bytes_per_sample as u64)) as u32;
        let riff_size = data_size + 36;

        // Write WAV header
        file.write_all(b"RIFF")?;
        file.write_all(&riff_size.to_le_bytes())?;
        file.write_all(b"WAVE")?;
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?; // PCM
        file.write_all(&num_channels.to_le_bytes())?;
        file.write_all(&self.sample_rate.to_le_bytes())?;
        file.write_all(&byte_rate.to_le_bytes())?;
        file.write_all(&block_align.to_le_bytes())?;
        file.write_all(&bits_per_sample.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&data_size.to_le_bytes())?;

        // Write audible synthetic sine waveform (440 Hz tone)
        let frequency = 440.0;
        for i in 0..sample_count {
            let t = (i as f32) / (self.sample_rate as f32);
            let sample = (t * frequency * 2.0 * std::f32::consts::PI).sin();
            let sample_i16 = (sample * 16000.0) as i16;
            file.write_all(&sample_i16.to_le_bytes())?;
        }
        file.flush()?;

        validate_wav_file(output_path)
    }
}

impl TtsEngine for MockTtsEngine {
    fn discover_capabilities(&self) -> Result<TtsRuntimeCapabilities, TtsError> {
        Ok(TtsRuntimeCapabilities {
            discovered: true,
            binary_path: Some("/mock/bin/piper".into()),
            version: Some("Mock Piper 1.0.0".into()),
            supported_formats: vec!["wav".into()],
            error: None,
        })
    }

    fn verify_runtime(&self) -> Result<bool, TtsError> {
        Ok(true)
    }

    fn synthesize(
        &self,
        input: &TtsSynthesisInput,
        output_path: &Path,
        cancel_rx: Option<&tokio::sync::watch::Receiver<bool>>,
    ) -> Result<WavAudioMetadata, TtsError> {
        if let Some(rx) = cancel_rx {
            if *rx.borrow() {
                return Err(TtsError::Cancellation);
            }
        }

        self.generate_synthetic_wav(&input.text, output_path)
    }
}
