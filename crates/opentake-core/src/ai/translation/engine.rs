use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::media::process::{run_managed_process, ProcessSpec};

use super::errors::TranslationError;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub const DEFAULT_TRANSLATION_TIMEOUT: Duration = Duration::from_secs(120);
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Abstract contract for offline machine translation engines.
pub trait TranslationEngine: Send + Sync {
    fn is_available(&self) -> bool;
    fn runtime_path(&self) -> Option<PathBuf>;
    fn runtime_version(&self) -> Option<String>;

    fn translate<'a>(
        &'a self,
        model_path: &'a Path,
        texts: &'a [String],
        source_lang: &'a str,
        target_lang: &'a str,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, TranslationError>>;
}

/// Helper converting language code to natural language name.
pub fn language_code_to_name(code: &str) -> &'static str {
    match code.to_lowercase().as_str() {
        "en" => "English",
        "es" => "Spanish",
        "bn" => "Bengali",
        "fr" => "French",
        "de" => "German",
        "ja" => "Japanese",
        "zh" => "Chinese",
        "ar" => "Arabic",
        "ru" => "Russian",
        "pt" => "Portuguese",
        "hi" => "Hindi",
        _ => "English",
    }
}

/// Production implementation of TranslationEngine executing a local `llama-cli` process.
pub struct SystemLlamaCliTranslationEngine {
    binary_path: PathBuf,
    version: String,
}

impl SystemLlamaCliTranslationEngine {
    pub fn new(binary_path: PathBuf, version: String) -> Self {
        Self {
            binary_path,
            version,
        }
    }

    async fn translate_single_text(
        &self,
        model_path: &Path,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        cancel_token: &CancellationToken,
    ) -> Result<String, TranslationError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }

        let src_name = language_code_to_name(source_lang);
        let tgt_name = language_code_to_name(target_lang);

        let prompt = format!(
            "<|im_start|>system\nYou are a professional subtitle translator. Translate the given subtitle text from {src_name} to {tgt_name}. Output ONLY the direct translated text. Do NOT add explanations, notes, quotes, or markdown.\n<|im_end|>\n<|im_start|>user\n{trimmed}\n<|im_end|>\n<|im_start|>assistant\n"
        );

        let threads = std::thread::available_parallelism()
            .map(|p| p.get() as u32)
            .unwrap_or(4)
            .min(8);

        let args = vec![
            "-m".to_string(),
            model_path.to_string_lossy().to_string(),
            "-p".to_string(),
            prompt,
            "-n".to_string(),
            "256".to_string(),
            "--temp".to_string(),
            "0.1".to_string(),
            "-c".to_string(),
            "2048".to_string(),
            "--no-warmup".to_string(),
            "-t".to_string(),
            threads.to_string(),
            "--simple-io".to_string(),
            "--no-display-prompt".to_string(),
            "-st".to_string(),
        ];

        let spec = ProcessSpec::new(self.binary_path.clone(), args, DEFAULT_TRANSLATION_TIMEOUT);

        let output = run_managed_process(spec, Some(cancel_token))
            .await
            .map_err(|e| {
                TranslationError::InferenceFailed(format!("Failed to run llama-cli: {e}"))
            })?;

        if cancel_token.is_cancelled() {
            return Err(TranslationError::CancellationRequested);
        }

        let combined = output.stdout;
        // Parse assistant output
        let translated = if let Some(idx) = combined.rfind("<|im_start|>assistant") {
            let after = &combined[idx + "<|im_start|>assistant".len()..];
            let clean = after.trim_start();
            let mut end_pos = clean.len();
            for marker in &["[ Prompt:", "<|im_end|>", "\nExiting", "\n> "] {
                if let Some(pos) = clean.find(marker) {
                    end_pos = end_pos.min(pos);
                }
            }
            &clean[..end_pos]
        } else {
            &combined
        };

        let result = translated.trim().trim_matches('"').to_string();
        Ok(result)
    }
}

impl TranslationEngine for SystemLlamaCliTranslationEngine {
    fn is_available(&self) -> bool {
        self.binary_path.exists()
    }

    fn runtime_path(&self) -> Option<PathBuf> {
        Some(self.binary_path.clone())
    }

    fn runtime_version(&self) -> Option<String> {
        Some(self.version.clone())
    }

    fn translate<'a>(
        &'a self,
        model_path: &'a Path,
        texts: &'a [String],
        source_lang: &'a str,
        target_lang: &'a str,
        cancel_token: CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, TranslationError>> {
        Box::pin(async move {
            let mut results = Vec::with_capacity(texts.len());
            for text in texts {
                if cancel_token.is_cancelled() {
                    return Err(TranslationError::CancellationRequested);
                }
                let res = self
                    .translate_single_text(
                        model_path,
                        text,
                        source_lang,
                        target_lang,
                        &cancel_token,
                    )
                    .await?;
                results.push(res);
            }
            Ok(results)
        })
    }
}

/// Probes a single candidate binary to evaluate if it is a working llama-cli executable.
pub async fn probe_llama_binary(path: &Path) -> Result<String, String> {
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
        vec!["--version".to_string()],
        PROBE_TIMEOUT,
    );

    let cancel_token = CancellationToken::new();
    let output = run_managed_process(spec, Some(&cancel_token))
        .await
        .map_err(|e| format!("Failed to probe executable: {e}"))?;

    let combined = format!("{}\n{}", output.stdout, output.stderr);
    let lower = combined.to_lowercase();

    if !lower.contains("llama") && !lower.contains("version") && !lower.contains("build") {
        return Err("Binary does not match llama-cli interface.".to_string());
    }

    let first_line = combined.lines().next().unwrap_or("llama.cpp (local)");
    Ok(first_line.trim().to_string())
}

/// Discovers candidate llama-cli binaries on host system.
pub async fn discover_llama_runtime(
    custom_path: Option<&Path>,
) -> Result<SystemLlamaCliTranslationEngine, String> {
    if let Some(custom) = custom_path {
        let version = probe_llama_binary(custom).await?;
        return Ok(SystemLlamaCliTranslationEngine::new(
            custom.to_path_buf(),
            version,
        ));
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);
        candidates.push(home_path.join(".local/share/opentake/llama-bin/llama-cli"));
        candidates.push(home_path.join(".local/share/opentake/bin/llama-cli"));
        candidates.push(home_path.join(".local/share/open-media-workbench/bin/llama-cli"));
        candidates.push(home_path.join(".local/bin/llama-cli"));
    }

    candidates.push(PathBuf::from("/usr/local/bin/llama-cli"));
    candidates.push(PathBuf::from("/usr/bin/llama-cli"));

    for candidate in candidates {
        if candidate.exists() {
            if let Ok(version) = probe_llama_binary(&candidate).await {
                return Ok(SystemLlamaCliTranslationEngine::new(candidate, version));
            }
        }
    }

    Err("No compatible llama-cli binary discovered on system.".to_string())
}

/// Mock engine for isolated unit and contract tests.
pub struct MockTranslationEngine {
    pub is_available: bool,
    pub responses: Option<Vec<String>>,
}

impl MockTranslationEngine {
    pub fn new(is_available: bool) -> Self {
        Self {
            is_available,
            responses: None,
        }
    }

    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            is_available: true,
            responses: Some(responses),
        }
    }
}

impl TranslationEngine for MockTranslationEngine {
    fn is_available(&self) -> bool {
        self.is_available
    }

    fn runtime_path(&self) -> Option<PathBuf> {
        if self.is_available {
            Some(PathBuf::from("/mock/bin/llama-cli"))
        } else {
            None
        }
    }

    fn runtime_version(&self) -> Option<String> {
        if self.is_available {
            Some("llama.cpp vMock".to_string())
        } else {
            None
        }
    }

    fn translate<'a>(
        &'a self,
        _model_path: &'a Path,
        texts: &'a [String],
        _source_lang: &'a str,
        target_lang: &'a str,
        _cancel_token: CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, TranslationError>> {
        Box::pin(async move {
            if !self.is_available {
                return Err(TranslationError::RuntimeUnavailable(
                    "Mock translation runtime is marked unavailable.".to_string(),
                ));
            }

            if let Some(ref resps) = self.responses {
                return Ok(resps.clone());
            }

            let mut out = Vec::with_capacity(texts.len());
            for t in texts {
                out.push(format!("[{target_lang}] {t}"));
            }
            Ok(out)
        })
    }
}
