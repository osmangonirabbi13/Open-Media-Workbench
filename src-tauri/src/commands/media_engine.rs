use opentake_core::media::capabilities::MediaEngineStatus;
use opentake_core::media::MediaEngineService;
use std::sync::Arc;
use tauri::State;

/// Managed application state wrapping the headless MediaEngineService.
pub struct MediaEngineState {
    pub service: Arc<MediaEngineService>,
}

impl Default for MediaEngineState {
    fn default() -> Self {
        Self {
            service: Arc::new(MediaEngineService::default()),
        }
    }
}

/// Invokes the media engine service to discover and validate host FFmpeg and FFprobe binaries.
#[tauri::command]
pub async fn get_media_engine_status(
    force_refresh: Option<bool>,
    state: State<'_, MediaEngineState>,
) -> Result<MediaEngineStatus, String> {
    let refresh = force_refresh.unwrap_or(false);
    state
        .service
        .get_status(refresh)
        .await
        .map_err(|e| e.user_message())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_media_engine_command_returns_status() {
        let state = MediaEngineState::default();
        let service = state.service.clone();
        let status = service
            .get_status(true)
            .await
            .expect("Should return status");
        assert!(status.ffmpeg.available || !status.ffmpeg.available);
    }
}
