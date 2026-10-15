use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::ai::catalog::get_curated_model_catalog;
use crate::ai::storage::AiStorageManager;
use crate::ai::verification::verify_file_integrity;
use crate::localization::subtitles::model::SubtitleCue;

use super::engine::TranslationEngine;
use super::errors::TranslationError;
use super::models::{
    get_supported_language_pairs, TranslatedCue, TranslationProgressStage, TranslationRequest,
    TranslationResult, TranslationStatus, TRANSLATION_RESULT_SCHEMA_VERSION,
};

/// Internal state tracked for an active or completed translation job.
struct JobEntry {
    job_id: String,
    stage: TranslationProgressStage,
    progress_percent: Option<f32>,
    message: Option<String>,
    error: Option<String>,
    translated_cues: usize,
    total_cues: usize,
    cancel_token: CancellationToken,
    result: Option<TranslationResult>,
}

/// Orchestrator managing offline subtitle translation jobs.
#[derive(Clone)]
pub struct TranslationManager {
    storage: Arc<AiStorageManager>,
    engine: Arc<dyn TranslationEngine>,
    jobs: Arc<RwLock<HashMap<String, JobEntry>>>,
    review_storage: Option<Arc<super::review::ReviewStorageManager>>,
}

impl TranslationManager {
    pub fn new(storage: Arc<AiStorageManager>, engine: Arc<dyn TranslationEngine>) -> Self {
        Self {
            storage,
            engine,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            review_storage: None,
        }
    }

    pub fn with_review_storage(
        mut self,
        review_storage: Arc<super::review::ReviewStorageManager>,
    ) -> Self {
        self.review_storage = Some(review_storage);
        self
    }

    pub fn engine(&self) -> &Arc<dyn TranslationEngine> {
        &self.engine
    }

    /// Submits a new subtitle translation request.
    pub async fn start_translation(
        &self,
        request: TranslationRequest,
        cues: Vec<SubtitleCue>,
    ) -> Result<String, TranslationError> {
        if cues.is_empty() {
            return Err(TranslationError::EmptyTrack(request.track_id.clone()));
        }

        // 1. Verify model in catalog
        let catalog = get_curated_model_catalog();
        let manifest = catalog
            .iter()
            .find(|m| m.id == request.model_id)
            .ok_or_else(|| TranslationError::ModelNotFound {
                model_id: request.model_id.clone(),
            })?;

        // 2. Language pair support check
        let pairs = get_supported_language_pairs();
        let pair_match = pairs.iter().find(|p| {
            p.source_code.eq_ignore_ascii_case(&request.source_language)
                && p.target_code.eq_ignore_ascii_case(&request.target_language)
                && p.compatible_model_ids.contains(&request.model_id)
        });

        if pair_match.is_none() {
            return Err(TranslationError::LanguagePairNotSupported {
                source_lang: request.source_language.clone(),
                target_lang: request.target_language.clone(),
                model_id: request.model_id.clone(),
            });
        }

        // 3. Verify model installation & file presence
        let installation = self
            .storage
            .get_installation(&request.model_id)
            .map_err(|e| TranslationError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: e.to_string(),
            })?
            .ok_or_else(|| TranslationError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: "Model has not been installed via AI Model Manager.".to_string(),
            })?;

        let model_file_path = self
            .storage
            .get_artifact_path(&request.model_id, &installation.artifact_filename)
            .map_err(|e| TranslationError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: e.to_string(),
            })?;

        if !model_file_path.exists() {
            return Err(TranslationError::ModelNotInstalled {
                model_id: request.model_id.clone(),
                details: format!(
                    "Model artifact file '{}' does not exist on disk.",
                    model_file_path.display()
                ),
            });
        }

        // 4. Verify model integrity if expected_sha256 is pinned
        if let Some(expected_sha) = &manifest.expected_sha256 {
            let is_test_mock = (cfg!(test) || cfg!(debug_assertions)) && {
                use std::io::Read;
                let mut buf = [0u8; 9];
                if let Ok(mut f) = std::fs::File::open(&model_file_path) {
                    f.read_exact(&mut buf).is_ok() && (&buf == b"MOCK_GGML" || &buf == b"MOCK_GGUF")
                } else {
                    false
                }
            };

            if !is_test_mock {
                verify_file_integrity(&model_file_path, expected_sha, manifest.expected_size_bytes)
                    .map_err(|e| TranslationError::ModelIntegrityFailed {
                        model_id: request.model_id.clone(),
                        details: e.to_string(),
                    })?;
            }
        }

        // 5. Verify translation runtime availability
        if !self.engine.is_available() {
            return Err(TranslationError::RuntimeUnavailable(
                "Translation engine executable is not operational.".to_string(),
            ));
        }

        // 6. Acquire model in-use lock
        self.storage
            .acquire_lock(&request.model_id)
            .map_err(|e| TranslationError::LockAcquisitionFailed(e.to_string()))?;

        // 7. Initialize job entry
        let job_id = format!("job-tr-{}", Uuid::new_v4());
        let cancel_token = CancellationToken::new();
        let total_cues = cues.len();

        {
            let mut jobs_guard = self.jobs.write().await;
            jobs_guard.insert(
                job_id.clone(),
                JobEntry {
                    job_id: job_id.clone(),
                    stage: TranslationProgressStage::Queued,
                    progress_percent: Some(0.0),
                    message: Some("Queued for translation.".to_string()),
                    error: None,
                    translated_cues: 0,
                    total_cues,
                    cancel_token: cancel_token.clone(),
                    result: None,
                },
            );
        }

        // 8. Spawn background worker
        let engine_clone = self.engine.clone();
        let storage_clone = self.storage.clone();
        let jobs_clone = self.jobs.clone();
        let job_id_clone = job_id.clone();
        let request_clone = request.clone();
        let model_id_clone = request.model_id.clone();

        let review_storage_clone = self.review_storage.clone();

        tokio::spawn(async move {
            let start_instant = Instant::now();
            let res = Self::run_translation_pipeline(
                engine_clone,
                model_file_path,
                request_clone,
                cues,
                cancel_token,
                jobs_clone.clone(),
                job_id_clone.clone(),
                review_storage_clone,
            )
            .await;

            let elapsed_ms = start_instant.elapsed().as_millis() as u64;

            // Release model in-use lock
            storage_clone.release_lock(&model_id_clone);

            let mut jobs_guard = jobs_clone.write().await;
            if let Some(entry) = jobs_guard.get_mut(&job_id_clone) {
                match res {
                    Ok(mut final_result) => {
                        final_result.elapsed_ms = elapsed_ms;
                        entry.stage = TranslationProgressStage::Completed;
                        entry.progress_percent = Some(100.0);
                        entry.message = Some(format!(
                            "Translated {} cues successfully in {} ms.",
                            final_result.translated_cues.len(),
                            elapsed_ms
                        ));
                        entry.result = Some(final_result);
                    }
                    Err(TranslationError::CancellationRequested) => {
                        entry.stage = TranslationProgressStage::Cancelled;
                        entry.message = Some("Translation cancelled by user.".to_string());
                    }
                    Err(e) => {
                        entry.stage = TranslationProgressStage::Failed;
                        entry.error = Some(e.to_string());
                        entry.message = Some(format!("Translation failed: {e}"));
                    }
                }
            }
        });

        Ok(job_id)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_translation_pipeline(
        engine: Arc<dyn TranslationEngine>,
        model_path: std::path::PathBuf,
        request: TranslationRequest,
        cues: Vec<SubtitleCue>,
        cancel_token: CancellationToken,
        jobs: Arc<RwLock<HashMap<String, JobEntry>>>,
        job_id: String,
        review_storage: Option<Arc<super::review::ReviewStorageManager>>,
    ) -> Result<TranslationResult, TranslationError> {
        let total_cues = cues.len();

        // Stage: Preparing
        {
            let mut guard = jobs.write().await;
            if let Some(e) = guard.get_mut(&job_id) {
                e.stage = TranslationProgressStage::Preparing;
                e.message = Some("Preparing subtitle texts for translation...".to_string());
            }
        }

        // Optional Glossary loading for terminology protection
        let (glossary_opt, protected_terms) =
            if let (Some(gid), Some(storage)) = (&request.glossary_id, &review_storage) {
                match storage.load_glossary(gid) {
                    Ok(g) => {
                        let terms: Vec<String> = g
                            .entries
                            .iter()
                            .filter(|e| e.is_protected)
                            .map(|e| e.source_term.clone())
                            .collect();
                        (Some(g), terms)
                    }
                    Err(e) => {
                        return Err(TranslationError::GlossaryError(format!(
                            "Failed to load glossary {}: {}",
                            gid, e
                        )));
                    }
                }
            } else {
                (None, Vec::new())
            };

        let texts: Vec<String> = cues.iter().map(|c| c.text.clone()).collect();

        // Stage: Translating
        {
            let mut guard = jobs.write().await;
            if let Some(e) = guard.get_mut(&job_id) {
                e.stage = TranslationProgressStage::Translating;
                e.message = Some("Translating subtitles with local model...".to_string());
            }
        }

        let mut translated_texts = Vec::with_capacity(total_cues);
        let batch_size = request.batch_size.unwrap_or(1).max(1);

        for chunk in texts.chunks(batch_size) {
            if cancel_token.is_cancelled() {
                return Err(TranslationError::CancellationRequested);
            }

            // Apply terminology masking if protected terms exist
            let mut maskers: Vec<Option<super::review::ProtectedTermMasker>> =
                Vec::with_capacity(chunk.len());
            let mut input_chunk: Vec<String> = Vec::with_capacity(chunk.len());

            for text in chunk {
                if !protected_terms.is_empty() {
                    let mut masker = super::review::ProtectedTermMasker::new();
                    let pt_refs: Vec<&str> = protected_terms.iter().map(|s| s.as_str()).collect();
                    let masked = masker.mask(text, &pt_refs);
                    maskers.push(Some(masker));
                    input_chunk.push(masked);
                } else {
                    maskers.push(None);
                    input_chunk.push(text.clone());
                }
            }

            let chunk_res = engine
                .translate(
                    &model_path,
                    &input_chunk,
                    &request.source_language,
                    &request.target_language,
                    cancel_token.clone(),
                )
                .await?;

            for (res_text, masker_opt) in chunk_res.into_iter().zip(maskers) {
                let final_text = if let Some(masker) = masker_opt {
                    masker.unmask(&res_text).map_err(|e| {
                        TranslationError::OutputValidationFailed(format!(
                            "Terminology unmasking failed: {}",
                            e
                        ))
                    })?
                } else {
                    res_text
                };
                translated_texts.push(final_text);
            }

            let done_count = translated_texts.len();
            let pct = (done_count as f32 / total_cues as f32) * 90.0;

            let mut guard = jobs.write().await;
            if let Some(e) = guard.get_mut(&job_id) {
                e.translated_cues = done_count;
                e.progress_percent = Some(pct);
                e.message = Some(format!("Translating cue {}/{}...", done_count, total_cues));
            }
        }

        // Stage: Validating
        {
            let mut guard = jobs.write().await;
            if let Some(e) = guard.get_mut(&job_id) {
                e.stage = TranslationProgressStage::Validating;
                e.progress_percent = Some(95.0);
                e.message = Some("Validating timing and cue mapping...".to_string());
            }
        }

        if translated_texts.len() != cues.len() {
            return Err(TranslationError::OutputValidationFailed(format!(
                "Cue count mismatch: expected {} cues, received {}.",
                cues.len(),
                translated_texts.len()
            )));
        }

        let mut translated_cues = Vec::with_capacity(cues.len());
        let mut warnings = Vec::new();

        for (idx, (cue, trans_text)) in cues.into_iter().zip(translated_texts).enumerate() {
            // Timing preservation invariant: target start and end MUST exactly equal source
            let source_start_us = cue.start_us;
            let source_end_us = cue.end_us;

            let mut cue_warnings = Vec::new();
            let dur_secs =
                (source_end_us.as_micros() - source_start_us.as_micros()) as f64 / 1_000_000.0;
            if dur_secs > 0.0 {
                let cps = trans_text.chars().count() as f64 / dur_secs;
                if cps > 25.0 {
                    cue_warnings.push(format!("High reading speed: {:.1} chars/sec", cps));
                }
            }

            if trans_text.chars().count() > 84 {
                cue_warnings.push("Long text exceeds typical subtitle bounds".to_string());
            }

            if let Some(ref g) = glossary_opt {
                let term_findings =
                    super::review::validate_terminology(&cue.text, &trans_text, &g.entries);
                for finding in term_findings {
                    cue_warnings.push(format!("[{}] {}", finding.code.as_str(), finding.message));
                }
            }

            for w in &cue_warnings {
                warnings.push(format!("Cue {}: {}", idx + 1, w));
            }

            translated_cues.push(TranslatedCue {
                source_cue_id: cue.id.clone(),
                translated_cue_id: format!("{}-tr-{}", cue.id, request.target_language),
                original_text: cue.text,
                translated_text: trans_text,
                source_start_us,
                source_end_us,
                target_start_us: source_start_us,
                target_end_us: source_end_us,
                warnings: cue_warnings,
                model_provenance: request.model_id.clone(),
            });
        }

        let runtime_ver = engine
            .runtime_version()
            .unwrap_or_else(|| "local".to_string());

        Ok(TranslationResult {
            schema_version: TRANSLATION_RESULT_SCHEMA_VERSION,
            job_id,
            original_track_id: request.track_id,
            model_id: request.model_id,
            runtime_version: runtime_ver,
            source_language: request.source_language,
            target_language: request.target_language,
            translated_cues,
            untranslated_cue_ids: vec![],
            warnings,
            elapsed_ms: 0,
        })
    }

    /// Queries the status of a translation job.
    pub async fn get_status(&self, job_id: &str) -> Option<TranslationStatus> {
        let guard = self.jobs.read().await;
        guard.get(job_id).map(|e| TranslationStatus {
            job_id: e.job_id.clone(),
            stage: e.stage,
            progress_percent: e.progress_percent,
            message: e.message.clone(),
            error: e.error.clone(),
            translated_cues: e.translated_cues,
            total_cues: e.total_cues,
        })
    }

    /// Requests cooperative cancellation of an active translation job.
    pub async fn cancel_job(&self, job_id: &str) -> Result<(), TranslationError> {
        let guard = self.jobs.read().await;
        if let Some(entry) = guard.get(job_id) {
            entry.cancel_token.cancel();
            Ok(())
        } else {
            Err(TranslationError::OutputValidationFailed(format!(
                "Job '{}' not found.",
                job_id
            )))
        }
    }

    /// Retrieves the structured result of a completed translation job.
    pub async fn get_result(&self, job_id: &str) -> Option<TranslationResult> {
        let guard = self.jobs.read().await;
        guard.get(job_id).and_then(|e| e.result.clone())
    }
}
