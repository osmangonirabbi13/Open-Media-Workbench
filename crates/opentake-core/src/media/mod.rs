pub mod authorization;
pub mod capabilities;
pub mod compatibility;
pub mod discovery;
pub mod error;
pub mod identity;
pub mod import;
pub mod metadata;
pub mod probe;
pub mod process;
pub mod proxy;
pub mod range;
pub mod version;
pub mod waveform;
pub mod waveform_cache;

pub use authorization::{AuthorizationRecord, MediaAuthorizationRegistry, SourceProvenance};
pub use compatibility::{CompatibilityStatus, MediaCompatibility};
pub use error::MediaEngineError;
pub use identity::{MediaAsset, SourceIdentity};
pub use import::{
    validate_media_file_path, ImportJobStatus, MediaLibrarySession, ValidatedFile,
    MAX_CONCURRENT_PROBE_JOBS, MAX_IMPORT_FILE_SIZE_BYTES,
};
pub use metadata::{
    normalize_ffprobe_json, AudioStreamMetadata, ContainerMetadata, MediaMetadata,
    OtherStreamMetadata, Rational, VideoStreamMetadata,
};
pub use probe::probe_media_file;
pub use proxy::{
    compute_proxy_cache_key, ProxyEngine, ProxyMapping, DEFAULT_PROXY_TIMEOUT, PROXY_MAX_HEIGHT,
};
pub use range::{
    determine_mime_type, parse_byte_range, read_range_slice, ResolvedByteRange,
    MAX_RANGE_CHUNK_BYTES,
};
pub use waveform::{
    WaveformBin, WaveformData, WaveformEngine, WaveformLodLevel, WaveformSegment,
    BASE_BINS_PER_SECOND, MAX_SEGMENT_BINS, SAMPLES_PER_BASE_BIN, WAVEFORM_ALGORITHM_VERSION,
    WAVEFORM_SAMPLE_RATE,
};
pub use waveform_cache::{
    compute_waveform_cache_key, get_waveform_cache_path, load_waveform_from_cache,
    save_waveform_to_cache, WAVEFORM_CACHE_MAGIC, WAVEFORM_CACHE_SCHEMA_VERSION,
};

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;

use self::capabilities::{MediaEngineCompatibility, MediaEngineStatus, ToolStatus};
use self::discovery::{discover_tool, DiscoveryConfig, ToolType};
use self::process::{run_managed_process, ProcessSpec};
use self::version::parse_version_output;

/// Diagnostic timeout for quick `-version` query execution (3 seconds).
pub const DIAGNOSTIC_TIMEOUT: Duration = Duration::from_secs(3);

/// Headless service managing media tool discovery, verification, and status diagnostics.
#[derive(Debug, Clone)]
pub struct MediaEngineService {
    config: DiscoveryConfig,
    cached_status: Arc<RwLock<Option<MediaEngineStatus>>>,
}

impl Default for MediaEngineService {
    fn default() -> Self {
        Self::new(DiscoveryConfig::default())
    }
}

impl MediaEngineService {
    pub fn new(config: DiscoveryConfig) -> Self {
        Self {
            config,
            cached_status: Arc::new(RwLock::new(None)),
        }
    }

    /// Returns current media engine status, using cached results unless `force_refresh` is true.
    pub async fn get_status(
        &self,
        force_refresh: bool,
    ) -> Result<MediaEngineStatus, MediaEngineError> {
        if !force_refresh {
            let cache_read = self.cached_status.read().await;
            if let Some(ref status) = *cache_read {
                return Ok(status.clone());
            }
        }

        let evaluated_status = self.evaluate_toolchain().await;

        let mut cache_write = self.cached_status.write().await;
        *cache_write = Some(evaluated_status.clone());

        Ok(evaluated_status)
    }

    /// Evaluates both FFmpeg and FFprobe on the host.
    async fn evaluate_toolchain(&self) -> MediaEngineStatus {
        let mut warnings = Vec::new();

        // 1. Evaluate FFmpeg
        let ffmpeg_status = self.diagnose_tool(ToolType::Ffmpeg).await;

        // 2. Evaluate FFprobe
        let ffprobe_status = self.diagnose_tool(ToolType::Ffprobe).await;

        // 3. Determine Overall Compatibility
        let overall_status = match (ffmpeg_status.available, ffprobe_status.available) {
            (true, true) => {
                // Check if major versions are aligned
                if let (Some(maj1), Some(maj2)) =
                    (ffmpeg_status.major_version, ffprobe_status.major_version)
                {
                    if maj1 != maj2 {
                        warnings.push(format!(
                            "Version mismatch: FFmpeg is v{maj1} but FFprobe is v{maj2}. Same-version binaries are recommended."
                        ));
                    }
                }

                // Check libx264 support
                if !ffmpeg_status.has_libx264 {
                    warnings.push(
                        "FFmpeg build does not list libx264 encoder. Video export may be limited."
                            .to_string(),
                    );
                    MediaEngineCompatibility::Degraded
                } else {
                    MediaEngineCompatibility::Ready
                }
            }
            (false, false) => {
                warnings
                    .push("Neither FFmpeg nor FFprobe was found on the host system.".to_string());
                MediaEngineCompatibility::Missing
            }
            (true, false) => {
                warnings.push(
                    "FFprobe was not found. File import and format inspection will fail."
                        .to_string(),
                );
                MediaEngineCompatibility::Missing
            }
            (false, true) => {
                warnings.push(
                    "FFmpeg was not found. Media editing and rendering will fail.".to_string(),
                );
                MediaEngineCompatibility::Missing
            }
        };

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        MediaEngineStatus {
            overall_status,
            ffmpeg: ffmpeg_status,
            ffprobe: ffprobe_status,
            warnings,
            evaluated_at_epoch_ms: now_ms,
        }
    }

    /// Discovers and validates a single tool by executing `-version`.
    pub async fn diagnose_tool(&self, tool: ToolType) -> ToolStatus {
        let discovered = match discover_tool(tool, &self.config) {
            Ok(d) => d,
            Err(e) => {
                return ToolStatus {
                    available: false,
                    error: Some(e.user_message()),
                    ..Default::default()
                };
            }
        };

        let spec = ProcessSpec::new(
            discovered.path.clone(),
            vec!["-version".to_string()],
            DIAGNOSTIC_TIMEOUT,
        );

        let process_output = match run_managed_process(spec, None).await {
            Ok(out) => out,
            Err(e) => {
                return ToolStatus {
                    available: false,
                    executable_path: Some(discovered.path.display().to_string()),
                    canonical_path: Some(discovered.canonical_path.display().to_string()),
                    error: Some(e.user_message()),
                    ..Default::default()
                };
            }
        };

        if let Some(code) = process_output.exit_code {
            if code != 0 {
                return ToolStatus {
                    available: false,
                    executable_path: Some(discovered.path.display().to_string()),
                    canonical_path: Some(discovered.canonical_path.display().to_string()),
                    error: Some(format!("Tool exited with non-zero code {code}")),
                    ..Default::default()
                };
            }
        }

        let parsed_info = match parse_version_output(tool, &process_output.stdout) {
            Ok(info) => info,
            Err(e) => {
                return ToolStatus {
                    available: false,
                    executable_path: Some(discovered.path.display().to_string()),
                    canonical_path: Some(discovered.canonical_path.display().to_string()),
                    error: Some(e.user_message()),
                    ..Default::default()
                };
            }
        };

        ToolStatus {
            available: true,
            executable_path: Some(discovered.path.display().to_string()),
            canonical_path: Some(discovered.canonical_path.display().to_string()),
            version: Some(parsed_info.display_version()),
            major_version: parsed_info.major,
            is_gpl: parsed_info.is_gpl,
            has_libx264: parsed_info.has_libx264,
            has_libopus: parsed_info.has_libopus,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_media_engine_service_evaluation() {
        let service = MediaEngineService::default();
        let status = service
            .get_status(true)
            .await
            .expect("Status query should not fail");

        // On Ubuntu workstation with /usr/bin/ffmpeg and /usr/bin/ffprobe installed:
        if std::path::Path::new("/usr/bin/ffmpeg").exists()
            && std::path::Path::new("/usr/bin/ffprobe").exists()
        {
            assert!(status.ffmpeg.available);
            assert!(status.ffprobe.available);
            assert_eq!(status.overall_status, MediaEngineCompatibility::Ready);
            assert_eq!(status.ffmpeg.major_version, Some(8));
            assert_eq!(status.ffprobe.major_version, Some(8));
            assert!(status.ffmpeg.is_gpl);
            assert!(status.ffmpeg.has_libx264);
        }

        // Second call should return cached status
        let cached = service
            .get_status(false)
            .await
            .expect("Cache query should succeed");
        assert_eq!(status.evaluated_at_epoch_ms, cached.evaluated_at_epoch_ms);
    }
}
