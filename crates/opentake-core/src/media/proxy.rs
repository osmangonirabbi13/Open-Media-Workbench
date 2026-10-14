use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use super::discovery::{discover_tool, DiscoveryConfig, ToolType};
use super::error::MediaEngineError;
use super::identity::MediaAsset;
use super::probe::probe_media_file;
use super::process::{run_managed_process, ProcessSpec};

/// Default maximum timeout for generating a preview proxy (60 seconds).
pub const DEFAULT_PROXY_TIMEOUT: Duration = Duration::from_secs(60);

/// Current schema version for proxy cache invalidation.
pub const PROXY_CACHE_SCHEMA_VERSION: &str = "opentake-proxy-v1";

/// Maximum vertical resolution for preview proxies (720p).
pub const PROXY_MAX_HEIGHT: u32 = 720;

/// Information recording the relationship between original source and generated proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyMapping {
    pub source_canonical_path: String,
    pub proxy_path: PathBuf,
    pub source_duration_micros: Option<u64>,
    pub proxy_duration_micros: Option<u64>,
    pub cache_key: String,
}

/// Generates a deterministic cache key for a media asset's preview proxy.
pub fn compute_proxy_cache_key(asset: &MediaAsset) -> String {
    let mut hasher = Sha256::new();
    hasher.update(PROXY_CACHE_SCHEMA_VERSION.as_bytes());
    hasher.update(asset.source.canonical_path.as_bytes());
    hasher.update(asset.source.file_size.to_le_bytes());
    if let Some(mtime) = asset.source.modified_epoch_ms {
        hasher.update(mtime.to_le_bytes());
    }
    hasher.update(PROXY_MAX_HEIGHT.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

/// Manages the generation, caching, and retrieval of preview proxy files.
#[derive(Debug, Clone)]
pub struct ProxyEngine {
    cache_dir: PathBuf,
    ffmpeg_path: Option<PathBuf>,
    ffprobe_path: Option<PathBuf>,
}

impl ProxyEngine {
    pub fn new(
        cache_dir: PathBuf,
        ffmpeg_path: Option<PathBuf>,
        ffprobe_path: Option<PathBuf>,
    ) -> Self {
        Self {
            cache_dir,
            ffmpeg_path,
            ffprobe_path,
        }
    }

    /// Returns the target finalized proxy file path for a cache key.
    pub fn get_proxy_path(&self, cache_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{cache_key}.mp4"))
    }

    /// Checks if a valid, existing proxy is already available in the cache.
    pub async fn find_cached_proxy(&self, asset: &MediaAsset) -> Option<ProxyMapping> {
        let cache_key = compute_proxy_cache_key(asset);
        let proxy_path = self.get_proxy_path(&cache_key);

        if proxy_path.exists() {
            // Verify file has non-zero size
            if let Ok(metadata) = fs::metadata(&proxy_path) {
                if metadata.is_file() && metadata.len() > 0 {
                    return Some(ProxyMapping {
                        source_canonical_path: asset.source.canonical_path.clone(),
                        proxy_path,
                        source_duration_micros: asset.metadata.duration_micros(),
                        proxy_duration_micros: asset.metadata.duration_micros(),
                        cache_key,
                    });
                }
            }
        }

        None
    }

    /// Generates a preview proxy for an imported media asset.
    ///
    /// # Transcoding Rules
    /// - Scales video to maximum height 720p preserving aspect ratio.
    /// - Converts to H.264 video with `yuv420p` pixel format.
    /// - Retains AAC audio if present in source; omits audio (`-an`) if source is silent.
    /// - Adds `-movflags +faststart` for immediate playback streaming.
    /// - Verifies output with FFprobe before finalizing.
    /// - Cleans up partial `.tmp.mp4` on failure or cancellation.
    pub async fn generate_proxy(
        &self,
        asset: &MediaAsset,
        cancellation: Option<&CancellationToken>,
    ) -> Result<ProxyMapping, MediaEngineError> {
        // 1. Check existing cache first
        if let Some(cached) = self.find_cached_proxy(asset).await {
            return Ok(cached);
        }

        // 2. Ensure cache directory exists
        fs::create_dir_all(&self.cache_dir).map_err(|e| {
            MediaEngineError::Internal(format!("Failed to create proxy cache directory: {e}"))
        })?;

        let cache_key = compute_proxy_cache_key(asset);
        let final_path = self.get_proxy_path(&cache_key);
        let temp_path = self.cache_dir.join(format!("{cache_key}.tmp.mp4"));

        // Remove any stale temporary file from a previous interrupted run
        if temp_path.exists() {
            let _ = fs::remove_file(&temp_path);
        }

        // 3. Resolve ffmpeg binary
        let ffmpeg_binary = match &self.ffmpeg_path {
            Some(p) => p.clone(),
            None => {
                let discovered = discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default())?;
                discovered.canonical_path
            }
        };

        // 4. Build safe argument vector
        let has_audio = !asset.metadata.audio_streams.is_empty();
        let scale_filter = "scale=w='min(1280,iw)':h=-2";

        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            asset.source.canonical_path.clone(),
            "-vf".to_string(),
            scale_filter.to_string(),
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "veryfast".to_string(),
            "-crf".to_string(),
            "26".to_string(),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
        ];

        if has_audio {
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-b:a".to_string());
            args.push("128k".to_string());
        } else {
            args.push("-an".to_string());
        }

        args.push("--".to_string());
        args.push(temp_path.to_string_lossy().to_string());

        let mut spec = ProcessSpec::new(ffmpeg_binary, args, DEFAULT_PROXY_TIMEOUT);
        spec.max_stdout_bytes = 256 * 1024;
        spec.max_stderr_bytes = 512 * 1024;

        // 5. Run transcode process
        let output = match run_managed_process(spec, cancellation).await {
            Ok(out) => out,
            Err(e) => {
                // Clean up partial temp output
                if temp_path.exists() {
                    let _ = fs::remove_file(&temp_path);
                }
                return Err(match e {
                    MediaEngineError::Cancelled { .. } => MediaEngineError::ProxyCancelled {
                        id: asset.id.clone(),
                    },
                    other => MediaEngineError::ProxyGenerationFailed {
                        id: asset.id.clone(),
                        reason: other.user_message(),
                    },
                });
            }
        };

        if let Some(code) = output.exit_code {
            if code != 0 {
                if temp_path.exists() {
                    let _ = fs::remove_file(&temp_path);
                }
                return Err(MediaEngineError::ProxyGenerationFailed {
                    id: asset.id.clone(),
                    reason: output.stderr.trim().to_string(),
                });
            }
        }

        // 6. Verify generated proxy with FFprobe before finalizing
        let temp_size = match fs::metadata(&temp_path) {
            Ok(m) => m.len(),
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                return Err(MediaEngineError::ProxyGenerationFailed {
                    id: asset.id.clone(),
                    reason: format!("Generated proxy file is inaccessible: {e}"),
                });
            }
        };

        if temp_size == 0 {
            let _ = fs::remove_file(&temp_path);
            return Err(MediaEngineError::ProxyGenerationFailed {
                id: asset.id.clone(),
                reason: "Generated proxy file is 0 bytes".to_string(),
            });
        }

        let probe_result = probe_media_file(
            self.ffprobe_path.as_ref(),
            &temp_path,
            temp_size,
            cancellation,
        )
        .await;

        let verified_meta = match probe_result {
            Ok(m) => m,
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                return Err(MediaEngineError::ProxyGenerationFailed {
                    id: asset.id.clone(),
                    reason: format!("Generated proxy failed verification probe: {e}"),
                });
            }
        };

        // 7. Atomic rename temp -> final
        fs::rename(&temp_path, &final_path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            MediaEngineError::Internal(format!("Failed to finalize proxy file: {e}"))
        })?;

        Ok(ProxyMapping {
            source_canonical_path: asset.source.canonical_path.clone(),
            proxy_path: final_path,
            source_duration_micros: asset.metadata.duration_micros(),
            proxy_duration_micros: verified_meta.duration_micros(),
            cache_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::compatibility::CompatibilityStatus;
    use crate::media::compatibility::MediaCompatibility;
    use crate::media::identity::SourceIdentity;
    use crate::media::metadata::ContainerMetadata;
    use crate::media::metadata::MediaMetadata;

    #[test]
    fn test_compute_proxy_cache_key_deterministic() {
        let source1 = SourceIdentity::new("/path/to/clip.mov".to_string(), 50000, Some(12345));
        let meta1 = MediaMetadata {
            container: ContainerMetadata {
                format_name: "mov".to_string(),
                format_long_name: None,
                duration_micros: Some(5_000_000),
                start_time_micros: None,
                bitrate_bps: None,
                size_bytes: 50000,
                stream_count: 1,
            },
            video_streams: vec![],
            audio_streams: vec![],
            other_streams: vec![],
        };
        let compat1 = MediaCompatibility {
            overall: CompatibilityStatus::Supported,
            container_status: CompatibilityStatus::Supported,
            video_status: CompatibilityStatus::Supported,
            audio_status: CompatibilityStatus::Supported,
            can_inspect: true,
            can_edit_future: true,
            warnings: vec![],
        };
        let asset1 = MediaAsset::new(source1, meta1, compat1);

        let key1 = compute_proxy_cache_key(&asset1);
        let key2 = compute_proxy_cache_key(&asset1);
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 64); // SHA-256 hex string
    }
}
