//! Subtitle Burn-In Video Export Pipeline using FFmpeg and libass.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::localization::subtitles::model::SubtitleTrack;
use crate::media::process::{run_managed_process, ProcessSpec};

use super::ass::generate_ass_script;
use super::errors::StylingError;
use super::model::{AudioExportPolicy, ExportResolution, SubtitleExportProfile, SubtitleStyle};

/// Request payload to initiate a video subtitle burn-in export job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleBurnInRequest {
    pub job_id: String,
    pub source_media_path: PathBuf,
    pub track: SubtitleTrack,
    pub style: SubtitleStyle,
    pub profile: SubtitleExportProfile,
    pub staging_dir: PathBuf,
}

/// Dynamic progress and lifecycle status of a burn-in export job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "stage")]
pub enum SubtitleBurnInStatus {
    Queued,
    Preparing,
    GeneratingAss,
    Encoding {
        progress_pct: f64,
        message: String,
    },
    Validating,
    ReadyToSave {
        staged_output_path: PathBuf,
        size_bytes: u64,
        duration_secs: f64,
        width: u32,
        height: u32,
    },
    Completed {
        final_destination_path: PathBuf,
    },
    Failed {
        error: String,
    },
    Cancelled,
}

/// Comprehensive outcome of a successful burn-in export and validation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleBurnInResult {
    pub job_id: String,
    pub staged_output_path: PathBuf,
    pub file_size_bytes: u64,
    pub duration_secs: f64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub validation_warnings: Vec<String>,
}

/// Builds escaped FFmpeg filtergraph parameter for the ASS subtitle filter.
/// FFmpeg filter option parsing requires colons and backslashes in filenames to be escaped.
pub fn build_ass_filter_arg(ass_path: &Path) -> String {
    let raw = ass_path.to_string_lossy();
    // Escape backslashes first, then colons
    let escaped = raw.replace('\\', "\\\\").replace(':', "\\:");
    format!("ass='{}'", escaped)
}

/// Validates FFmpeg and libass filter availability on the host.
pub fn verify_ffmpeg_burnin_support() -> Result<(), StylingError> {
    let output = Command::new("ffmpeg")
        .arg("-filters")
        .output()
        .map_err(|e| StylingError::FfmpegError(format!("Failed to execute ffmpeg: {e}")))?;

    if !output.status.success() {
        return Err(StylingError::FfmpegError(
            "ffmpeg exited with error status".into(),
        ));
    }

    let filters_str = String::from_utf8_lossy(&output.stdout);
    if !filters_str.contains(" ass ") && !filters_str.contains(" subtitles ") {
        return Err(StylingError::FfmpegError(
            "Installed FFmpeg does not support libass (filters 'ass' or 'subtitles' missing)"
                .into(),
        ));
    }

    Ok(())
}

/// Probes output file metadata via ffprobe to strictly validate output container and streams.
pub fn probe_video_output(
    path: &Path,
) -> Result<(f64, u32, u32, String, Option<String>), StylingError> {
    if !path.exists() {
        return Err(StylingError::EmptyOutput(format!(
            "Output file does not exist: {}",
            path.display()
        )));
    }

    let size = fs::metadata(path).map_err(StylingError::IoError)?.len();
    if size == 0 {
        return Err(StylingError::EmptyOutput(format!(
            "Output file is 0 bytes: {}",
            path.display()
        )));
    }

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,codec_name",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
            &path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| StylingError::ValidationFailed(format!("Failed to execute ffprobe: {e}")))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(StylingError::ValidationFailed(format!(
            "ffprobe validation failed: {err_msg}"
        )));
    }

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| StylingError::ValidationFailed(format!("Invalid ffprobe JSON output: {e}")))?;

    let stream = parsed["streams"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| {
            StylingError::ValidationFailed("No video stream found in rendered output".into())
        })?;

    let width = stream["width"].as_u64().unwrap_or(0) as u32;
    let height = stream["height"].as_u64().unwrap_or(0) as u32;
    let vcodec = stream["codec_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    if width == 0 || height == 0 {
        return Err(StylingError::ValidationFailed(
            "Rendered output has invalid video stream dimensions (0x0)".into(),
        ));
    }

    let duration_secs = parsed["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    // Audio check
    let a_output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &path.to_string_lossy(),
        ])
        .output()
        .ok();

    let audio_codec = a_output
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok((duration_secs, width, height, vcodec, audio_codec))
}

/// Executes the full burn-in rendering pipeline.
pub async fn execute_subtitle_burn_in(
    request: SubtitleBurnInRequest,
    cancellation: Option<&CancellationToken>,
) -> Result<SubtitleBurnInResult, StylingError> {
    // 1. Technical validation
    verify_ffmpeg_burnin_support()?;

    if !request.source_media_path.exists() {
        return Err(StylingError::InvalidProperty(format!(
            "Source media file not found: {}",
            request.source_media_path.display()
        )));
    }

    // 2. Prepare staging directories
    fs::create_dir_all(&request.staging_dir)?;
    let ass_path = request.staging_dir.join(format!("{}.ass", request.job_id));
    let staged_output_path = request
        .staging_dir
        .join(format!("{}_burned.mp4", request.job_id));

    // 3. Generate ASS Script
    let ass_content = generate_ass_script(&request.track, &request.style)?;
    fs::write(&ass_path, ass_content)?;

    // 4. Build Filtergraph
    let ass_filter = build_ass_filter_arg(&ass_path);
    let video_filter = match request.profile.resolution {
        ExportResolution::Original => ass_filter,
        ExportResolution::Res1080p => {
            format!("scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2,{ass_filter}")
        }
        ExportResolution::Res720p => {
            format!("scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2,{ass_filter}")
        }
        ExportResolution::Custom { width, height } => {
            format!("scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,{ass_filter}")
        }
    };

    // 5. Build structured FFmpeg argument vector
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        request.source_media_path.to_string_lossy().to_string(),
        "-vf".to_string(),
        video_filter,
        "-c:v".to_string(),
        request.profile.video_codec.clone(),
        "-preset".to_string(),
        request.profile.preset.clone(),
        "-crf".to_string(),
        request.profile.crf.to_string(),
    ];

    match request.profile.audio_policy {
        AudioExportPolicy::Copy => {
            args.push("-c:a".to_string());
            args.push("copy".to_string());
        }
        AudioExportPolicy::Aac => {
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-b:a".to_string());
            args.push("192k".to_string());
        }
    }

    args.push("-movflags".to_string());
    args.push("+faststart".to_string());

    args.push(staged_output_path.to_string_lossy().to_string());

    // 6. Execute managed process with cancellation and process group isolation
    let spec = ProcessSpec {
        program: PathBuf::from("ffmpeg"),
        args,
        timeout: Duration::from_secs(600), // 10 minutes max
        max_stdout_bytes: 256 * 1024,
        max_stderr_bytes: 1024 * 1024,
        working_dir: Some(request.staging_dir.clone()),
    };

    let result = run_managed_process(spec, cancellation)
        .await
        .map_err(|e| StylingError::FfmpegError(format!("{e}")))?;

    if let Some(code) = result.exit_code {
        if code != 0 {
            // Clean up temporary staged output if failed
            let _ = fs::remove_file(&staged_output_path);
            let _ = fs::remove_file(&ass_path);
            return Err(StylingError::FfmpegError(format!(
                "FFmpeg exited with non-zero code {code}: {}",
                result.stderr
            )));
        }
    }

    // Clean up temporary ASS file
    let _ = fs::remove_file(&ass_path);

    // 7. Validate output container and stream integrity
    let (duration_secs, width, height, vcodec, audio_codec) =
        probe_video_output(&staged_output_path)?;
    let size_bytes = fs::metadata(&staged_output_path)
        .map_err(StylingError::IoError)?
        .len();

    let mut warnings = Vec::new();
    if duration_secs == 0.0 {
        warnings.push("Could not determine duration of output video".to_string());
    }

    Ok(SubtitleBurnInResult {
        job_id: request.job_id,
        staged_output_path,
        file_size_bytes: size_bytes,
        duration_secs,
        width,
        height,
        video_codec: vcodec,
        audio_codec,
        validation_warnings: warnings,
    })
}

/// Central burn-in export job state manager.
pub struct BurnInExportManager {
    current_status: Arc<RwLock<Option<SubtitleBurnInStatus>>>,
    active_token: Arc<RwLock<Option<CancellationToken>>>,
}

impl Default for BurnInExportManager {
    fn default() -> Self {
        Self {
            current_status: Arc::new(RwLock::new(None)),
            active_token: Arc::new(RwLock::new(None)),
        }
    }
}

impl BurnInExportManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_status(&self) -> SubtitleBurnInStatus {
        let lock = self.current_status.read().await;
        lock.clone().unwrap_or(SubtitleBurnInStatus::Queued)
    }

    pub async fn cancel(&self) {
        let token_lock = self.active_token.read().await;
        if let Some(token) = token_lock.as_ref() {
            token.cancel();
        }
        let mut status_lock = self.current_status.write().await;
        *status_lock = Some(SubtitleBurnInStatus::Cancelled);
    }

    pub async fn start_export(
        &self,
        request: SubtitleBurnInRequest,
    ) -> Result<String, StylingError> {
        let job_id = request.job_id.clone();
        let token = CancellationToken::new();

        {
            let mut token_lock = self.active_token.write().await;
            *token_lock = Some(token.clone());
            let mut status_lock = self.current_status.write().await;
            *status_lock = Some(SubtitleBurnInStatus::Preparing);
        }

        let status_arc = self.current_status.clone();
        let token_clone = token.clone();

        tokio::spawn(async move {
            {
                let mut lock = status_arc.write().await;
                *lock = Some(SubtitleBurnInStatus::Encoding {
                    progress_pct: 10.0,
                    message: "Rendering subtitles with libass and x264...".to_string(),
                });
            }

            match execute_subtitle_burn_in(request, Some(&token_clone)).await {
                Ok(res) => {
                    let mut lock = status_arc.write().await;
                    *lock = Some(SubtitleBurnInStatus::ReadyToSave {
                        staged_output_path: res.staged_output_path,
                        size_bytes: res.file_size_bytes,
                        duration_secs: res.duration_secs,
                        width: res.width,
                        height: res.height,
                    });
                }
                Err(StylingError::Cancelled) => {
                    let mut lock = status_arc.write().await;
                    *lock = Some(SubtitleBurnInStatus::Cancelled);
                }
                Err(e) => {
                    let mut lock = status_arc.write().await;
                    *lock = Some(SubtitleBurnInStatus::Failed {
                        error: e.to_string(),
                    });
                }
            }
        });

        Ok(job_id)
    }

    /// Safely finalizes export by moving/copying staged video to user's selected destination.
    pub async fn finalize_export(
        &self,
        destination_path: &Path,
        source_original_path: &Path,
    ) -> Result<PathBuf, StylingError> {
        // Enforce source media immutability: NEVER overwrite source
        if destination_path == source_original_path {
            return Err(StylingError::InvalidProperty(
                "Destination path cannot be the same as the source video file (source immutability enforced)".into(),
            ));
        }

        let staged_path = {
            let lock = self.current_status.read().await;
            match lock.as_ref() {
                Some(SubtitleBurnInStatus::ReadyToSave {
                    staged_output_path, ..
                }) => staged_output_path.clone(),
                _ => {
                    return Err(StylingError::InvalidProperty(
                        "No completed staged export is ready to finalize".into(),
                    ));
                }
            }
        };

        if !staged_path.exists() {
            return Err(StylingError::EmptyOutput(format!(
                "Staged export missing: {}",
                staged_path.display()
            )));
        }

        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy then remove staged file
        fs::copy(&staged_path, destination_path)?;
        let _ = fs::remove_file(&staged_path);

        let mut lock = self.current_status.write().await;
        *lock = Some(SubtitleBurnInStatus::Completed {
            final_destination_path: destination_path.to_path_buf(),
        });

        Ok(destination_path.to_path_buf())
    }
}
