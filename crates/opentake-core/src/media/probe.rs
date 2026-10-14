use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::discovery::{discover_tool, DiscoveryConfig, ToolType};
use super::error::MediaEngineError;
use super::metadata::{normalize_ffprobe_json, MediaMetadata};
use super::process::{run_managed_process, ProcessSpec};

/// Default timeout for FFprobe inspection (10 seconds).
pub const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Bounded output memory limit for FFprobe JSON output (2 MiB).
pub const PROBE_MAX_STDOUT_BYTES: usize = 2 * 1024 * 1024;

/// Bounded output memory limit for FFprobe stderr output (256 KiB).
pub const PROBE_MAX_STDERR_BYTES: usize = 256 * 1024;

/// Probes a local media file using FFprobe and returns normalized metadata.
///
/// # Security & Safety
/// - Accepts only an existing canonicalized `target_file` path.
/// - Passes `--` delimiter prior to the file path to prevent FFprobe option injection.
/// - Enforces maximum timeout (10s) and bounded memory buffers.
/// - Supports immediate cancellation via `CancellationToken`.
pub async fn probe_media_file(
    ffprobe_path: Option<&PathBuf>,
    target_file: &Path,
    file_size_bytes: u64,
    cancellation: Option<&CancellationToken>,
) -> Result<MediaMetadata, MediaEngineError> {
    // 1. Resolve ffprobe executable
    let executable = match ffprobe_path {
        Some(p) => p.clone(),
        None => {
            let discovered = discover_tool(ToolType::Ffprobe, &DiscoveryConfig::default())?;
            discovered.canonical_path
        }
    };

    // 2. Build argument vector with option-injection defense
    let path_str = target_file.to_string_lossy().to_string();
    let args = vec![
        "-v".to_string(),
        "error".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        "--".to_string(), // End of options flag: prevents filenames like "-v" or "--help" being parsed as flags
        path_str.clone(),
    ];

    let mut spec = ProcessSpec::new(executable, args, DEFAULT_PROBE_TIMEOUT);
    spec.max_stdout_bytes = PROBE_MAX_STDOUT_BYTES;
    spec.max_stderr_bytes = PROBE_MAX_STDERR_BYTES;

    // 3. Execute managed process
    let output = run_managed_process(spec, cancellation).await?;

    // 4. Validate process exit code
    if let Some(code) = output.exit_code {
        if code != 0 {
            let stderr_clean = output.stderr.trim();
            return Err(MediaEngineError::CorruptMedia {
                path: path_str,
                details: if stderr_clean.is_empty() {
                    format!("ffprobe exited with status code {code}")
                } else {
                    stderr_clean.to_string()
                },
            });
        }
    }

    // 5. Parse and normalize output
    normalize_ffprobe_json(&output.stdout, file_size_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_probe_missing_binary() {
        let fake_path = PathBuf::from("/nonexistent/bin/ffprobe");
        let target = PathBuf::from("/tmp/video.mp4");

        let res = probe_media_file(Some(&fake_path), &target, 1024, None).await;
        assert!(res.is_err());
    }
}
