use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use crate::media::process::{run_managed_process, ProcessSpec};

use super::errors::TranscriptionError;
use super::models::{get_supported_whisper_languages, WhisperRuntimeCapabilities};
use super::segments::{parse_timestamp_str_to_micros, RawTranscriptionSegment};

/// Boxed future alias for trait object dyn-compatibility without external macro dependencies.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Default execution timeout for Whisper inference on a single audio staging file (30 minutes).
pub const DEFAULT_WHISPER_INFERENCE_TIMEOUT: Duration = Duration::from_secs(1800);

/// Probe timeout when verifying a candidate whisper runtime binary (5 seconds).
pub const RUNTIME_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Raw inference output containing segments and optional detected language.
#[derive(Debug, Clone)]
pub struct RawInferenceOutput {
    pub segments: Vec<RawTranscriptionSegment>,
    pub detected_language: Option<String>,
}

/// Abstract contract for Whisper speech-to-text inference backends.
pub trait WhisperEngine: Send + Sync {
    /// Returns whether the engine backend is installed and operational.
    fn is_available(&self) -> bool;

    /// Returns the verified executable path to the runtime binary if applicable.
    fn runtime_path(&self) -> Option<PathBuf>;

    /// Returns the detected runtime version string.
    fn runtime_version(&self) -> Option<String>;

    /// Executes speech-to-text inference on the given audio file using the provided model.
    fn transcribe<'a>(
        &'a self,
        model_path: &'a Path,
        audio_path: &'a Path,
        language: &'a str,
        threads: Option<u32>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'a, Result<RawInferenceOutput, TranscriptionError>>;
}

/// Upstream whisper.cpp JSON output schema mapping.
#[derive(Debug, Deserialize)]
struct WhisperCliResultJson {
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhisperCliTimestampJson {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhisperCliOffsetJson {
    pub from: Option<i64>,
    pub to: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WhisperCliSegmentJson {
    pub timestamps: Option<WhisperCliTimestampJson>,
    pub offsets: Option<WhisperCliOffsetJson>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhisperCliOutputJson {
    pub result: Option<WhisperCliResultJson>,
    pub transcription: Option<Vec<WhisperCliSegmentJson>>,
}

/// Parses structured whisper.cpp JSON output into raw segments.
pub fn parse_whisper_json_output(
    json_str: &str,
    default_lang: &str,
) -> Result<RawInferenceOutput, TranscriptionError> {
    let parsed: WhisperCliOutputJson = serde_json::from_str(json_str).map_err(|e| {
        TranscriptionError::OutputParsingFailed(format!(
            "Failed to deserialize whisper.cpp JSON output: {e}"
        ))
    })?;

    let detected_language = parsed.result.and_then(|r| r.language).or_else(|| {
        if default_lang != "auto" {
            Some(default_lang.to_string())
        } else {
            None
        }
    });

    let mut raw_segments = Vec::new();

    if let Some(transcription) = parsed.transcription {
        for seg in transcription {
            let text = seg.text.unwrap_or_default();
            if text.trim().is_empty() {
                continue;
            }

            let mut start_us = 0i64;
            let mut end_us = 0i64;

            if let Some(offsets) = seg.offsets {
                if let Some(from_ms) = offsets.from {
                    start_us = from_ms * 1_000;
                }
                if let Some(to_ms) = offsets.to {
                    end_us = to_ms * 1_000;
                }
            } else if let Some(ts) = seg.timestamps {
                if let Some(from_str) = ts.from {
                    start_us = parse_timestamp_str_to_micros(&from_str).unwrap_or(0);
                }
                if let Some(to_str) = ts.to {
                    end_us = parse_timestamp_str_to_micros(&to_str).unwrap_or(start_us + 100_000);
                }
            }

            raw_segments.push(RawTranscriptionSegment {
                start_us,
                end_us,
                text,
                detected_language: detected_language.clone(),
                confidence: None,
            });
        }
    }

    Ok(RawInferenceOutput {
        segments: raw_segments,
        detected_language,
    })
}

/// Production implementation executing a local `whisper-cli` or `whisper.cpp` binary.
pub struct SystemWhisperCliEngine {
    binary_path: PathBuf,
    version: String,
}

impl SystemWhisperCliEngine {
    pub fn new(binary_path: PathBuf, version: String) -> Self {
        Self {
            binary_path,
            version,
        }
    }
}

impl SystemWhisperCliEngine {
    async fn transcribe_internal(
        &self,
        model_path: &Path,
        audio_path: &Path,
        language: &str,
        threads: Option<u32>,
        cancel_token: CancellationToken,
    ) -> Result<RawInferenceOutput, TranscriptionError> {
        if !self.binary_path.exists() {
            return Err(TranscriptionError::RuntimeUnavailable(format!(
                "Whisper binary '{}' does not exist.",
                self.binary_path.display()
            )));
        }

        if !model_path.exists() {
            return Err(TranscriptionError::ModelNotInstalled {
                model_id: model_path.to_string_lossy().to_string(),
                details: "Model artifact file missing from disk.".to_string(),
            });
        }

        if !audio_path.exists() {
            return Err(TranscriptionError::AudioExtractionFailed(format!(
                "Staged audio file '{}' does not exist.",
                audio_path.display()
            )));
        }

        // Generate temporary output file prefix in audio file's parent dir
        let temp_dir = audio_path.parent().unwrap_or_else(|| Path::new("/tmp"));
        let job_uuid = uuid::Uuid::new_v4().to_string();
        let out_prefix = temp_dir.join(format!("whisper_out_{job_uuid}"));
        let json_out_path = temp_dir.join(format!("whisper_out_{job_uuid}.json"));

        let num_threads = threads.unwrap_or_else(|| {
            let cores = std::thread::available_parallelism()
                .map(|p| p.get() as u32)
                .unwrap_or(2);
            // Limit to at most 4 threads to prevent locking UI responsiveness
            cores.clamp(1, 4)
        });

        let args = vec![
            "-m".to_string(),
            model_path.to_string_lossy().to_string(),
            "-f".to_string(),
            audio_path.to_string_lossy().to_string(),
            "-l".to_string(),
            language.to_string(),
            "-t".to_string(),
            num_threads.to_string(),
            "-oj".to_string(),
            "-of".to_string(),
            out_prefix.to_string_lossy().to_string(),
            "--no-prints".to_string(),
        ];

        let spec = ProcessSpec::new(
            self.binary_path.clone(),
            args,
            DEFAULT_WHISPER_INFERENCE_TIMEOUT,
        );

        let output = run_managed_process(spec, Some(&cancel_token))
            .await
            .map_err(|e| {
                let _ = std::fs::remove_file(&json_out_path);
                TranscriptionError::ProcessFailed {
                    exit_code: None,
                    details: format!("Whisper inference execution failed: {e}"),
                }
            })?;

        if let Some(exit_code) = output.exit_code {
            if exit_code != 0 {
                let _ = std::fs::remove_file(&json_out_path);
                return Err(TranscriptionError::ProcessFailed {
                    exit_code: Some(exit_code),
                    details: format!(
                        "Whisper runtime exited with error: {}",
                        output.stderr.lines().last().unwrap_or("Unknown error")
                    ),
                });
            }
        }

        // Read and parse the generated JSON output
        if !json_out_path.exists() {
            return Err(TranscriptionError::OutputParsingFailed(
                "Whisper inference completed but JSON output file was not found on disk."
                    .to_string(),
            ));
        }

        let json_content = tokio::fs::read_to_string(&json_out_path)
            .await
            .map_err(|e| {
                let _ = std::fs::remove_file(&json_out_path);
                TranscriptionError::OutputParsingFailed(format!(
                    "Failed to read whisper JSON output: {e}"
                ))
            })?;

        // Cleanup the temporary JSON output file immediately
        let _ = std::fs::remove_file(&json_out_path);

        parse_whisper_json_output(&json_content, language)
    }
}

impl WhisperEngine for SystemWhisperCliEngine {
    fn is_available(&self) -> bool {
        self.binary_path.exists()
    }

    fn runtime_path(&self) -> Option<PathBuf> {
        Some(self.binary_path.clone())
    }

    fn runtime_version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn transcribe<'a>(
        &'a self,
        model_path: &'a Path,
        audio_path: &'a Path,
        language: &'a str,
        threads: Option<u32>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'a, Result<RawInferenceOutput, TranscriptionError>> {
        Box::pin(async move {
            self.transcribe_internal(model_path, audio_path, language, threads, cancel_token)
                .await
        })
    }
}

/// Probes a single candidate binary to determine if it is a genuine, compatible whisper.cpp executable.
pub async fn probe_candidate_binary(path: &Path) -> Result<String, String> {
    if !path.exists() || !path.is_file() {
        return Err("Binary does not exist or is not a regular file.".to_string());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = path.metadata() {
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err("File is not executable.".to_string());
            }
        }
    }

    let spec = ProcessSpec::new(
        path.to_path_buf(),
        vec!["-h".to_string()],
        RUNTIME_PROBE_TIMEOUT,
    );

    let cancel_token = CancellationToken::new();
    let output = run_managed_process(spec, Some(&cancel_token))
        .await
        .map_err(|e| format!("Failed to probe executable: {e}"))?;

    let combined = format!("{}\n{}", output.stdout, output.stderr);
    let lower = combined.to_lowercase();

    // Verify presence of characteristic whisper.cpp flags
    let has_model_flag = lower.contains("-m,") || lower.contains("--model");
    let has_file_flag = lower.contains("-f,") || lower.contains("--file");
    let _has_lang_flag = lower.contains("-l,") || lower.contains("--language");

    if !has_model_flag || !has_file_flag {
        return Err(
            "Executable does not match whisper.cpp command-line interface (-m, -f).".to_string(),
        );
    }

    // Try to extract version from output
    let mut version = "whisper.cpp (local)".to_string();
    for line in combined.lines() {
        if line.contains("whisper") || line.contains("version") {
            let trimmed = line.trim();
            if trimmed.len() < 50 {
                version = trimmed.to_string();
                break;
            }
        }
    }

    Ok(version)
}

/// Discovers and evaluates available whisper.cpp runtimes on the host.
pub async fn discover_whisper_runtime(custom_path: Option<&Path>) -> WhisperRuntimeCapabilities {
    let mut errors = Vec::new();
    let supported_languages = get_supported_whisper_languages();

    // 1. If user provided a custom path, evaluate it first
    if let Some(custom) = custom_path {
        match probe_candidate_binary(custom).await {
            Ok(ver) => {
                return WhisperRuntimeCapabilities {
                    is_available: true,
                    runtime_path: Some(custom.to_string_lossy().to_string()),
                    runtime_version: Some(ver),
                    is_custom: true,
                    supported_languages,
                    errors: vec![],
                };
            }
            Err(e) => {
                errors.push(format!(
                    "Custom runtime at '{}' is invalid: {e}",
                    custom.display()
                ));
            }
        }
    }

    // 2. Candidate well-known local paths
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        candidates.push(home_path.join(".local/share/open-media-workbench/bin/whisper-cli"));
        candidates.push(home_path.join(".local/bin/whisper-cli"));
        candidates.push(home_path.join(".local/bin/whisper.cpp"));
        candidates.push(home_path.join(".local/bin/whisper"));
    }

    candidates.push(PathBuf::from("/usr/local/bin/whisper-cli"));
    candidates.push(PathBuf::from("/usr/bin/whisper-cli"));
    candidates.push(PathBuf::from("/usr/local/bin/whisper-cpp"));
    candidates.push(PathBuf::from("/usr/bin/whisper-cpp"));
    candidates.push(PathBuf::from("/usr/local/bin/whisper"));
    candidates.push(PathBuf::from("/usr/bin/whisper"));

    for candidate in candidates {
        if candidate.exists() {
            if let Ok(ver) = probe_candidate_binary(&candidate).await {
                return WhisperRuntimeCapabilities {
                    is_available: true,
                    runtime_path: Some(candidate.to_string_lossy().to_string()),
                    runtime_version: Some(ver),
                    is_custom: false,
                    supported_languages,
                    errors: vec![],
                };
            }
        }
    }

    WhisperRuntimeCapabilities {
        is_available: false,
        runtime_path: None,
        runtime_version: None,
        is_custom: false,
        supported_languages,
        errors,
    }
}

/// Deterministic mock Whisper engine for automated testing and CI.
#[derive(Clone)]
pub struct MockWhisperEngine {
    available: bool,
    canned_segments: Vec<RawTranscriptionSegment>,
    detected_language: Option<String>,
}

impl MockWhisperEngine {
    pub fn new(available: bool) -> Self {
        Self {
            available,
            canned_segments: Vec::new(),
            detected_language: Some("en".to_string()),
        }
    }

    pub fn with_segments(mut self, segments: Vec<RawTranscriptionSegment>) -> Self {
        self.canned_segments = segments;
        self
    }

    pub fn with_language(mut self, lang: Option<String>) -> Self {
        self.detected_language = lang;
        self
    }
}

impl MockWhisperEngine {
    async fn transcribe_internal(
        &self,
        _model_path: &Path,
        _audio_path: &Path,
        language: &str,
        _threads: Option<u32>,
        cancel_token: CancellationToken,
    ) -> Result<RawInferenceOutput, TranscriptionError> {
        if !self.available {
            return Err(TranscriptionError::RuntimeUnavailable(
                "Mock whisper engine is set to unavailable.".to_string(),
            ));
        }

        if cancel_token.is_cancelled() {
            return Err(TranscriptionError::JobCancelled);
        }

        // Return canned segments if provided
        if !self.canned_segments.is_empty() {
            return Ok(RawInferenceOutput {
                segments: self.canned_segments.clone(),
                detected_language: self.detected_language.clone(),
            });
        }

        // Default realistic speech segments (including Bengali Unicode check)
        let segments = if language == "bn" {
            vec![
                RawTranscriptionSegment {
                    start_us: 0,
                    end_us: 2_500_000,
                    text: "ওপেন মিডিয়া ওয়ার্কবেঞ্চে স্বাগতম।".to_string(),
                    detected_language: Some("bn".to_string()),
                    confidence: Some(0.96),
                },
                RawTranscriptionSegment {
                    start_us: 2_600_000,
                    end_us: 5_000_000,
                    text: "এটি একটি নির্ভরযোগ্য অফলাইন ট্রান্সক্রিপশন ইঞ্জিন।".to_string(),
                    detected_language: Some("bn".to_string()),
                    confidence: Some(0.94),
                },
            ]
        } else {
            vec![
                RawTranscriptionSegment {
                    start_us: 0,
                    end_us: 2_500_000,
                    text: "Welcome to Open Media Workbench.".to_string(),
                    detected_language: Some("en".to_string()),
                    confidence: Some(0.98),
                },
                RawTranscriptionSegment {
                    start_us: 2_600_000,
                    end_us: 5_200_000,
                    text: "This is a reliable offline speech-to-text transcription engine."
                        .to_string(),
                    detected_language: Some("en".to_string()),
                    confidence: Some(0.95),
                },
            ]
        };

        Ok(RawInferenceOutput {
            segments,
            detected_language: if language == "auto" {
                Some("en".to_string())
            } else {
                Some(language.to_string())
            },
        })
    }
}

impl WhisperEngine for MockWhisperEngine {
    fn is_available(&self) -> bool {
        self.available
    }

    fn runtime_path(&self) -> Option<PathBuf> {
        if self.available {
            Some(PathBuf::from("/mock/bin/whisper-cli"))
        } else {
            None
        }
    }

    fn runtime_version(&self) -> Option<String> {
        if self.available {
            Some("whisper.cpp (mock v1.7.0)".to_string())
        } else {
            None
        }
    }

    fn transcribe<'a>(
        &'a self,
        model_path: &'a Path,
        audio_path: &'a Path,
        language: &'a str,
        threads: Option<u32>,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'a, Result<RawInferenceOutput, TranscriptionError>> {
        Box::pin(async move {
            self.transcribe_internal(model_path, audio_path, language, threads, cancel_token)
                .await
        })
    }
}
