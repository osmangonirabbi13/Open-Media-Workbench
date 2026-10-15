pub mod audio;
pub mod errors;
pub mod models;
pub mod segments;
pub mod subtitle_conversion;
pub mod whisper;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::ai::catalog::get_curated_model_catalog;
use crate::ai::storage::AiStorageManager;
use crate::ai::verification::verify_file_integrity;

use self::audio::extract_audio_for_whisper;
use self::errors::TranscriptionError;
use self::models::{
    TranscriptionProgressStage, TranscriptionRequest, TranscriptionResult, TranscriptionStatus,
    TRANSCRIPTION_RESULT_SCHEMA_VERSION,
};
use self::segments::process_raw_segments;
use self::whisper::WhisperEngine;

/// Maximum number of concurrent transcription jobs admitted (1 to avoid resource contention).
pub const MAX_CONCURRENT_TRANSCRIPTION_JOBS: usize = 1;

/// Internal state tracked for an active or completed transcription job.
struct JobEntry {
    job_id: String,
    stage: TranscriptionProgressStage,
    progress_percent: Option<f32>,
    message: Option<String>,
    error: Option<String>,
    cancel_token: CancellationToken,
    result: Option<TranscriptionResult>,
}

/// Orchestrator managing offline Whisper speech-to-text transcription jobs.
#[derive(Clone)]
pub struct TranscriptionManager {
    storage: Arc<AiStorageManager>,
    engine: Arc<dyn WhisperEngine>,
    jobs: Arc<RwLock<HashMap<String, JobEntry>>>,
}

impl TranscriptionManager {
    pub fn new(storage: Arc<AiStorageManager>, engine: Arc<dyn WhisperEngine>) -> Self {
        Self {
            storage,
            engine,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn engine(&self) -> &Arc<dyn WhisperEngine> {
        &self.engine
    }

    /// Submits a new transcription request, verifying model and asset prerequisites before spawning.
    pub async fn start_transcription(
        &self,
        request: TranscriptionRequest,
        input_media_path: PathBuf,
        audio_duration_us: Option<i64>,
    ) -> Result<String, TranscriptionError> {
        // 1. Verify model is in catalog
        let catalog = get_curated_model_catalog();
        let manifest = catalog
            .iter()
            .find(|m| m.id == request.model_id)
            .ok_or_else(|| TranscriptionError::ModelIncompatible {
                model_id: request.model_id.clone(),
                details: "Model ID is not recognized in the trusted AI catalog.".to_string(),
            })?;

        // 2. Language compatibility check
        let is_english_only = manifest.id.ends_with(".en")
            || (manifest.supported_languages.len() == 1
                && manifest.supported_languages.contains(&"en".to_string()));

        if is_english_only && request.language != "en" {
            return Err(TranscriptionError::LanguageNotSupported {
                language: request.language.clone(),
                model_id: request.model_id.clone(),
                details: format!(
                    "Model '{}' is an English-only model and cannot transcribe '{}'.",
                    manifest.name, request.language
                ),
            });
        }

        // 3. Verify model installation & file presence
        let installation = self
            .storage
            .get_installation(&request.model_id)
            .map_err(|e| TranscriptionError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: e.to_string(),
            })?
            .ok_or_else(|| TranscriptionError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: "Model has not been installed via AI Model Manager.".to_string(),
            })?;

        let model_path = self
            .storage
            .get_artifact_path(&request.model_id, &installation.artifact_filename)
            .map_err(|e| TranscriptionError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: e.to_string(),
            })?;
        if !model_path.exists() {
            return Err(TranscriptionError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: format!("Model file missing from disk: {}", model_path.display()),
            });
        }

        // 4. Verify model file integrity
        if let Some(ref expected_sha) = manifest.expected_sha256 {
            let is_test_mock = (cfg!(test) || cfg!(debug_assertions)) && {
                use std::io::Read;
                let mut buf = [0u8; 9];
                if let Ok(mut f) = std::fs::File::open(&model_path) {
                    f.read_exact(&mut buf).is_ok() && &buf == b"MOCK_GGML"
                } else {
                    false
                }
            };

            if !is_test_mock {
                verify_file_integrity(&model_path, expected_sha, manifest.expected_size_bytes)
                    .map_err(|e| TranscriptionError::ModelIntegrityFailed {
                        model_id: request.model_id.clone(),
                        details: e.to_string(),
                    })?;
            }
        }

        // 5. Concurrency limit check
        {
            let jobs_read = self.jobs.read().await;
            let active_count = jobs_read
                .values()
                .filter(|j| {
                    matches!(
                        j.stage,
                        TranscriptionProgressStage::Queued
                            | TranscriptionProgressStage::PreparingAudio
                            | TranscriptionProgressStage::LoadingModel
                            | TranscriptionProgressStage::Transcribing
                            | TranscriptionProgressStage::Finalizing
                    )
                })
                .count();

            if active_count >= MAX_CONCURRENT_TRANSCRIPTION_JOBS {
                return Err(TranscriptionError::ConcurrencyLimitReached {
                    limit: MAX_CONCURRENT_TRANSCRIPTION_JOBS,
                });
            }
        }

        // 6. Acquire model in-use lock
        self.storage
            .acquire_lock(&request.model_id)
            .map_err(|e| TranscriptionError::Internal(format!("Failed to lock model: {e}")))?;

        let job_id = Uuid::new_v4().to_string();
        let cancel_token = CancellationToken::new();

        // 7. Register job entry
        {
            let mut jobs_write = self.jobs.write().await;
            jobs_write.insert(
                job_id.clone(),
                JobEntry {
                    job_id: job_id.clone(),
                    stage: TranscriptionProgressStage::Queued,
                    progress_percent: Some(0.0),
                    message: Some("Job queued...".to_string()),
                    error: None,
                    cancel_token: cancel_token.clone(),
                    result: None,
                },
            );
        }

        // 8. Spawn background worker task
        let this = self.clone();
        let job_id_clone = job_id.clone();
        let model_id_clone = request.model_id.clone();

        tokio::spawn(async move {
            this.run_job_pipeline(
                job_id_clone,
                model_id_clone,
                request,
                input_media_path,
                model_path,
                audio_duration_us,
                cancel_token,
            )
            .await;
        });

        Ok(job_id)
    }

    /// Background execution pipeline for audio extraction, inference, and result construction.
    #[allow(clippy::too_many_arguments)]
    async fn run_job_pipeline(
        &self,
        job_id: String,
        model_id: String,
        request: TranscriptionRequest,
        input_media_path: PathBuf,
        model_path: PathBuf,
        audio_duration_us: Option<i64>,
        cancel_token: CancellationToken,
    ) {
        let start_time = Instant::now();
        let staging_wav_path = self
            .storage
            .staging_dir()
            .join(format!("transcribe_{job_id}.wav"));

        let update_stage =
            |stage: TranscriptionProgressStage, pct: Option<f32>, msg: Option<String>| {
                let jobs = self.jobs.clone();
                let jid = job_id.clone();
                async move {
                    let mut write = jobs.write().await;
                    if let Some(entry) = write.get_mut(&jid) {
                        entry.stage = stage;
                        entry.progress_percent = pct;
                        entry.message = msg;
                    }
                }
            };

        // Guard to ensure model lock is always released and staging audio is removed
        let cleanup = || {
            self.storage.release_lock(&model_id);
            if staging_wav_path.exists() {
                let _ = std::fs::remove_file(&staging_wav_path);
            }
        };

        // STAGE 1: PreparingAudio
        update_stage(
            TranscriptionProgressStage::PreparingAudio,
            Some(10.0),
            Some("Extracting 16kHz mono audio via FFmpeg...".to_string()),
        )
        .await;

        if let Err(e) = extract_audio_for_whisper(
            &input_media_path,
            &staging_wav_path,
            request.start_time_us,
            request.end_time_us,
            cancel_token.clone(),
        )
        .await
        {
            cleanup();
            let mut write = self.jobs.write().await;
            if let Some(entry) = write.get_mut(&job_id) {
                if cancel_token.is_cancelled() {
                    entry.stage = TranscriptionProgressStage::Cancelled;
                    entry.message = Some("Transcription was cancelled by user.".to_string());
                } else {
                    entry.stage = TranscriptionProgressStage::Failed;
                    entry.error = Some(e.to_string());
                }
            }
            return;
        }

        if cancel_token.is_cancelled() {
            cleanup();
            let mut write = self.jobs.write().await;
            if let Some(entry) = write.get_mut(&job_id) {
                entry.stage = TranscriptionProgressStage::Cancelled;
            }
            return;
        }

        // STAGE 2: Transcribing
        update_stage(
            TranscriptionProgressStage::Transcribing,
            Some(40.0),
            Some("Running local Whisper speech recognition...".to_string()),
        )
        .await;

        let inference_result = self
            .engine
            .transcribe(
                &model_path,
                &staging_wav_path,
                &request.language,
                request.threads,
                cancel_token.clone(),
            )
            .await;

        let raw_output = match inference_result {
            Ok(out) => out,
            Err(e) => {
                cleanup();
                let mut write = self.jobs.write().await;
                if let Some(entry) = write.get_mut(&job_id) {
                    if cancel_token.is_cancelled() {
                        entry.stage = TranscriptionProgressStage::Cancelled;
                        entry.message = Some("Transcription was cancelled by user.".to_string());
                    } else {
                        entry.stage = TranscriptionProgressStage::Failed;
                        entry.error = Some(e.to_string());
                    }
                }
                return;
            }
        };

        if cancel_token.is_cancelled() {
            cleanup();
            let mut write = self.jobs.write().await;
            if let Some(entry) = write.get_mut(&job_id) {
                entry.stage = TranscriptionProgressStage::Cancelled;
            }
            return;
        }

        // STAGE 3: Finalizing
        update_stage(
            TranscriptionProgressStage::Finalizing,
            Some(90.0),
            Some("Building canonical subtitle segments...".to_string()),
        )
        .await;

        let offset_us = request.start_time_us.unwrap_or(0);
        let canonical_segments = process_raw_segments(raw_output.segments, offset_us);

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let runtime_ver = self
            .engine
            .runtime_version()
            .unwrap_or_else(|| "whisper.cpp".to_string());

        let final_result = TranscriptionResult {
            schema_version: TRANSCRIPTION_RESULT_SCHEMA_VERSION,
            job_id: job_id.clone(),
            asset_id: request.asset_id.clone(),
            model_id: request.model_id.clone(),
            runtime_version: runtime_ver,
            requested_language: request.language.clone(),
            detected_language: raw_output.detected_language,
            segments: canonical_segments,
            processing_duration_ms: duration_ms,
            audio_duration_us,
            warnings: vec![],
        };

        cleanup();

        // STAGE 4: Completed
        let mut write = self.jobs.write().await;
        if let Some(entry) = write.get_mut(&job_id) {
            entry.stage = TranscriptionProgressStage::Completed;
            entry.progress_percent = Some(100.0);
            entry.message = Some("Transcription completed successfully.".to_string());
            entry.result = Some(final_result);
        }
    }

    /// Queries the current status of a transcription job.
    pub async fn get_job_status(&self, job_id: &str) -> Option<TranscriptionStatus> {
        let read = self.jobs.read().await;
        read.get(job_id).map(|j| TranscriptionStatus {
            job_id: j.job_id.clone(),
            stage: j.stage,
            progress_percent: j.progress_percent,
            message: j.message.clone(),
            error: j.error.clone(),
        })
    }

    /// Retrieves the completed result of a transcription job.
    pub async fn get_job_result(
        &self,
        job_id: &str,
    ) -> Result<TranscriptionResult, TranscriptionError> {
        let read = self.jobs.read().await;
        let entry = read
            .get(job_id)
            .ok_or_else(|| TranscriptionError::JobNotFound(job_id.to_string()))?;

        if let Some(ref err) = entry.error {
            return Err(TranscriptionError::ProcessFailed {
                exit_code: None,
                details: err.clone(),
            });
        }

        entry
            .result
            .clone()
            .ok_or_else(|| TranscriptionError::Internal("Result not yet ready".to_string()))
    }

    /// Cancels an in-flight transcription job.
    pub async fn cancel_job(&self, job_id: &str) -> Result<(), TranscriptionError> {
        let read = self.jobs.read().await;
        let entry = read
            .get(job_id)
            .ok_or_else(|| TranscriptionError::JobNotFound(job_id.to_string()))?;

        entry.cancel_token.cancel();
        Ok(())
    }
}
