use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use opentake_core::media::identity::MediaAsset;
use opentake_core::media::proxy::ProxyEngine;
use opentake_core::media::range::determine_mime_type;

use super::media_import::MediaLibraryState;

/// Status descriptor for preview proxy generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ProxyStateInfo {
    None,
    Generating,
    Ready { proxy_path: String },
    Failed { error: String },
    Cancelled,
}

/// Rich playback information returned to the frontend preview monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackInfo {
    pub asset_id: String,
    pub playback_url: String,
    pub is_proxy: bool,
    pub mime_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub fps: Option<f64>,
    pub has_audio: bool,
    pub proxy_state: ProxyStateInfo,
}

/// Response returned by proxy query/trigger commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatusResponse {
    pub asset_id: String,
    pub status: ProxyStateInfo,
}

/// Shared application state for playback and preview proxy workflows.
#[derive(Clone)]
pub struct PlaybackState {
    pub proxy_engine: Arc<ProxyEngine>,
    pub active_cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub proxy_status: Arc<RwLock<HashMap<String, ProxyStateInfo>>>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        let cache_dir = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| PathBuf::from(h).join(".cache"))
                    .unwrap_or_else(std::env::temp_dir)
            })
            .join("opentake")
            .join("proxies");

        Self {
            proxy_engine: Arc::new(ProxyEngine::new(cache_dir, None, None)),
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
            proxy_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl PlaybackState {
    pub fn new(cache_dir: PathBuf, ffmpeg: Option<PathBuf>, ffprobe: Option<PathBuf>) -> Self {
        Self {
            proxy_engine: Arc::new(ProxyEngine::new(cache_dir, ffmpeg, ffprobe)),
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
            proxy_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Resolves an authorized media asset ID into an internal streaming URL and playback metadata.
#[tauri::command]
pub async fn get_media_playback_url(
    asset_id: String,
    library: State<'_, MediaLibraryState>,
    playback: State<'_, PlaybackState>,
) -> Result<PlaybackInfo, String> {
    let asset: MediaAsset = library
        .session
        .get_asset_by_id(&asset_id)
        .await
        .ok_or_else(|| "Media asset not found or authorization revoked".to_string())?;

    // Check if proxy is already cached or requested
    let cached_proxy = playback.proxy_engine.find_cached_proxy(&asset).await;

    let (playback_url, is_proxy, proxy_state) = if let Some(mapping) = cached_proxy {
        (
            format!("opentake-media://localhost/{asset_id}?proxy=1"),
            true,
            ProxyStateInfo::Ready {
                proxy_path: mapping.proxy_path.to_string_lossy().to_string(),
            },
        )
    } else {
        let status_read = playback.proxy_status.read().await;
        let current_status = status_read
            .get(&asset_id)
            .cloned()
            .unwrap_or(ProxyStateInfo::None);
        (
            format!("opentake-media://localhost/{asset_id}"),
            false,
            current_status,
        )
    };

    let video_stream = asset.metadata.video_streams.first();
    let width = video_stream.map(|v| v.width);
    let height = video_stream.map(|v| v.height);
    let fps = video_stream
        .and_then(|v| v.average_frame_rate.as_ref())
        .map(|r| r.to_f64_lossy());
    let has_audio = !asset.metadata.audio_streams.is_empty();
    let duration_seconds = asset
        .metadata
        .duration_micros()
        .map(|m| m as f64 / 1_000_000.0);

    let path_buf = PathBuf::from(&asset.source.canonical_path);
    let mime_type =
        determine_mime_type(&path_buf, Some(&asset.metadata.container.format_name)).to_string();

    Ok(PlaybackInfo {
        asset_id,
        playback_url,
        is_proxy,
        mime_type,
        width,
        height,
        duration_seconds,
        fps,
        has_audio,
        proxy_state,
    })
}

/// Triggers background preview proxy generation for an authorized media asset.
#[tauri::command]
pub async fn request_preview_proxy(
    asset_id: String,
    library: State<'_, MediaLibraryState>,
    playback: State<'_, PlaybackState>,
) -> Result<ProxyStatusResponse, String> {
    let asset = library
        .session
        .get_asset_by_id(&asset_id)
        .await
        .ok_or_else(|| "Media asset not found or authorization revoked".to_string())?;

    // Check if already cached
    if let Some(cached) = playback.proxy_engine.find_cached_proxy(&asset).await {
        let status = ProxyStateInfo::Ready {
            proxy_path: cached.proxy_path.to_string_lossy().to_string(),
        };
        let mut write = playback.proxy_status.write().await;
        write.insert(asset_id.clone(), status.clone());
        return Ok(ProxyStatusResponse { asset_id, status });
    }

    // Check if already generating
    {
        let read = playback.proxy_status.read().await;
        if let Some(ProxyStateInfo::Generating) = read.get(&asset_id) {
            return Ok(ProxyStatusResponse {
                asset_id,
                status: ProxyStateInfo::Generating,
            });
        }
    }

    // Create cancellation token
    let token = CancellationToken::new();
    {
        let mut cancels = playback.active_cancellations.lock().await;
        cancels.insert(asset_id.clone(), token.clone());
    }

    // Update status to Generating
    {
        let mut write = playback.proxy_status.write().await;
        write.insert(asset_id.clone(), ProxyStateInfo::Generating);
    }

    // Spawn background task
    let engine = playback.proxy_engine.clone();
    let proxy_status = playback.proxy_status.clone();
    let active_cancels = playback.active_cancellations.clone();
    let id_clone = asset_id.clone();

    tokio::spawn(async move {
        let result = engine.generate_proxy(&asset, Some(&token)).await;

        let final_status = match result {
            Ok(mapping) => ProxyStateInfo::Ready {
                proxy_path: mapping.proxy_path.to_string_lossy().to_string(),
            },
            Err(opentake_core::media::error::MediaEngineError::ProxyCancelled { .. }) => {
                ProxyStateInfo::Cancelled
            }
            Err(e) => ProxyStateInfo::Failed {
                error: e.user_message(),
            },
        };

        let mut write = proxy_status.write().await;
        write.insert(id_clone.clone(), final_status);

        let mut cancels = active_cancels.lock().await;
        cancels.remove(&id_clone);
    });

    Ok(ProxyStatusResponse {
        asset_id,
        status: ProxyStateInfo::Generating,
    })
}

/// Cancels an in-flight preview proxy generation task for an asset.
#[tauri::command]
pub async fn cancel_preview_proxy(
    asset_id: String,
    playback: State<'_, PlaybackState>,
) -> Result<bool, String> {
    let mut cancels = playback.active_cancellations.lock().await;
    if let Some(token) = cancels.remove(&asset_id) {
        token.cancel();
        let mut write = playback.proxy_status.write().await;
        write.insert(asset_id, ProxyStateInfo::Cancelled);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Queries the current proxy status for an asset.
#[tauri::command]
pub async fn get_proxy_status(
    asset_id: String,
    library: State<'_, MediaLibraryState>,
    playback: State<'_, PlaybackState>,
) -> Result<ProxyStatusResponse, String> {
    // If we have an active status in memory, return it
    {
        let read = playback.proxy_status.read().await;
        if let Some(status) = read.get(&asset_id) {
            return Ok(ProxyStatusResponse {
                asset_id,
                status: status.clone(),
            });
        }
    }

    // Check if proxy exists on disk
    if let Some(asset) = library.session.get_asset_by_id(&asset_id).await {
        if let Some(cached) = playback.proxy_engine.find_cached_proxy(&asset).await {
            let status = ProxyStateInfo::Ready {
                proxy_path: cached.proxy_path.to_string_lossy().to_string(),
            };
            return Ok(ProxyStatusResponse { asset_id, status });
        }
    }

    Ok(ProxyStatusResponse {
        asset_id,
        status: ProxyStateInfo::None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_state_initialization() {
        let state = PlaybackState::default();
        let active = state.active_cancellations.blocking_lock();
        assert!(active.is_empty());
    }
}
