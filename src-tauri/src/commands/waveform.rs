use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use opentake_core::media::identity::MediaAsset;
use opentake_core::media::waveform::{WaveformData, WaveformEngine, WaveformSegment};

use super::media_import::MediaLibraryState;

/// Status descriptor for asynchronous audio waveform extraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum WaveformStatusInfo {
    None,
    Extracting,
    Ready {
        duration_micros: u64,
        total_samples: u64,
    },
    NoAudio,
    Failed {
        error: String,
    },
    Cancelled,
}

/// Response returned by waveform query and trigger commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformStatusResponse {
    pub asset_id: String,
    pub status: WaveformStatusInfo,
}

/// Response returned by segment slice queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformSegmentResponse {
    pub asset_id: String,
    pub segment: Option<WaveformSegment>,
    pub status: WaveformStatusInfo,
}

/// Shared application state for background waveform extraction jobs and multi-LOD cache.
#[derive(Clone)]
pub struct WaveformState {
    pub engine: Arc<WaveformEngine>,
    pub active_cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub status_map: Arc<RwLock<HashMap<String, WaveformStatusInfo>>>,
    pub in_memory_cache: Arc<RwLock<HashMap<String, Arc<WaveformData>>>>,
}

impl Default for WaveformState {
    fn default() -> Self {
        let cache_dir = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| PathBuf::from(h).join(".cache"))
                    .unwrap_or_else(std::env::temp_dir)
            })
            .join("opentake")
            .join("waveforms");

        Self {
            engine: Arc::new(WaveformEngine::new(cache_dir, None)),
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
            status_map: Arc::new(RwLock::new(HashMap::new())),
            in_memory_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Queries current waveform status for an authorized media asset.
#[tauri::command]
pub async fn get_waveform_status(
    asset_id: String,
    library: State<'_, MediaLibraryState>,
    waveform: State<'_, WaveformState>,
) -> Result<WaveformStatusResponse, String> {
    let asset: MediaAsset = library
        .session
        .get_asset_by_id(&asset_id)
        .await
        .ok_or_else(|| "Media asset not found or authorization revoked".to_string())?;

    // Check if media contains no audio
    if asset.metadata.audio_streams.is_empty() {
        return Ok(WaveformStatusResponse {
            asset_id,
            status: WaveformStatusInfo::NoAudio,
        });
    }

    // Check in-flight status map
    {
        let read = waveform.status_map.read().await;
        if let Some(status) = read.get(&asset_id) {
            return Ok(WaveformStatusResponse {
                asset_id,
                status: status.clone(),
            });
        }
    }

    // Check in-memory dataset cache
    {
        let cache_read = waveform.in_memory_cache.read().await;
        if let Some(data) = cache_read.get(&asset_id) {
            let status = WaveformStatusInfo::Ready {
                duration_micros: data.duration_micros,
                total_samples: data.total_samples,
            };
            return Ok(WaveformStatusResponse { asset_id, status });
        }
    }

    // Check disk cache
    if let Ok(Some(cached)) = waveform.engine.find_cached_waveform(&asset, 0) {
        let status = WaveformStatusInfo::Ready {
            duration_micros: cached.duration_micros,
            total_samples: cached.total_samples,
        };
        let mut cache_write = waveform.in_memory_cache.write().await;
        cache_write.insert(asset_id.clone(), Arc::new(cached));
        return Ok(WaveformStatusResponse { asset_id, status });
    }

    Ok(WaveformStatusResponse {
        asset_id,
        status: WaveformStatusInfo::None,
    })
}

/// Triggers background waveform extraction and multi-LOD pyramid generation for an authorized media asset.
#[tauri::command]
pub async fn request_waveform_analysis(
    asset_id: String,
    library: State<'_, MediaLibraryState>,
    waveform: State<'_, WaveformState>,
) -> Result<WaveformStatusResponse, String> {
    let asset: MediaAsset = library
        .session
        .get_asset_by_id(&asset_id)
        .await
        .ok_or_else(|| "Media asset not found or authorization revoked".to_string())?;

    // Check if media has no audio
    if asset.metadata.audio_streams.is_empty() {
        return Ok(WaveformStatusResponse {
            asset_id,
            status: WaveformStatusInfo::NoAudio,
        });
    }

    // Check if already in memory
    {
        let cache_read = waveform.in_memory_cache.read().await;
        if let Some(data) = cache_read.get(&asset_id) {
            let status = WaveformStatusInfo::Ready {
                duration_micros: data.duration_micros,
                total_samples: data.total_samples,
            };
            return Ok(WaveformStatusResponse { asset_id, status });
        }
    }

    // Check if on disk
    if let Ok(Some(cached)) = waveform.engine.find_cached_waveform(&asset, 0) {
        let status = WaveformStatusInfo::Ready {
            duration_micros: cached.duration_micros,
            total_samples: cached.total_samples,
        };
        let mut cache_write = waveform.in_memory_cache.write().await;
        cache_write.insert(asset_id.clone(), Arc::new(cached));
        return Ok(WaveformStatusResponse { asset_id, status });
    }

    // Check if already extracting
    {
        let read = waveform.status_map.read().await;
        if let Some(WaveformStatusInfo::Extracting) = read.get(&asset_id) {
            return Ok(WaveformStatusResponse {
                asset_id,
                status: WaveformStatusInfo::Extracting,
            });
        }
    }

    // Register cancellation token
    let token = CancellationToken::new();
    {
        let mut cancels = waveform.active_cancellations.lock().await;
        cancels.insert(asset_id.clone(), token.clone());
    }

    // Set extracting state
    {
        let mut write = waveform.status_map.write().await;
        write.insert(asset_id.clone(), WaveformStatusInfo::Extracting);
    }

    // Spawn background streaming extraction task
    let engine = waveform.engine.clone();
    let status_map = waveform.status_map.clone();
    let in_memory_cache = waveform.in_memory_cache.clone();
    let active_cancels = waveform.active_cancellations.clone();
    let id_clone = asset_id.clone();

    tokio::spawn(async move {
        let result = engine.extract_waveform(&asset, 0, Some(&token)).await;

        let final_status = match result {
            Ok(data) => {
                let status = WaveformStatusInfo::Ready {
                    duration_micros: data.duration_micros,
                    total_samples: data.total_samples,
                };
                let mut cache_write = in_memory_cache.write().await;
                cache_write.insert(id_clone.clone(), Arc::new(data));
                status
            }
            Err(opentake_core::media::error::MediaEngineError::WaveformCancelled { .. }) => {
                WaveformStatusInfo::Cancelled
            }
            Err(opentake_core::media::error::MediaEngineError::NoAudioTrack { .. }) => {
                WaveformStatusInfo::NoAudio
            }
            Err(e) => WaveformStatusInfo::Failed {
                error: e.user_message(),
            },
        };

        let mut write = status_map.write().await;
        write.insert(id_clone.clone(), final_status);

        let mut cancels = active_cancels.lock().await;
        cancels.remove(&id_clone);
    });

    Ok(WaveformStatusResponse {
        asset_id,
        status: WaveformStatusInfo::Extracting,
    })
}

/// Cancels in-flight waveform analysis for an asset.
#[tauri::command]
pub async fn cancel_waveform_analysis(
    asset_id: String,
    waveform: State<'_, WaveformState>,
) -> Result<bool, String> {
    let mut cancels = waveform.active_cancellations.lock().await;
    if let Some(token) = cancels.remove(&asset_id) {
        token.cancel();
        let mut write = waveform.status_map.write().await;
        write.insert(asset_id, WaveformStatusInfo::Cancelled);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Queries a bounded slice of waveform amplitude bins covering a visible source interval.
#[tauri::command]
pub async fn get_waveform_segment(
    asset_id: String,
    start_micros: i64,
    end_micros: i64,
    pixel_width: u32,
    library: State<'_, MediaLibraryState>,
    waveform: State<'_, WaveformState>,
) -> Result<WaveformSegmentResponse, String> {
    let asset: MediaAsset = library
        .session
        .get_asset_by_id(&asset_id)
        .await
        .ok_or_else(|| "Media asset not found or authorization revoked".to_string())?;

    if asset.metadata.audio_streams.is_empty() {
        return Ok(WaveformSegmentResponse {
            asset_id,
            segment: None,
            status: WaveformStatusInfo::NoAudio,
        });
    }

    // Try in-memory cache first
    {
        let cache_read = waveform.in_memory_cache.read().await;
        if let Some(data) = cache_read.get(&asset_id) {
            let segment = data.get_segment(start_micros, end_micros, pixel_width);
            return Ok(WaveformSegmentResponse {
                asset_id,
                segment: Some(segment),
                status: WaveformStatusInfo::Ready {
                    duration_micros: data.duration_micros,
                    total_samples: data.total_samples,
                },
            });
        }
    }

    // Try disk cache
    if let Ok(Some(cached)) = waveform.engine.find_cached_waveform(&asset, 0) {
        let segment = cached.get_segment(start_micros, end_micros, pixel_width);
        let status = WaveformStatusInfo::Ready {
            duration_micros: cached.duration_micros,
            total_samples: cached.total_samples,
        };
        let mut cache_write = waveform.in_memory_cache.write().await;
        cache_write.insert(asset_id.clone(), Arc::new(cached));

        return Ok(WaveformSegmentResponse {
            asset_id,
            segment: Some(segment),
            status,
        });
    }

    // Check status map
    let status_read = waveform.status_map.read().await;
    let current_status = status_read
        .get(&asset_id)
        .cloned()
        .unwrap_or(WaveformStatusInfo::None);

    Ok(WaveformSegmentResponse {
        asset_id,
        segment: None,
        status: current_status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_state_initialization() {
        let state = WaveformState::default();
        let cancels = state.active_cancellations.blocking_lock();
        assert!(cancels.is_empty());
    }
}
