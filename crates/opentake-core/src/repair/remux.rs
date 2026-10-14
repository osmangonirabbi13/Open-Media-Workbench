use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::media::discovery::{discover_tool, DiscoveryConfig, ToolType};
use crate::media::error::MediaEngineError;
use crate::media::process::{run_managed_process, ProcessSpec};

/// Default timeout for remux operations (3 minutes).
pub const REMUX_OPERATION_TIMEOUT: Duration = Duration::from_secs(180);

/// Maximum stderr bytes to capture during remux execution (512 KiB).
pub const MAX_REMUX_STDERR_BYTES: usize = 512 * 1024;

/// Result of executing a recovery FFmpeg process.
#[derive(Debug, Clone)]
pub struct RecoveryExecutionOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub type RemuxExecutionResult = RecoveryExecutionOutput;

/// Executes lossless container remux by copying video and audio streams into a freshly muxed container.
///
/// Stream copy (`-c copy`) is strictly used without re-encoding.
/// When `timestamp_rebuild` is true, conservative timestamp regeneration flags (`+genpts`, `avoid_negative_ts`)
/// are applied to fix non-monotonic or negative DTS/PTS issues.
pub async fn execute_lossless_remux(
    source_path: &Path,
    output_path: &Path,
    timestamp_rebuild: bool,
    container_extension: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<RecoveryExecutionOutput, MediaEngineError> {
    let ffmpeg = discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default()).map_err(|e| {
        MediaEngineError::BinaryNotFound {
            tool: "ffmpeg".to_string(),
            searched_paths: vec![e.to_string()],
        }
    })?;

    let mut args: Vec<String> = vec!["-y".to_string(), "-v".to_string(), "warning".to_string()];

    // Timestamp handling flags if requested
    if timestamp_rebuild {
        args.push("-fflags".to_string());
        args.push("+genpts".to_string());
    }

    // Input file specification
    args.push("-i".to_string());
    args.push(source_path.to_string_lossy().to_string());

    // Conservative stream selection: Map all video and audio tracks, ignoring corrupt data/subtitle tracks
    args.push("-map".to_string());
    args.push("0:v?".to_string());
    args.push("-map".to_string());
    args.push("0:a?".to_string());

    // Pure stream copy - no re-encoding!
    args.push("-c".to_string());
    args.push("copy".to_string());

    if timestamp_rebuild {
        args.push("-avoid_negative_ts".to_string());
        args.push("make_zero".to_string());
    }

    // MP4/MOV faststart to ensure moov atom is at beginning of file
    let ext_lower = container_extension.to_lowercase();
    if ext_lower == "mp4" || ext_lower == "mov" || ext_lower == "m4v" {
        args.push("-movflags".to_string());
        args.push("+faststart".to_string());
    }

    // Destination path
    args.push(output_path.to_string_lossy().to_string());

    let mut spec = ProcessSpec::new(ffmpeg.canonical_path, args, REMUX_OPERATION_TIMEOUT);
    spec.max_stderr_bytes = MAX_REMUX_STDERR_BYTES;

    let proc_out = run_managed_process(spec, cancellation).await?;

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    if let Some(code) = proc_out.exit_code {
        if code != 0 {
            errors.push(format!(
                "FFmpeg remux process exited with error code {code}"
            ));
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
            } else if line_lower.contains("warn") || line_lower.contains("non-monotonic") {
                warnings.push(line.trim().to_string());
            }
        }
    }

    let success = proc_out.exit_code == Some(0) && errors.is_empty();

    Ok(RemuxExecutionResult {
        success,
        exit_code: proc_out.exit_code,
        duration: proc_out.duration,
        warnings,
        errors,
    })
}
