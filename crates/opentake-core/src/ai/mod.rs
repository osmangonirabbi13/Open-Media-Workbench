//! # Shared AI Model Infrastructure for Open Media Workbench
//!
//! Provides an offline-first, shared model manager for:
//! - Curated, verified catalog management
//! - User-consented streaming downloads with bounded memory
//! - Cryptographic SHA-256 integrity verification
//! - Centralized local model storage and path protection
//! - Best-effort hardware capability detection
//! - Typed runtime contracts for future inference engines (Whisper, Piper, LLMs)

pub mod alignment;
pub mod catalog;
pub mod downloads;
pub mod errors;
pub mod hardware;
pub mod models;
pub mod runtime;
pub mod storage;
pub mod transcription;
pub mod verification;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::watch;

pub use catalog::{find_catalog_model, get_curated_model_catalog, is_trusted_download_url};
pub use errors::AiError;
pub use hardware::detect_hardware;
pub use models::*;
pub use runtime::{evaluate_model_compatibility, AiModelCompatibility, AiRuntime};
pub use storage::AiStorageManager;
pub use transcription::{
    audio::extract_audio_for_whisper,
    errors::TranscriptionError,
    models::{
        get_supported_whisper_languages, TranscriptionLanguage, TranscriptionProgressStage,
        TranscriptionRequest, TranscriptionResult, TranscriptionSegment, TranscriptionStatus,
        WhisperRuntimeCapabilities, TRANSCRIPTION_RESULT_SCHEMA_VERSION,
    },
    subtitle_conversion::{append_track_to_document, convert_transcription_result_to_track},
    whisper::{
        discover_whisper_runtime, parse_whisper_json_output, MockWhisperEngine,
        SystemWhisperCliEngine, WhisperEngine,
    },
    TranscriptionManager,
};
pub use verification::{compute_file_sha256, verify_file_integrity};

/// Type alias for thread-safe active download tracking map.
pub type ActiveDownloadMap =
    Arc<RwLock<HashMap<String, (watch::Sender<bool>, AiModelDownloadProgress)>>>;

/// Central manager orchestrating catalog discovery, downloads, storage, and integrity.
#[derive(Clone)]
pub struct AiModelManager {
    storage: AiStorageManager,
    active_downloads: ActiveDownloadMap,
}

impl AiModelManager {
    /// Creates a new `AiModelManager` rooted at the provided `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self, AiError> {
        let storage = AiStorageManager::new(base_dir)?;
        Ok(Self {
            storage,
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Accesses the underlying storage manager.
    pub fn storage(&self) -> &AiStorageManager {
        &self.storage
    }

    /// Lists all models in the curated catalog alongside their dynamic installation/download status.
    pub fn list_models(&self) -> Result<Vec<AiModelSummary>, AiError> {
        let catalog = get_curated_model_catalog();
        let mut summaries = Vec::with_capacity(catalog.len());

        for manifest in catalog {
            summaries.push(self.build_model_summary(manifest)?);
        }

        Ok(summaries)
    }

    /// Queries the dynamic status of a specific model by ID.
    pub fn get_model_status(&self, model_id: &str) -> Result<AiModelSummary, AiError> {
        let manifest = find_catalog_model(model_id)
            .ok_or_else(|| AiError::ModelNotFound(model_id.to_string()))?;
        self.build_model_summary(manifest)
    }

    fn build_model_summary(&self, manifest: AiModelManifest) -> Result<AiModelSummary, AiError> {
        let installation = self.storage.get_installation(&manifest.id)?;

        let download_progress = self
            .active_downloads
            .read()
            .ok()
            .and_then(|downloads| downloads.get(&manifest.id).map(|(_, p)| p.clone()));

        let status = if let Some(ref progress) = download_progress {
            match progress.state {
                AiDownloadState::Queued | AiDownloadState::Downloading => AiModelStatus::Installing,
                AiDownloadState::Verifying => AiModelStatus::Verifying,
                AiDownloadState::Failed => AiModelStatus::Failed,
                AiDownloadState::Cancelled => AiModelStatus::NotInstalled,
                AiDownloadState::Complete => AiModelStatus::Installed,
            }
        } else if let Some(ref inst) = installation {
            let artifact_path = self
                .storage
                .get_artifact_path(&manifest.id, &inst.artifact_filename);
            match artifact_path {
                Ok(path) if path.is_file() => AiModelStatus::Installed,
                _ => AiModelStatus::Corrupted,
            }
        } else {
            AiModelStatus::NotInstalled
        };

        // In STEP 18, runtime engines are planned for future steps.
        // Ready status requires BOTH an installed artifact and an active execution engine.
        let runtime_available = false;

        Ok(AiModelSummary {
            manifest,
            status,
            installation,
            download_progress,
            runtime_available,
        })
    }

    /// Initiates a verified background download for a catalog model with user consent.
    pub async fn start_download(&self, model_id: &str) -> Result<AiModelDownloadProgress, AiError> {
        let manifest = find_catalog_model(model_id)
            .ok_or_else(|| AiError::ModelNotFound(model_id.to_string()))?;

        if !manifest.installable || manifest.artifact_url.is_none() {
            return Err(AiError::ModelNotInstallable(model_id.to_string()));
        }

        if self.storage.get_installation(model_id)?.is_some() {
            return Err(AiError::ModelAlreadyInstalled(model_id.to_string()));
        }

        // Check if download is already running
        {
            let downloads = self.active_downloads.read().map_err(|_| {
                AiError::StorageError("Failed to acquire read lock on active downloads".into())
            })?;
            if let Some((_, progress)) = downloads.get(model_id) {
                if progress.state == AiDownloadState::Downloading
                    || progress.state == AiDownloadState::Queued
                {
                    return Err(AiError::DownloadInProgress(model_id.to_string()));
                }
            }
        }

        // Verify available disk space
        let hardware = detect_hardware(self.storage.base_dir());
        if let Some(available) = hardware.storage_available_bytes {
            // Require 10% safety margin above expected size
            let required_with_margin = manifest
                .expected_size_bytes
                .saturating_add(manifest.expected_size_bytes / 10);
            if available < required_with_margin {
                return Err(AiError::InsufficientDiskSpace {
                    required: required_with_margin,
                    available,
                });
            }
        }

        let job_id = uuid::Uuid::new_v4().to_string();
        let staging_path = self.storage.create_staging_path(model_id, &job_id)?;
        let (cancel_tx, cancel_rx) = watch::channel(false);

        let initial_progress = AiModelDownloadProgress {
            model_id: model_id.to_string(),
            bytes_downloaded: 0,
            total_bytes: manifest.expected_size_bytes,
            progress_pct: 0.0,
            state: AiDownloadState::Queued,
            error: None,
        };

        {
            let mut downloads = self.active_downloads.write().map_err(|_| {
                AiError::StorageError("Failed to acquire write lock on active downloads".into())
            })?;
            downloads.insert(model_id.to_string(), (cancel_tx, initial_progress.clone()));
        }

        // Spawn background task
        let manager_clone = self.clone();
        let manifest_clone = manifest.clone();
        let staging_path_clone = staging_path.clone();
        let model_id_owned = model_id.to_string();

        tokio::spawn(async move {
            let active_map = manager_clone.active_downloads.clone();
            let model_id_cb = model_id_owned.clone();

            let progress_cb = move |progress: AiModelDownloadProgress| {
                if let Ok(mut downloads) = active_map.write() {
                    if let Some(entry) = downloads.get_mut(&model_id_cb) {
                        entry.1 = progress;
                    }
                }
            };

            let download_res = downloads::download_model_artifact(
                &manifest_clone,
                &staging_path_clone,
                cancel_rx,
                progress_cb,
            )
            .await;

            match download_res {
                Ok((computed_sha256, actual_size)) => {
                    let installation = AiModelInstallation {
                        model_id: model_id_owned.clone(),
                        installed_at: chrono_like_timestamp(),
                        installed_version: manifest_clone.version.clone(),
                        artifact_filename: manifest_clone.artifact_filename.clone(),
                        actual_size_bytes: actual_size,
                        verified_sha256: computed_sha256,
                        in_use: false,
                    };

                    let _ = manager_clone
                        .storage
                        .commit_installation(&staging_path_clone, installation);

                    // Clean up active download
                    if let Ok(mut downloads) = manager_clone.active_downloads.write() {
                        downloads.remove(&model_id_owned);
                    }
                }
                Err(_e) => {
                    // Retain error state in active downloads for observer to inspect
                }
            }
        });

        Ok(initial_progress)
    }

    /// Returns the progress of an active or recent download job.
    pub fn get_download_progress(&self, model_id: &str) -> Option<AiModelDownloadProgress> {
        self.active_downloads
            .read()
            .ok()
            .and_then(|downloads| downloads.get(model_id).map(|(_, p)| p.clone()))
    }

    /// Cancels an in-progress download job.
    pub fn cancel_download(&self, model_id: &str) -> Result<(), AiError> {
        let mut downloads = self.active_downloads.write().map_err(|_| {
            AiError::StorageError("Failed to acquire write lock on active downloads".into())
        })?;

        if let Some((cancel_tx, _)) = downloads.get_mut(model_id) {
            let _ = cancel_tx.send(true);
            downloads.remove(model_id);
            Ok(())
        } else {
            Err(AiError::ModelNotFound(format!(
                "No active download found for {model_id}"
            )))
        }
    }

    /// Re-evaluates SHA-256 integrity of an installed model.
    pub fn verify_model(&self, model_id: &str) -> Result<bool, AiError> {
        let manifest = find_catalog_model(model_id)
            .ok_or_else(|| AiError::ModelNotFound(model_id.to_string()))?;
        let expected_sha256 = manifest
            .expected_sha256
            .as_deref()
            .ok_or_else(|| AiError::ModelNotInstallable(model_id.to_string()))?;

        self.storage
            .verify_installed_model(model_id, expected_sha256, manifest.expected_size_bytes)
    }

    /// Safely uninstalls a model and removes its files from shared storage.
    pub fn remove_model(&self, model_id: &str) -> Result<(), AiError> {
        self.storage.remove_model(model_id)
    }

    /// Reports current disk and cache usage of the managed AI storage repository.
    pub fn get_storage_usage(&self) -> Result<AiStorageUsage, AiError> {
        self.storage.compute_storage_usage()
    }

    /// Detects and returns current host hardware metrics.
    pub fn get_hardware_info(&self) -> AiHardwareInfo {
        detect_hardware(self.storage.base_dir())
    }
}

/// Fallback UTC timestamp string generator without pulling in large external chrono dependency.
fn chrono_like_timestamp() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", duration.as_secs(), duration.subsec_millis())
}
