use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::media::discovery::{discover_tool, DiscoveryConfig, ToolType};
use crate::media::error::MediaEngineError;
use crate::media::process::{run_managed_process, ProcessSpec};

use super::recovery_report::SalvageStreamType;
use super::remux::RecoveryExecutionOutput;

/// Default timeout for salvage operations (3 minutes).
pub const SALVAGE_OPERATION_TIMEOUT: Duration = Duration::from_secs(180);

/// Maximum stderr bytes to capture during salvage execution (512 KiB).
pub const MAX_SALVAGE_STDERR_BYTES: usize = 512 * 1024;

pub type SalvageExecutionResult = RecoveryExecutionOutput;

/// Executes conservative stream salvage to isolate intact video or audio tracks
/// from a damaged or partially corrupted container using stream copy (`-c copy`).
pub async fn execute_stream_salvage(
    source_path: &Path,
    output_path: &Path,
    stream_type: SalvageStreamType,
    container_extension: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<RecoveryExecutionOutput, MediaEngineError> {
    let ffmpeg = discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default()).map_err(|e| {
        MediaEngineError::BinaryNotFound {
            tool: "ffmpeg".to_string(),
            searched_paths: vec![e.to_string()],
        }
    })?;

    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "-v".to_string(),
        "warning".to_string(),
        "-i".to_string(),
        source_path.to_string_lossy().to_string(),
    ];

    match stream_type {
        SalvageStreamType::VideoOnly => {
            args.push("-map".to_string());
            args.push("0:v:0".to_string());
            args.push("-c:v".to_string());
            args.push("copy".to_string());
            args.push("-an".to_string()); // Disable audio
        }
        SalvageStreamType::AudioOnly => {
            args.push("-map".to_string());
            args.push("0:a:0".to_string());
            args.push("-c:a".to_string());
            args.push("copy".to_string());
            args.push("-vn".to_string()); // Disable video
        }
        SalvageStreamType::AllReadable => {
            args.push("-map".to_string());
            args.push("0:v?".to_string());
            args.push("-map".to_string());
            args.push("0:a?".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
        }
    }

    // MP4/MOV faststart
    let ext_lower = container_extension.to_lowercase();
    if ext_lower == "mp4" || ext_lower == "mov" || ext_lower == "m4v" {
        args.push("-movflags".to_string());
        args.push("+faststart".to_string());
    }

    // Destination path
    args.push(output_path.to_string_lossy().to_string());

    let mut spec = ProcessSpec::new(ffmpeg.canonical_path, args, SALVAGE_OPERATION_TIMEOUT);
    spec.max_stderr_bytes = MAX_SALVAGE_STDERR_BYTES;

    let proc_out = run_managed_process(spec, cancellation).await?;

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if let Some(code) = proc_out.exit_code {
        if code != 0 {
            errors.push(format!("FFmpeg stream salvage exited with code {code}"));
        }
    }

    if !proc_out.stderr.is_empty() {
        for line in proc_out.stderr.lines() {
            let line_lower = line.to_lowercase();
            if line_lower.contains("error")
                || line_lower.contains("invalid")
                || line_lower.contains("failed")
            {
                errors.push(line.trim().to_string());
            } else if line_lower.contains("warn") {
                warnings.push(line.trim().to_string());
            }
        }
    }

    let success = proc_out.exit_code == Some(0) && errors.is_empty();

    Ok(SalvageExecutionResult {
        success,
        exit_code: proc_out.exit_code,
        duration: proc_out.duration,
        warnings,
        errors,
    })
}
