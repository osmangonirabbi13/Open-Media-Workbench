use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;

use super::error::MediaEngineError;

/// Default maximum memory limit for diagnostic standard output buffer (512 KiB).
pub const DEFAULT_MAX_STDOUT_BYTES: usize = 512 * 1024;

/// Default maximum memory limit for diagnostic standard error buffer (512 KiB).
pub const DEFAULT_MAX_STDERR_BYTES: usize = 512 * 1024;

/// Specification for launching a managed child process.
#[derive(Debug, Clone)]
pub struct ProcessSpec {
    /// Approved, validated path to the executable binary.
    pub program: PathBuf,
    /// Explicit argument vector (passed directly without shell expansion).
    pub args: Vec<String>,
    /// Maximum allowed execution duration.
    pub timeout: Duration,
    /// Maximum bytes to retain in stdout buffer.
    pub max_stdout_bytes: usize,
    /// Maximum bytes to retain in stderr buffer.
    pub max_stderr_bytes: usize,
    /// Optional isolated working directory.
    pub working_dir: Option<PathBuf>,
}

impl ProcessSpec {
    pub fn new(program: PathBuf, args: Vec<String>, timeout: Duration) -> Self {
        Self {
            program,
            args,
            timeout,
            max_stdout_bytes: DEFAULT_MAX_STDOUT_BYTES,
            max_stderr_bytes: DEFAULT_MAX_STDERR_BYTES,
            working_dir: None,
        }
    }
}

/// Output captured from a managed child process execution.
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub is_stdout_truncated: bool,
    pub is_stderr_truncated: bool,
    pub duration: Duration,
}

/// Asynchronously executes a child process with timeout, cancellation, bounded output,
/// and process group isolation.
pub async fn run_managed_process(
    spec: ProcessSpec,
    cancellation: Option<&CancellationToken>,
) -> Result<ProcessOutput, MediaEngineError> {
    let start_time = Instant::now();
    let command_repr = format!("{} {}", spec.program.display(), spec.args.join(" "));

    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    // Isolate child process group on Unix so child and descendants can be killed cleanly
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    if let Some(ref cwd) = spec.working_dir {
        cmd.current_dir(cwd);
    }

    // Standardize locale to C for consistent parsing
    cmd.env("LC_ALL", "C");

    let mut child = cmd.spawn().map_err(|e| MediaEngineError::SpawnFailed {
        command: command_repr.clone(),
        details: e.to_string(),
    })?;

    let pid = child.id();

    let mut stdout_pipe = child.stdout.take().ok_or_else(|| {
        MediaEngineError::Internal("Failed to capture child stdout pipe".to_string())
    })?;

    let mut stderr_pipe = child.stderr.take().ok_or_else(|| {
        MediaEngineError::Internal("Failed to capture child stderr pipe".to_string())
    })?;

    // Read stdout and stderr concurrently to prevent pipe deadlocks
    let max_stdout = spec.max_stdout_bytes;
    let max_stderr = spec.max_stderr_bytes;

    let read_stdout_task =
        tokio::spawn(async move { read_bounded_stream(&mut stdout_pipe, max_stdout).await });

    let read_stderr_task =
        tokio::spawn(async move { read_bounded_stream(&mut stderr_pipe, max_stderr).await });

    // Await process completion with timeout and cancellation
    let execution_result = tokio::select! {
        // 1. Process exit
        status_res = child.wait() => {
            match status_res {
                Ok(status) => Ok(status.code()),
                Err(e) => Err(MediaEngineError::Internal(format!("Failed to wait for child: {e}"))),
            }
        }
        // 2. Cancellation token triggered
        _ = async {
            match cancellation {
                Some(token) => token.cancelled().await,
                None => std::future::pending::<()>().await,
            }
        } => {
            terminate_and_reap(&mut child, pid).await;
            Err(MediaEngineError::Cancelled { command: command_repr.clone() })
        }
        // 3. Timeout expired
        _ = tokio::time::sleep(spec.timeout) => {
            terminate_and_reap(&mut child, pid).await;
            Err(MediaEngineError::Timeout {
                command: command_repr.clone(),
                duration_ms: spec.timeout.as_millis() as u64,
            })
        }
    };

    let exit_code = execution_result?;

    // Collect stream results
    let (stdout_bytes, is_stdout_truncated) = read_stdout_task
        .await
        .map_err(|e| MediaEngineError::Internal(format!("Stdout reader panicked: {e}")))?;

    let (stderr_bytes, is_stderr_truncated) = read_stderr_task
        .await
        .map_err(|e| MediaEngineError::Internal(format!("Stderr reader panicked: {e}")))?;

    let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr_str = String::from_utf8_lossy(&stderr_bytes).to_string();

    Ok(ProcessOutput {
        exit_code,
        stdout: stdout_str,
        stderr: stderr_str,
        is_stdout_truncated,
        is_stderr_truncated,
        duration: start_time.elapsed(),
    })
}

/// Reads a stream asynchronously up to `max_bytes`, discarding subsequent bytes to prevent memory overflow.
async fn read_bounded_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> (Vec<u8>, bool) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut is_truncated = false;

    while let Ok(n) = reader.read(&mut chunk).await {
        if n == 0 {
            break;
        }

        if buffer.len() + n <= max_bytes {
            buffer.extend_from_slice(&chunk[..n]);
        } else {
            let remaining = max_bytes.saturating_sub(buffer.len());
            if remaining > 0 {
                buffer.extend_from_slice(&chunk[..remaining]);
            }
            is_truncated = true;
            // Drain remaining bytes from the pipe to prevent child process stalling on a full buffer
            let mut drain_buf = [0u8; 8192];
            while let Ok(drained) = reader.read(&mut drain_buf).await {
                if drained == 0 {
                    break;
                }
            }
            break;
        }
    }

    (buffer, is_truncated)
}

/// Escalates process termination from SIGTERM to SIGKILL, then reaps the child process.
pub async fn terminate_and_reap(child: &mut Child, pid: Option<u32>) {
    if let Some(pid_val) = pid {
        #[cfg(unix)]
        {
            // Send SIGTERM to the process group
            let pgid = pid_val as libc::pid_t;
            unsafe {
                libc::kill(-pgid, libc::SIGTERM);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = child.start_kill();
        }

        // Grace period for graceful shutdown
        tokio::select! {
            _ = child.wait() => return,
            _ = tokio::time::sleep(Duration::from_millis(200)) => {}
        }

        // Escalate to SIGKILL if child is still running
        #[cfg(unix)]
        {
            let pgid = pid_val as libc::pid_t;
            unsafe {
                libc::kill(-pgid, libc::SIGKILL);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = child.kill().await;
        }
    } else {
        let _ = child.kill().await;
    }

    // Wait and reap child process to prevent zombies
    let _ = child.wait().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_managed_process_success() {
        let echo_path = if cfg!(windows) {
            PathBuf::from("cmd.exe")
        } else {
            PathBuf::from("/bin/echo")
        };

        if !echo_path.exists() {
            return;
        }

        let spec = ProcessSpec::new(
            echo_path,
            vec!["OpenTakeMediaTest".to_string()],
            Duration::from_secs(2),
        );
        let result = run_managed_process(spec, None)
            .await
            .expect("Should succeed");
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("OpenTakeMediaTest"));
        assert!(!result.is_stdout_truncated);
    }

    #[tokio::test]
    async fn test_run_managed_process_timeout() {
        let sleep_path = PathBuf::from("/bin/sleep");
        if !sleep_path.exists() {
            return;
        }

        // Run sleep for 5 seconds with a 50ms timeout
        let spec = ProcessSpec::new(sleep_path, vec!["5".to_string()], Duration::from_millis(50));
        let result = run_managed_process(spec, None).await;
        match result {
            Err(MediaEngineError::Timeout { duration_ms, .. }) => {
                assert_eq!(duration_ms, 50);
            }
            other => panic!("Expected Timeout error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_managed_process_cancellation() {
        let sleep_path = PathBuf::from("/bin/sleep");
        if !sleep_path.exists() {
            return;
        }

        let token = CancellationToken::new();
        let cancel_token_clone = token.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            cancel_token_clone.cancel();
        });

        let spec = ProcessSpec::new(sleep_path, vec!["5".to_string()], Duration::from_secs(5));
        let result = run_managed_process(spec, Some(&token)).await;
        match result {
            Err(MediaEngineError::Cancelled { .. }) => {}
            other => panic!("Expected Cancelled error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_run_managed_process_bounded_stdout() {
        let echo_path = PathBuf::from("/bin/echo");
        if !echo_path.exists() {
            return;
        }

        let mut spec = ProcessSpec::new(
            echo_path,
            vec!["1234567890abcdefghijklmnopqrstuvwxyz".to_string()],
            Duration::from_secs(2),
        );
        spec.max_stdout_bytes = 8; // Restrict to 8 bytes

        let result = run_managed_process(spec, None)
            .await
            .expect("Should succeed");
        assert!(result.is_stdout_truncated);
        assert_eq!(result.stdout.len(), 8);
    }
}
