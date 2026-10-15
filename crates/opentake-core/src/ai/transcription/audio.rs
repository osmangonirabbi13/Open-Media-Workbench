use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::media::discovery::{discover_tool, DiscoveryConfig, ToolType};
use crate::media::process::{run_managed_process, ProcessSpec};

use super::errors::TranscriptionError;

/// Target audio sample rate required by Whisper models (16 kHz).
pub const WHISPER_TARGET_SAMPLE_RATE: u32 = 16_000;

/// Default timeout for audio extraction preprocessing (15 minutes).
pub const DEFAULT_AUDIO_EXTRACTION_TIMEOUT: Duration = Duration::from_secs(900);

/// Builds the explicit FFmpeg argument vector for converting media into Whisper-compatible WAV.
pub fn build_audio_extraction_args(
    input_path: &Path,
    output_path: &Path,
    start_time_us: Option<i64>,
    end_time_us: Option<i64>,
) -> Vec<String> {
    let mut args = vec!["-y".to_string()];

    // Fast seek if start boundary provided
    if let Some(start_us) = start_time_us {
        if start_us > 0 {
            let start_sec = (start_us as f64) / 1_000_000.0;
            args.push("-ss".to_string());
            args.push(format!("{start_sec:.3}"));
        }
    }

    // Input path
    args.push("-i".to_string());
    args.push(input_path.to_string_lossy().to_string());

    // Duration if end boundary provided
    if let Some(end_us) = end_time_us {
        let start_us = start_time_us.unwrap_or(0);
        if end_us > start_us {
            let duration_sec = ((end_us - start_us) as f64) / 1_000_000.0;
            args.push("-t".to_string());
            args.push(format!("{duration_sec:.3}"));
        }
    }

    // Audio preprocessing flags
    args.extend_from_slice(&[
        "-vn".to_string(),
        "-ac".to_string(),
        "1".to_string(),
        "-ar".to_string(),
        WHISPER_TARGET_SAMPLE_RATE.to_string(),
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
        output_path.to_string_lossy().to_string(),
    ]);

    args
}

/// Extracts media audio into a 16kHz mono 16-bit PCM WAV staging file using system FFmpeg.
pub async fn extract_audio_for_whisper(
    input_path: &Path,
    output_path: &Path,
    start_time_us: Option<i64>,
    end_time_us: Option<i64>,
    cancel_token: CancellationToken,
) -> Result<(), TranscriptionError> {
    if !input_path.exists() {
        return Err(TranscriptionError::AudioExtractionFailed(format!(
            "Input file does not exist: {}",
            input_path.display()
        )));
    }

    let discovered = discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default()).map_err(|e| {
        TranscriptionError::AudioExtractionFailed(format!(
            "System FFmpeg binary was not found in standard system paths: {e}"
        ))
    })?;

    // Ensure staging output directory exists
    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            TranscriptionError::AudioExtractionFailed(format!(
                "Failed to create staging audio directory: {e}"
            ))
        })?;
    }

    let args = build_audio_extraction_args(input_path, output_path, start_time_us, end_time_us);
    let spec = ProcessSpec::new(
        discovered.canonical_path,
        args,
        DEFAULT_AUDIO_EXTRACTION_TIMEOUT,
    );

    let output = run_managed_process(spec, Some(&cancel_token))
        .await
        .map_err(|e| {
            // Clean up partial output file if error occurred
            let _ = std::fs::remove_file(output_path);
            TranscriptionError::AudioExtractionFailed(format!("FFmpeg process failed: {e}"))
        })?;

    if let Some(exit_code) = output.exit_code {
        if exit_code != 0 {
            let _ = std::fs::remove_file(output_path);
            let stderr_lower = output.stderr.to_lowercase();
            if stderr_lower.contains("does not contain any audio stream")
                || stderr_lower.contains("output file does not contain any stream")
                || stderr_lower.contains("matches no streams")
            {
                return Err(TranscriptionError::NoAudioStream(format!(
                    "Media file '{}' does not contain any readable audio stream.",
                    input_path.display()
                )));
            }
            return Err(TranscriptionError::AudioExtractionFailed(format!(
                "FFmpeg extraction exited with code {exit_code}: {}",
                output.stderr.lines().last().unwrap_or("Unknown error")
            )));
        }
    } else {
        let _ = std::fs::remove_file(output_path);
        return Err(TranscriptionError::AudioExtractionFailed(
            "FFmpeg extraction was terminated abnormally.".to_string(),
        ));
    }

    if !output_path.exists() {
        return Err(TranscriptionError::AudioExtractionFailed(
            "FFmpeg completed but target WAV file was not created.".to_string(),
        ));
    }

    Ok(())
}

/// Safe staging file guard that removes the file on Drop if not disarmed.
pub struct StagingAudioGuard {
    pub path: PathBuf,
    disarmed: bool,
}

impl StagingAudioGuard {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            disarmed: false,
        }
    }

    pub fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for StagingAudioGuard {
    fn drop(&mut self) {
        if !self.disarmed && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
