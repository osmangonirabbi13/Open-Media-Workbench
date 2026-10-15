//! Asynchronous Text-to-Speech Job Manager.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::watch;
use uuid::Uuid;

use crate::ai::catalog::find_catalog_model;
use crate::ai::storage::AiStorageManager;
use crate::localization::subtitles::model::SubtitleTrack;

use super::audio::{concatenate_wav_files, validate_wav_file};
use super::engine::TtsEngine;
use super::errors::TtsError;
use super::models::{
    AiVoiceManifest, TtsAudioSegment, TtsProgressStage, TtsRequest, TtsResult, TtsSource,
    TtsStatus, TtsSynthesisInput,
};
use super::segments::{chunk_script_text, extract_cues_for_tts, SubtitleCueForTts};

pub type TextItemToSynthesize = (Option<String>, String, Option<(u64, u64)>);

struct JobState {
    status: TtsStatus,
    result: Option<TtsResult>,
    cancel_tx: watch::Sender<bool>,
    _staging_dir: PathBuf,
    _model_id: String,
}

/// Orchestrates TTS synthesis jobs, voice model locking, cancellation, and audio staging.
#[derive(Clone)]
pub struct TtsManager {
    storage: Arc<AiStorageManager>,
    engine: Arc<dyn TtsEngine>,
    staging_dir: PathBuf,
    jobs: Arc<RwLock<HashMap<String, JobState>>>,
}

impl TtsManager {
    pub fn new(storage: Arc<AiStorageManager>, engine: Arc<dyn TtsEngine>) -> Self {
        let staging_dir = storage.base_dir().join("tts_staging");
        let _ = fs::create_dir_all(&staging_dir);

        Self {
            storage,
            engine,
            staging_dir,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn engine(&self) -> &dyn TtsEngine {
        self.engine.as_ref()
    }

    pub fn storage(&self) -> &AiStorageManager {
        &self.storage
    }

    /// Lists curated TTS voices and their local installation / readiness status.
    pub fn list_voices(&self) -> Vec<AiVoiceManifest> {
        let mut voices = Vec::new();

        // 1. Piper Lessac Medium
        let lessac_installed = self
            .storage
            .get_installation("piper-en-lessac-medium")
            .ok()
            .flatten()
            .is_some();
        let runtime_ready = self.engine.verify_runtime().unwrap_or(false);

        voices.push(AiVoiceManifest {
            id: "piper-en-lessac-medium".to_string(),
            display_name: "English (US) - Lessac (Medium)".to_string(),
            language: "en-US".to_string(),
            locale: "en_US".to_string(),
            voice_family: "piper".to_string(),
            model_id: "piper-en-lessac-medium".to_string(),
            sample_rate: 22050,
            speaker_count: 1,
            is_installed: lessac_installed,
            runtime_ready,
            license: "Public Domain".to_string(),
            description: "High quality neural English voice suitable for narration and dialogue."
                .to_string(),
        });

        voices
    }

    /// Initiates an asynchronous offline speech synthesis job.
    pub fn start_synthesis(
        &self,
        request: TtsRequest,
        track_ref: Option<SubtitleTrack>,
    ) -> Result<String, TtsError> {
        request
            .settings
            .validate()
            .map_err(TtsError::InvalidVoiceSettings)?;

        let voices = self.list_voices();
        let voice = voices
            .iter()
            .find(|v| v.id == request.voice_id)
            .ok_or_else(|| {
                TtsError::MissingModel(format!("Voice '{}' not found", request.voice_id))
            })?;

        // Language compatibility check
        let req_lang_prefix = request
            .language
            .split('-')
            .next()
            .unwrap_or(&request.language);
        let voice_lang_prefix = voice.language.split('-').next().unwrap_or(&voice.language);
        if req_lang_prefix != voice_lang_prefix && request.language != "auto" {
            return Err(TtsError::LanguageIncompatible {
                requested: request.language.clone(),
                voice_id: voice.id.clone(),
                supported: vec![voice.language.clone()],
            });
        }

        // Check if model is installed in storage
        let manifest = find_catalog_model(&voice.model_id)
            .ok_or_else(|| TtsError::MissingModel(voice.model_id.clone()))?;

        let model_dir = self
            .storage
            .resolve_model_dir(&voice.model_id)
            .map_err(|e| TtsError::StorageError(e.to_string()))?;

        let model_path = model_dir.join(&manifest.artifact_filename);
        if !model_path.is_file() {
            return Err(TtsError::MissingModel(format!(
                "Voice model file not installed: {}",
                model_path.display()
            )));
        }

        let config_artifact = manifest
            .additional_artifacts
            .iter()
            .find(|a| a.role == "config")
            .ok_or_else(|| {
                TtsError::MissingArtifact(format!(
                    "No config artifact defined for voice {}",
                    voice.id
                ))
            })?;

        let config_path = model_dir.join(&config_artifact.filename);
        if !config_path.is_file() {
            return Err(TtsError::MissingArtifact(format!(
                "Voice config file not installed: {}",
                config_path.display()
            )));
        }

        // Acquire in-use lock
        if !self
            .storage
            .acquire_lock(&voice.model_id)
            .map_err(|e| TtsError::StorageError(e.to_string()))?
        {
            return Err(TtsError::ModelInUse(voice.model_id.clone()));
        }

        let job_id = format!("tts_{}", Uuid::new_v4().simple());
        let job_staging_dir = self.staging_dir.join(&job_id);
        fs::create_dir_all(&job_staging_dir).map_err(|e| {
            TtsError::StorageError(format!("Failed to create job staging dir: {e}"))
        })?;

        // Prepare text items
        let items: Vec<TextItemToSynthesize> = match &request.source {
            TtsSource::Script { text } => {
                let chunks = chunk_script_text(text);
                if chunks.is_empty() {
                    self.storage.release_lock(&voice.model_id);
                    return Err(TtsError::InvalidInput("Script text is empty".into()));
                }
                chunks.into_iter().map(|c| (None, c, None)).collect()
            }
            TtsSource::SubtitleTrack { track_id, cue_ids } => {
                let track = track_ref.ok_or_else(|| TtsError::TrackNotFound(track_id.clone()))?;
                let cues = extract_cues_for_tts(&track, cue_ids.as_deref());
                if cues.is_empty() {
                    self.storage.release_lock(&voice.model_id);
                    return Err(TtsError::InvalidInput(
                        "Subtitle track has no non-empty cues to synthesize".into(),
                    ));
                }
                cues.into_iter()
                    .map(|c: SubtitleCueForTts| {
                        (Some(c.cue_id), c.text, Some((c.start_us, c.end_us)))
                    })
                    .collect()
            }
        };

        let total_segments = items.len();
        let (cancel_tx, cancel_rx) = watch::channel(false);

        let initial_status = TtsStatus {
            job_id: job_id.clone(),
            stage: TtsProgressStage::Queued,
            completed_segments: 0,
            total_segments,
            progress_pct: 0.0,
            current_message: "Job queued".to_string(),
            error: None,
        };

        {
            let mut jobs = self.jobs.write().unwrap();
            jobs.insert(
                job_id.clone(),
                JobState {
                    status: initial_status,
                    result: None,
                    cancel_tx,
                    _staging_dir: job_staging_dir.clone(),
                    _model_id: voice.model_id.clone(),
                },
            );
        }

        // Spawn async worker
        let manager_clone = self.clone();
        let job_id_clone = job_id.clone();
        let voice_id = voice.id.clone();
        let model_id_owned = voice.model_id.clone();
        let source_type = match &request.source {
            TtsSource::Script { .. } => "script",
            TtsSource::SubtitleTrack { .. } => "subtitle",
        }
        .to_string();

        tokio::spawn(async move {
            let res = manager_clone
                .run_synthesis_loop(
                    &job_id_clone,
                    &source_type,
                    &voice_id,
                    &model_path,
                    &config_path,
                    request.settings,
                    items,
                    job_staging_dir,
                    cancel_rx,
                )
                .await;

            // Release model lock in all cases
            manager_clone.storage.release_lock(&model_id_owned);

            if let Err(err) = res {
                let mut jobs = manager_clone.jobs.write().unwrap();
                if let Some(job) = jobs.get_mut(&job_id_clone) {
                    if err == TtsError::Cancellation {
                        job.status.stage = TtsProgressStage::Cancelled;
                        job.status.current_message = "Synthesis cancelled by user".into();
                    } else {
                        job.status.stage = TtsProgressStage::Failed;
                        job.status.current_message = format!("Synthesis failed: {err}");
                        job.status.error = Some(err.to_string());
                    }
                }
            }
        });

        Ok(job_id)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_synthesis_loop(
        &self,
        job_id: &str,
        source_type: &str,
        voice_id: &str,
        model_path: &Path,
        config_path: &Path,
        settings: super::models::TtsVoiceSettings,
        items: Vec<TextItemToSynthesize>,
        job_staging_dir: PathBuf,
        cancel_rx: watch::Receiver<bool>,
    ) -> Result<(), TtsError> {
        let total_segments = items.len();

        self.update_stage(
            job_id,
            TtsProgressStage::PreparingText,
            0,
            total_segments,
            "Preparing text segments...",
        );

        let mut segments: Vec<TtsAudioSegment> = Vec::new();
        let mut total_duration_us: u64 = 0;
        let mut warnings: Vec<String> = Vec::new();

        self.update_stage(
            job_id,
            TtsProgressStage::Synthesizing,
            0,
            total_segments,
            "Synthesizing speech...",
        );

        for (idx, (cue_id_opt, text, timing_opt)) in items.into_iter().enumerate() {
            if *cancel_rx.borrow() {
                return Err(TtsError::Cancellation);
            }

            let segment_id = format!("seg_{idx}_{}", Uuid::new_v4().simple());
            let segment_wav_path = job_staging_dir.join(format!("{segment_id}.wav"));

            let input = TtsSynthesisInput {
                text: text.clone(),
                voice_id: voice_id.to_string(),
                model_path: model_path.to_path_buf(),
                config_path: config_path.to_path_buf(),
                settings: settings.clone(),
            };

            let audio_meta = self
                .engine
                .synthesize(&input, &segment_wav_path, Some(&cancel_rx))?;

            let mut segment_warnings = Vec::new();
            let mut diff_us = None;
            let mut cue_start_us = None;
            let mut cue_end_us = None;

            if let Some((start, end)) = timing_opt {
                cue_start_us = Some(start);
                cue_end_us = Some(end);
                let cue_duration = end.saturating_sub(start);
                let diff = (audio_meta.duration_us as i64) - (cue_duration as i64);
                diff_us = Some(diff);

                if diff.abs() > 500_000 {
                    let sec_diff = (diff as f64) / 1_000_000.0;
                    let warn_msg = format!(
                        "Audio duration differs from subtitle cue by {:.2}s",
                        sec_diff
                    );
                    segment_warnings.push(warn_msg.clone());
                    warnings.push(format!(
                        "Cue {}: {}",
                        cue_id_opt.as_deref().unwrap_or("?"),
                        warn_msg
                    ));
                }
            }

            total_duration_us += audio_meta.duration_us;

            segments.push(TtsAudioSegment {
                id: segment_id,
                source_cue_id: cue_id_opt,
                original_text: text,
                voice_id: voice_id.to_string(),
                sample_rate: audio_meta.sample_rate,
                channel_count: audio_meta.channel_count,
                sample_count: audio_meta.sample_count,
                duration_us: audio_meta.duration_us,
                cue_start_us,
                cue_end_us,
                timing_difference_us: diff_us,
                staged_audio_path: segment_wav_path.to_string_lossy().to_string(),
                warnings: segment_warnings,
            });

            let completed = idx + 1;
            let msg = format!("Synthesized {completed} of {total_segments} segments");
            self.update_stage(
                job_id,
                TtsProgressStage::Synthesizing,
                completed,
                total_segments,
                &msg,
            );
        }

        self.update_stage(
            job_id,
            TtsProgressStage::Validating,
            total_segments,
            total_segments,
            "Validating generated speech...",
        );

        let final_result = TtsResult {
            schema_version: 1,
            job_id: job_id.to_string(),
            source_type: source_type.to_string(),
            voice_id: voice_id.to_string(),
            runtime: "Piper".to_string(),
            segments,
            total_duration_us,
            created_at_epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            warnings,
        };

        {
            let mut jobs = self.jobs.write().unwrap();
            if let Some(job) = jobs.get_mut(job_id) {
                job.status.stage = TtsProgressStage::Completed;
                job.status.completed_segments = total_segments;
                job.status.progress_pct = 100.0;
                job.status.current_message = "Synthesis completed successfully".into();
                job.result = Some(final_result);
            }
        }

        Ok(())
    }

    fn update_stage(
        &self,
        job_id: &str,
        stage: TtsProgressStage,
        completed: usize,
        total: usize,
        message: &str,
    ) {
        let mut jobs = self.jobs.write().unwrap();
        if let Some(job) = jobs.get_mut(job_id) {
            job.status.stage = stage;
            job.status.completed_segments = completed;
            job.status.total_segments = total;
            job.status.progress_pct = if total > 0 {
                (completed as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            job.status.current_message = message.to_string();
        }
    }

    /// Queries the status of an active or finished synthesis job.
    pub fn get_status(&self, job_id: &str) -> Option<TtsStatus> {
        self.jobs
            .read()
            .unwrap()
            .get(job_id)
            .map(|j| j.status.clone())
    }

    /// Cancels a running synthesis job.
    pub fn cancel_job(&self, job_id: &str) -> Result<bool, TtsError> {
        let jobs = self.jobs.read().unwrap();
        if let Some(job) = jobs.get(job_id) {
            let _ = job.cancel_tx.send(true);
            Ok(true)
        } else {
            Err(TtsError::JobNotFound(job_id.to_string()))
        }
    }

    /// Retrieves the completed result of a synthesis job.
    pub fn get_result(&self, job_id: &str) -> Result<TtsResult, TtsError> {
        let jobs = self.jobs.read().unwrap();
        let job = jobs
            .get(job_id)
            .ok_or_else(|| TtsError::JobNotFound(job_id.to_string()))?;

        job.result
            .clone()
            .ok_or_else(|| TtsError::ProcessFailed("Synthesis result not yet ready".into()))
    }

    /// Exports generated speech to a user-selected WAV destination file with no-overwrite safety.
    pub fn export_audio(
        &self,
        job_id: &str,
        segment_id: Option<&str>,
        destination_path: &Path,
    ) -> Result<PathBuf, TtsError> {
        if destination_path.exists() {
            return Err(TtsError::StorageError(format!(
                "Destination file already exists: {}",
                destination_path.display()
            )));
        }

        let result = self.get_result(job_id)?;

        if let Some(seg_id) = segment_id {
            // Export single segment
            let segment = result
                .segments
                .iter()
                .find(|s| s.id == seg_id)
                .ok_or_else(|| {
                    TtsError::AudioValidationFailed(format!("Segment '{seg_id}' not found"))
                })?;

            let staged_path = Path::new(&segment.staged_audio_path);
            if !staged_path.exists() {
                return Err(TtsError::AudioValidationFailed(
                    "Staged audio segment file is missing".into(),
                ));
            }

            fs::copy(staged_path, destination_path)
                .map_err(|e| TtsError::StorageError(format!("Failed to copy WAV file: {e}")))?;

            validate_wav_file(destination_path)?;
            Ok(destination_path.to_path_buf())
        } else {
            // Export all segments as concatenated sequential WAV
            let segment_paths: Vec<PathBuf> = result
                .segments
                .iter()
                .map(|s| PathBuf::from(&s.staged_audio_path))
                .collect();

            concatenate_wav_files(&segment_paths, destination_path)?;
            Ok(destination_path.to_path_buf())
        }
    }
}
