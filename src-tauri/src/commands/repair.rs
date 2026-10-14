use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use opentake_core::media::authorization::SourceProvenance;
use opentake_core::media::compatibility::CompatibilityStatus;
use opentake_core::media::identity::MediaAsset;
use opentake_core::repair::{
    analyze_reference_compatibility as analyze_ref_compat, discover_reconstruction_capabilities,
    finalize_recovered_file, generate_recovery_html_report, generate_recovery_json_report,
    run_diagnostics, run_reconstruction_pipeline, run_recovery_pipeline, validate_recovery_output,
    AnalysisMode, DiagnosticReport, ReconstructionEngineCapability, ReconstructionReport,
    ReconstructionRequest, ReconstructionStatus, RecoveryHistoryEntry, RecoveryHistoryStore,
    RecoveryMethod, RecoveryReport, RecoveryRequest, RecoveryReview, RecoveryStatus,
    ReferenceCompatibilityReport, ValidationLevel, ValidationOutcome, ValidationSummary,
};

use super::media_import::MediaLibraryState;

/// Real-time status update for an active or completed media recovery job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryJobStatus {
    pub job_id: String,
    pub asset_id: String,
    pub status: RecoveryStatus,
    pub stage_message: String,
    pub report: Option<RecoveryReport>,
    pub error_message: Option<String>,
}

/// Real-time status update for an active or completed MP4/MOV reconstruction job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructionJobStatus {
    pub job_id: String,
    pub damaged_asset_id: String,
    pub reference_asset_id: String,
    pub status: ReconstructionStatus,
    pub stage_message: String,
    pub report: Option<ReconstructionReport>,
    pub error_message: Option<String>,
}

/// Shared application state for repair diagnostics, recovery jobs, and cancellation tracking.
#[derive(Clone, Default)]
pub struct RepairDiagnosticsState {
    pub active_cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub latest_reports: Arc<RwLock<HashMap<String, DiagnosticReport>>>,
    pub recovery_jobs: Arc<RwLock<HashMap<String, RecoveryJobStatus>>>,
    pub recovery_cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub reconstruction_jobs: Arc<RwLock<HashMap<String, ReconstructionJobStatus>>>,
    pub reconstruction_cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// Map of job_id -> (staged_file_path, source_canonical_path) for safe finalization
    pub staged_outputs: Arc<RwLock<HashMap<String, (PathBuf, PathBuf)>>>,
    /// Map of job_id -> registered candidate asset ID for preview
    pub candidate_assets: Arc<RwLock<HashMap<String, String>>>,
}

/// Request options for media diagnostics.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseAssetOptions {
    pub asset_id: String,
    pub mode: Option<String>, // "quick" | "deep"
}

/// Opens native file dialog to import a media file specifically for repair diagnosis.
#[tauri::command]
pub async fn pick_and_import_repair_asset(
    library: State<'_, MediaLibraryState>,
) -> Result<Option<MediaAsset>, String> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Select Media File to Diagnose")
        .add_filter(
            "Supported Media (*.mp4, *.mov, *.mkv, *.avi, *.webm, *.m4v)",
            &["mp4", "mov", "mkv", "avi", "webm", "m4v"],
        )
        .add_filter("All Files (*.*)", &["*"]);

    let handle = dialog.pick_file().await;
    let Some(file) = handle else {
        return Ok(None);
    };

    let path = file.path().to_path_buf();
    let asset = library
        .session
        .import_authorized_repair_file(&path, SourceProvenance::NativeFileDialog, None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(asset))
}

/// Executes deterministic, read-only corruption diagnostics on an authorized media asset.
#[tauri::command]
pub async fn run_media_diagnostics(
    asset_id: String,
    mode: Option<String>,
    library: State<'_, MediaLibraryState>,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<DiagnosticReport, String> {
    // 1. Resolve authorized asset from session
    let asset = library
        .session
        .get_asset_by_id(&asset_id)
        .await
        .ok_or_else(|| {
            "Media asset not found in active session or authorization revoked".to_string()
        })?;

    let path = PathBuf::from(&asset.source.canonical_path);

    // 2. Strict Security: Verify path authorization in registry
    library
        .session
        .authorization_registry()
        .verify_authorization(&path, asset.source.file_size)
        .await
        .map_err(|e| format!("Authorization check failed: {e}"))?;

    // 3. Resolve analysis mode
    let analysis_mode = match mode.as_deref() {
        Some("deep") | Some("Deep") => AnalysisMode::Deep,
        _ => AnalysisMode::Quick,
    };

    // 4. Create and register cancellation token
    let token = CancellationToken::new();
    {
        let mut cancellations = repair_state.active_cancellations.lock().await;
        cancellations.insert(asset_id.clone(), token.clone());
    }

    // 5. Execute read-only diagnostics engine
    let report = run_diagnostics(
        &path,
        &asset.id,
        &asset.file_name,
        analysis_mode,
        Some(&token),
    )
    .await;

    // 6. Deregister cancellation token
    {
        let mut cancellations = repair_state.active_cancellations.lock().await;
        cancellations.remove(&asset_id);
    }

    // 7. Store latest report in state
    {
        let mut reports = repair_state.latest_reports.write().await;
        reports.insert(asset_id, report.clone());
    }

    Ok(report)
}

/// Cancels an in-progress diagnostics scan for the given asset.
#[tauri::command]
pub async fn cancel_media_diagnostics(
    asset_id: String,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<bool, String> {
    let cancellations = repair_state.active_cancellations.lock().await;
    if let Some(token) = cancellations.get(&asset_id) {
        token.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Retrieves the latest completed diagnostic report for an asset, if any.
#[tauri::command]
pub async fn get_diagnostic_report(
    asset_id: String,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<Option<DiagnosticReport>, String> {
    let reports = repair_state.latest_reports.read().await;
    Ok(reports.get(&asset_id).cloned())
}

/// Initiates an asynchronous media recovery job in an application-managed staging area.
#[tauri::command]
pub async fn start_media_recovery(
    request: RecoveryRequest,
    app: tauri::AppHandle,
    library: State<'_, MediaLibraryState>,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<RecoveryJobStatus, String> {
    // 1. Resolve and verify authorized asset
    let asset = library
        .session
        .get_asset_by_id(&request.asset_id)
        .await
        .ok_or_else(|| {
            "Media asset not found in active session or authorization revoked".to_string()
        })?;

    let source_path = PathBuf::from(&asset.source.canonical_path);

    library
        .session
        .authorization_registry()
        .verify_authorization(&source_path, asset.source.file_size)
        .await
        .map_err(|e| format!("Authorization verification failed: {e}"))?;

    // 2. Limit concurrent jobs to prevent excessive system load
    {
        let jobs = repair_state.recovery_jobs.read().await;
        let active_count = jobs
            .values()
            .filter(|j| {
                j.status == RecoveryStatus::Running
                    || j.status == RecoveryStatus::Queued
                    || j.status == RecoveryStatus::Validating
            })
            .count();
        if active_count >= 2 {
            return Err("Maximum concurrent recovery jobs (2) reached. Please wait for active jobs to complete.".to_string());
        }
    }

    let job_id = Uuid::new_v4().to_string();
    let token = CancellationToken::new();

    // 3. Staging area setup
    let staging_base = std::env::temp_dir()
        .join("opentake_recovery_staging")
        .join(&job_id);

    let initial_status = RecoveryJobStatus {
        job_id: job_id.clone(),
        asset_id: request.asset_id.clone(),
        status: RecoveryStatus::Running,
        stage_message: "Initializing recovery pipeline and stream inspect...".to_string(),
        report: None,
        error_message: None,
    };

    // 4. Register in state
    {
        let mut jobs = repair_state.recovery_jobs.write().await;
        jobs.insert(job_id.clone(), initial_status.clone());
        let mut cancellations = repair_state.recovery_cancellations.lock().await;
        cancellations.insert(job_id.clone(), token.clone());
    }

    // 5. Spawn background task
    let background_job_id = job_id.clone();
    let background_asset = asset.clone();
    let background_request = request.clone();
    let background_state = repair_state.inner().clone();
    let app_handle = app.clone();

    tokio::spawn(async move {
        let pipeline_result = run_recovery_pipeline(
            &source_path,
            &background_asset.id,
            &background_asset.file_name,
            &background_request,
            &staging_base,
            Some(&token),
        )
        .await;

        // Deregister cancellation
        {
            let mut cancellations = background_state.recovery_cancellations.lock().await;
            cancellations.remove(&background_job_id);
        }

        match pipeline_result {
            Ok(report) => {
                let status = report.processing_status;
                let is_playable = report.validation.is_playable;

                if is_playable {
                    if let Some(ref path_str) = report.output_file_path {
                        let staged_path = PathBuf::from(path_str);
                        let mut staged = background_state.staged_outputs.write().await;
                        staged.insert(
                            background_job_id.clone(),
                            (
                                staged_path,
                                PathBuf::from(&background_asset.source.canonical_path),
                            ),
                        );
                    }
                }

                let stage_message = match status {
                    RecoveryStatus::Completed => {
                        "Recovery and output validation completed successfully.".to_string()
                    }
                    RecoveryStatus::Partial => {
                        "Partial recovery: output is playable with warnings.".to_string()
                    }
                    RecoveryStatus::Failed => {
                        "Recovery failed: media could not be salvaged.".to_string()
                    }
                    RecoveryStatus::Cancelled => "Recovery was cancelled by user.".to_string(),
                    _ => "Recovery finished.".to_string(),
                };

                let history_entry = RecoveryHistoryEntry {
                    job_id: report.job_id.clone(),
                    source_asset_id: report.source_asset_id.clone(),
                    source_file_name: report.source_file_name.clone(),
                    recovery_method: match report.recovery_method {
                        RecoveryMethod::LosslessRemux => "Lossless Remux".to_string(),
                        RecoveryMethod::StreamSalvage => "Stream Salvage".to_string(),
                        RecoveryMethod::TimestampRebuild => "Timestamp Rebuild".to_string(),
                    },
                    attempt_date_time_epoch_ms: report.completed_at_epoch_ms,
                    final_status: status,
                    validation_level: report.validation.validation_level,
                    validation_outcome: if report.validation.passed {
                        if !report.warnings.is_empty() {
                            ValidationOutcome::Warning
                        } else {
                            ValidationOutcome::Passed
                        }
                    } else {
                        ValidationOutcome::Failed
                    },
                    output_available: is_playable,
                    output_file_name: Some(format!(
                        "recovered_{}.{}",
                        report.job_id, report.actual_output_container
                    )),
                    report_exported: false,
                    source_duration_seconds: report.source_duration_seconds,
                    recovered_duration_seconds: report.output_duration_seconds,
                    limitations: report.limitations.clone(),
                };
                record_recovery_history(Some(&app_handle), history_entry);

                let mut jobs = background_state.recovery_jobs.write().await;
                jobs.insert(
                    background_job_id.clone(),
                    RecoveryJobStatus {
                        job_id: background_job_id,
                        asset_id: background_asset.id,
                        status,
                        stage_message,
                        report: Some(report),
                        error_message: None,
                    },
                );
            }
            Err(e) => {
                let is_cancelled = matches!(
                    e,
                    opentake_core::media::error::MediaEngineError::Cancelled { .. }
                );

                let final_status = if is_cancelled {
                    RecoveryStatus::Cancelled
                } else {
                    RecoveryStatus::Failed
                };

                let mut jobs = background_state.recovery_jobs.write().await;
                jobs.insert(
                    background_job_id.clone(),
                    RecoveryJobStatus {
                        job_id: background_job_id,
                        asset_id: background_asset.id,
                        status: final_status,
                        stage_message: if is_cancelled {
                            "Operation cancelled by user.".to_string()
                        } else {
                            format!("Recovery error: {e}")
                        },
                        report: None,
                        error_message: Some(e.to_string()),
                    },
                );
            }
        }
    });

    Ok(initial_status)
}

/// Queries current status of an asynchronous recovery job.
#[tauri::command]
pub async fn get_recovery_status(
    job_id: String,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<Option<RecoveryJobStatus>, String> {
    let jobs = repair_state.recovery_jobs.read().await;
    Ok(jobs.get(&job_id).cloned())
}

/// Cancels an active media recovery job.
#[tauri::command]
pub async fn cancel_media_recovery(
    job_id: String,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<bool, String> {
    let cancellations = repair_state.recovery_cancellations.lock().await;
    if let Some(token) = cancellations.get(&job_id) {
        token.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Finalizes a staged recovery output into a user-specified destination path and
/// registers the resulting file into the active media library session as a newly authorized asset.
#[tauri::command]
pub async fn finalize_recovered_asset(
    job_id: String,
    destination_path: Option<String>,
    library: State<'_, MediaLibraryState>,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<MediaAsset, String> {
    // 1. Retrieve staged output path and source path
    let (staged_path, source_canonical) = {
        let staged = repair_state.staged_outputs.read().await;
        staged
            .get(&job_id)
            .cloned()
            .ok_or_else(|| "No validated recovery output found for this job ID".to_string())?
    };

    // 2. Resolve destination path: if not supplied, open native save dialog
    let target_dest = match destination_path {
        Some(path_str) => PathBuf::from(path_str),
        None => {
            let ext = staged_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4");

            let default_name = format!("recovered_{}.{}", job_id, ext);

            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Save Recovered Media")
                .set_file_name(&default_name)
                .add_filter(
                    "Media Video (*.mp4, *.mkv, *.mov)",
                    &[ext, "mp4", "mkv", "mov"],
                );

            let handle = dialog.save_file().await;
            let Some(file) = handle else {
                return Err("Finalize export cancelled by user".to_string());
            };
            file.path().to_path_buf()
        }
    };

    // 3. Finalize file safely with no-clobber and symlink check
    let finalized_path = finalize_recovered_file(&staged_path, &target_dest, &source_canonical)
        .map_err(|e| format!("Failed to finalize output: {e}"))?;

    // 4. Ingest into active session as a new authorized asset
    let new_asset = library
        .session
        .import_authorized_repair_file(&finalized_path, SourceProvenance::NativeFileDialog, None)
        .await
        .map_err(|e| format!("Failed to register recovered media in library: {e}"))?;

    // 5. Clean up temporary staged file
    let _ = fs::remove_file(&staged_path);
    if let Some(parent) = staged_path.parent() {
        let _ = fs::remove_dir(parent);
    }

    Ok(new_asset)
}

fn sanitize_privacy_string(input: &str) -> String {
    let mut out = input.to_string();
    if let Some(home) = std::env::var_os("HOME") {
        let home_str = home.to_string_lossy();
        if !home_str.is_empty() {
            out = out.replace(home_str.as_ref(), "~");
        }
    }
    if let Some(userprofile) = std::env::var_os("USERPROFILE") {
        let up_str = userprofile.to_string_lossy();
        if !up_str.is_empty() {
            out = out.replace(up_str.as_ref(), "~");
        }
    }
    if let Ok(user) = std::env::var("USER") {
        if user.len() > 2 {
            let target = format!("/home/{user}");
            out = out.replace(&target, "~");
        }
    }
    out
}

/// Exports a sanitized recovery report as a standalone JSON document chosen by the user.
#[tauri::command]
pub async fn export_recovery_report(report: RecoveryReport) -> Result<String, String> {
    let mut sanitized = report.clone();

    if let Some(base) = std::path::Path::new(&sanitized.source_file_name).file_name() {
        sanitized.source_file_name = base.to_string_lossy().to_string();
    }

    if let Some(ref path) = sanitized.output_file_path {
        sanitized.output_file_path = Some(sanitize_privacy_string(path));
    }

    sanitized.validation.details = sanitize_privacy_string(&sanitized.validation.details);
    for w in &mut sanitized.warnings {
        *w = sanitize_privacy_string(w);
    }
    for e in &mut sanitized.observed_ffmpeg_errors {
        *e = sanitize_privacy_string(e);
    }
    for l in &mut sanitized.limitations {
        *l = sanitize_privacy_string(l);
    }

    let json_bytes = serde_json::to_string_pretty(&sanitized)
        .map_err(|e| format!("Serialization error: {e}"))?;

    let default_name = format!("{}_recovery_report.json", sanitized.job_id);

    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Export Media Recovery Report")
        .set_file_name(&default_name)
        .add_filter("JSON Report (*.json)", &["json"]);

    let handle = dialog.save_file().await;
    let Some(dest) = handle else {
        return Err("Export cancelled by user".to_string());
    };

    let target_path = dest.path().to_path_buf();
    fs::write(&target_path, json_bytes)
        .map_err(|e| format!("Failed to write report to {}: {e}", target_path.display()))?;

    Ok(target_path.to_string_lossy().to_string())
}

/// Exports a sanitized diagnostic report as a standalone JSON document chosen by the user.
#[tauri::command]
pub async fn export_diagnostic_report(report: DiagnosticReport) -> Result<String, String> {
    let mut sanitized = report.clone();

    if let Some(base) = std::path::Path::new(&sanitized.file_name).file_name() {
        sanitized.file_name = base.to_string_lossy().to_string();
    }

    sanitized.detected_container.signature_details =
        sanitize_privacy_string(&sanitized.detected_container.signature_details);
    if let Some(ref warn) = sanitized.detected_container.mismatch_warning {
        sanitized.detected_container.mismatch_warning = Some(sanitize_privacy_string(warn));
    }

    for finding in &mut sanitized.findings {
        finding.summary = sanitize_privacy_string(&finding.summary);
        finding.evidence = sanitize_privacy_string(&finding.evidence);
        if let Some(ref action) = finding.recommended_action {
            finding.recommended_action = Some(sanitize_privacy_string(action));
        }
    }

    let json_bytes = serde_json::to_string_pretty(&sanitized)
        .map_err(|e| format!("Serialization error: {e}"))?;

    let default_name = format!("{}_diagnostic_report.json", sanitized.file_name);

    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Export Diagnostic Report")
        .set_file_name(&default_name)
        .add_filter("JSON Report (*.json)", &["json"]);

    let handle = dialog.save_file().await;
    let Some(dest) = handle else {
        return Err("Export cancelled by user".to_string());
    };

    let target_path = dest.path().to_path_buf();
    fs::write(&target_path, json_bytes)
        .map_err(|e| format!("Failed to write report to {}: {e}", target_path.display()))?;

    Ok(target_path.to_string_lossy().to_string())
}

/// Opens native file dialog to import a healthy reference media file for MP4/MOV reconstruction.
#[tauri::command]
pub async fn pick_and_import_reference_asset(
    library: State<'_, MediaLibraryState>,
) -> Result<Option<MediaAsset>, String> {
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Select Healthy Reference Recording (MP4/MOV)")
        .add_filter(
            "MP4/MOV Reference Video (*.mp4, *.mov, *.m4v)",
            &["mp4", "mov", "m4v"],
        )
        .add_filter("All Files (*.*)", &["*"]);

    let handle = dialog.pick_file().await;
    let Some(file) = handle else {
        return Ok(None);
    };

    let path = file.path().to_path_buf();
    let asset = library
        .session
        .import_authorized_repair_file(&path, SourceProvenance::NativeFileDialog, None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(asset))
}

/// Analyzes donor reference recording compatibility with an unfinalized/damaged recording.
#[tauri::command]
pub async fn analyze_reference_compatibility(
    damaged_asset_id: String,
    reference_asset_id: String,
    library: State<'_, MediaLibraryState>,
) -> Result<ReferenceCompatibilityReport, String> {
    if damaged_asset_id == reference_asset_id {
        return Err("Source damaged asset and reference asset must be distinct files.".to_string());
    }

    let damaged_asset = library
        .session
        .get_asset_by_id(&damaged_asset_id)
        .await
        .ok_or_else(|| "Damaged asset not found in active session or unauthorized".to_string())?;

    let reference_asset = library
        .session
        .get_asset_by_id(&reference_asset_id)
        .await
        .ok_or_else(|| "Reference asset not found in active session or unauthorized".to_string())?;

    let damaged_path = PathBuf::from(&damaged_asset.source.canonical_path);
    let reference_path = PathBuf::from(&reference_asset.source.canonical_path);

    library
        .session
        .authorization_registry()
        .verify_authorization(&damaged_path, damaged_asset.source.file_size)
        .await
        .map_err(|e| format!("Damaged file authorization check failed: {e}"))?;

    library
        .session
        .authorization_registry()
        .verify_authorization(&reference_path, reference_asset.source.file_size)
        .await
        .map_err(|e| format!("Reference file authorization check failed: {e}"))?;

    analyze_ref_compat(
        &damaged_path,
        &damaged_asset_id,
        &reference_path,
        &reference_asset_id,
        None,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Initiates an asynchronous reference-based MP4/MOV reconstruction pipeline in application staging.
#[tauri::command]
pub async fn start_mp4_reconstruction(
    request: ReconstructionRequest,
    app: tauri::AppHandle,
    library: State<'_, MediaLibraryState>,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<ReconstructionJobStatus, String> {
    if request.damaged_asset_id == request.reference_asset_id {
        return Err("Damaged asset and reference asset must be distinct files.".to_string());
    }

    let damaged_asset = library
        .session
        .get_asset_by_id(&request.damaged_asset_id)
        .await
        .ok_or_else(|| "Damaged asset not found in active session or unauthorized".to_string())?;

    let reference_asset = library
        .session
        .get_asset_by_id(&request.reference_asset_id)
        .await
        .ok_or_else(|| "Reference asset not found in active session or unauthorized".to_string())?;

    let damaged_path = PathBuf::from(&damaged_asset.source.canonical_path);
    let reference_path = PathBuf::from(&reference_asset.source.canonical_path);

    library
        .session
        .authorization_registry()
        .verify_authorization(&damaged_path, damaged_asset.source.file_size)
        .await
        .map_err(|e| format!("Damaged file authorization check failed: {e}"))?;

    library
        .session
        .authorization_registry()
        .verify_authorization(&reference_path, reference_asset.source.file_size)
        .await
        .map_err(|e| format!("Reference file authorization check failed: {e}"))?;

    // Check concurrency limits
    {
        let jobs = repair_state.reconstruction_jobs.read().await;
        let active_count = jobs
            .values()
            .filter(|j| {
                j.status == ReconstructionStatus::Queued
                    || j.status == ReconstructionStatus::CheckingReference
                    || j.status == ReconstructionStatus::Analyzing
                    || j.status == ReconstructionStatus::Reconstructing
                    || j.status == ReconstructionStatus::Validating
            })
            .count();
        if active_count >= 2 {
            return Err("Maximum concurrent reconstruction jobs (2) reached. Please wait for active jobs to complete.".to_string());
        }
    }

    let job_id = Uuid::new_v4().to_string();
    let token = CancellationToken::new();

    let staging_base = std::env::temp_dir()
        .join("opentake_recovery_staging")
        .join(&job_id);

    let initial_status = ReconstructionJobStatus {
        job_id: job_id.clone(),
        damaged_asset_id: request.damaged_asset_id.clone(),
        reference_asset_id: request.reference_asset_id.clone(),
        status: ReconstructionStatus::Analyzing,
        stage_message: "Analyzing reference compatibility and donor parameters...".to_string(),
        report: None,
        error_message: None,
    };

    {
        let mut jobs = repair_state.reconstruction_jobs.write().await;
        jobs.insert(job_id.clone(), initial_status.clone());
        let mut cancellations = repair_state.reconstruction_cancellations.lock().await;
        cancellations.insert(job_id.clone(), token.clone());
    }

    let background_job_id = job_id.clone();
    let background_damaged_asset = damaged_asset.clone();
    let background_ref_asset = reference_asset.clone();
    let background_request = request.clone();
    let background_state = repair_state.inner().clone();
    let app_handle = app.clone();

    tokio::spawn(async move {
        let pipeline_result = run_reconstruction_pipeline(
            &damaged_path,
            &background_damaged_asset.id,
            &background_damaged_asset.file_name,
            &reference_path,
            &background_ref_asset.id,
            &background_ref_asset.file_name,
            &background_request,
            &staging_base,
            Some(&token),
        )
        .await;

        {
            let mut cancellations = background_state.reconstruction_cancellations.lock().await;
            cancellations.remove(&background_job_id);
        }

        match pipeline_result {
            Ok(report) => {
                let status = report.processing_status;
                let is_playable = report.validation.is_playable;

                if is_playable {
                    if let Some(ref path_str) = report.output_file_path {
                        let staged_path = PathBuf::from(path_str);
                        let mut staged = background_state.staged_outputs.write().await;
                        staged.insert(
                            background_job_id.clone(),
                            (
                                staged_path,
                                PathBuf::from(&background_damaged_asset.source.canonical_path),
                            ),
                        );
                    }
                }

                let stage_message = match status {
                    ReconstructionStatus::Completed => {
                        "Reference-based reconstruction and output validation completed successfully.".to_string()
                    }
                    ReconstructionStatus::Partial => {
                        "Partial reconstruction: output is playable with warnings.".to_string()
                    }
                    ReconstructionStatus::Failed => {
                        "Reconstruction failed: media could not be recovered.".to_string()
                    }
                    ReconstructionStatus::Cancelled => "Reconstruction was cancelled by user.".to_string(),
                    _ => "Reconstruction finished.".to_string(),
                };

                let history_entry = RecoveryHistoryEntry {
                    job_id: report.operation_id.clone(),
                    source_asset_id: report.damaged_asset_id.clone(),
                    source_file_name: report.damaged_file_name.clone(),
                    recovery_method: "Reference-Based MP4 Reconstruction".to_string(),
                    attempt_date_time_epoch_ms: report.completed_at_epoch_ms,
                    final_status: match report.processing_status {
                        ReconstructionStatus::Completed => RecoveryStatus::Completed,
                        ReconstructionStatus::Partial => RecoveryStatus::Partial,
                        ReconstructionStatus::Failed => RecoveryStatus::Failed,
                        ReconstructionStatus::Cancelled => RecoveryStatus::Cancelled,
                        _ => RecoveryStatus::Running,
                    },
                    validation_level: report.validation.validation_level,
                    validation_outcome: if report.validation.passed {
                        if !report.warnings.is_empty() {
                            ValidationOutcome::Warning
                        } else {
                            ValidationOutcome::Passed
                        }
                    } else {
                        ValidationOutcome::Failed
                    },
                    output_available: is_playable,
                    output_file_name: Some(format!("recovered_{}.mp4", report.operation_id)),
                    report_exported: false,
                    source_duration_seconds: None,
                    recovered_duration_seconds: report.recovered_duration_seconds,
                    limitations: report.uncertainties.clone(),
                };
                record_recovery_history(Some(&app_handle), history_entry);

                let mut jobs = background_state.reconstruction_jobs.write().await;
                jobs.insert(
                    background_job_id.clone(),
                    ReconstructionJobStatus {
                        job_id: background_job_id,
                        damaged_asset_id: background_damaged_asset.id,
                        reference_asset_id: background_ref_asset.id,
                        status,
                        stage_message,
                        report: Some(report),
                        error_message: None,
                    },
                );
            }
            Err(e) => {
                let is_cancelled = matches!(
                    e,
                    opentake_core::media::error::MediaEngineError::Cancelled { .. }
                );

                let final_status = if is_cancelled {
                    ReconstructionStatus::Cancelled
                } else {
                    ReconstructionStatus::Failed
                };

                let mut jobs = background_state.reconstruction_jobs.write().await;
                jobs.insert(
                    background_job_id.clone(),
                    ReconstructionJobStatus {
                        job_id: background_job_id,
                        damaged_asset_id: background_damaged_asset.id,
                        reference_asset_id: background_ref_asset.id,
                        status: final_status,
                        stage_message: if is_cancelled {
                            "Operation cancelled by user.".to_string()
                        } else {
                            format!("Reconstruction error: {e}")
                        },
                        report: None,
                        error_message: Some(e.to_string()),
                    },
                );
            }
        }
    });

    Ok(initial_status)
}

/// Queries current status of an asynchronous reconstruction job.
#[tauri::command]
pub async fn get_reconstruction_status(
    job_id: String,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<Option<ReconstructionJobStatus>, String> {
    let jobs = repair_state.reconstruction_jobs.read().await;
    Ok(jobs.get(&job_id).cloned())
}

/// Cancels an active MP4/MOV reconstruction job.
#[tauri::command]
pub async fn cancel_mp4_reconstruction(
    job_id: String,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<bool, String> {
    let cancellations = repair_state.reconstruction_cancellations.lock().await;
    if let Some(token) = cancellations.get(&job_id) {
        token.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Returns discovered reconstruction engine capabilities.
#[tauri::command]
pub async fn get_reconstruction_capabilities() -> Result<Vec<ReconstructionEngineCapability>, String>
{
    Ok(discover_reconstruction_capabilities())
}

/// Exports a sanitized reconstruction report as a standalone JSON document chosen by the user.
#[tauri::command]
pub async fn export_reconstruction_report(report: ReconstructionReport) -> Result<String, String> {
    let mut sanitized = report.clone();

    if let Some(ref mut path) = sanitized.output_file_path {
        *path = sanitize_privacy_string(path);
    }
    for w in &mut sanitized.warnings {
        *w = sanitize_privacy_string(w);
    }
    for u in &mut sanitized.uncertainties {
        *u = sanitize_privacy_string(u);
    }
    for w in &mut sanitized.reference_compatibility.warnings {
        *w = sanitize_privacy_string(w);
    }

    let json_bytes = serde_json::to_string_pretty(&sanitized)
        .map_err(|e| format!("Serialization error: {e}"))?;

    let default_name = format!("{}_reconstruction_report.json", sanitized.operation_id);

    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Export Reconstruction Report")
        .set_file_name(&default_name)
        .add_filter("JSON Report (*.json)", &["json"]);

    let handle = dialog.save_file().await;
    let Some(dest) = handle else {
        return Err("Export cancelled by user".to_string());
    };

    let target_path = dest.path().to_path_buf();
    fs::write(&target_path, json_bytes)
        .map_err(|e| format!("Failed to write report to {}: {e}", target_path.display()))?;

    Ok(target_path.to_string_lossy().to_string())
}

fn get_history_file_path(app: Option<&tauri::AppHandle>) -> PathBuf {
    use tauri::Manager;
    if let Some(app) = app {
        if let Ok(dir) = app.path().app_data_dir() {
            let _ = fs::create_dir_all(&dir);
            return dir.join("recovery_history.json");
        }
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let dir = PathBuf::from(home).join(".open-media-workbench");
        let _ = fs::create_dir_all(&dir);
        return dir.join("recovery_history.json");
    }
    std::env::temp_dir().join("open_media_workbench_recovery_history.json")
}

fn record_recovery_history(app: Option<&tauri::AppHandle>, entry: RecoveryHistoryEntry) {
    let history_file = get_history_file_path(app);
    let mut store = RecoveryHistoryStore::load_from_file(&history_file);
    store.add_entry(entry);
    let _ = store.save_to_file(&history_file);
}

fn mark_history_report_exported(app: Option<&tauri::AppHandle>, job_id: &str) {
    let history_file = get_history_file_path(app);
    let mut store = RecoveryHistoryStore::load_from_file(&history_file);
    if let Some(entry) = store.entries.iter_mut().find(|e| e.job_id == job_id) {
        entry.report_exported = true;
        let _ = store.save_to_file(&history_file);
    }
}

/// Registers a validated recovery or reconstruction candidate from app-owned staging
/// into the active MediaLibrarySession under `SourceProvenance::StagedRecoveryCandidate`.
/// Returns the resulting MediaAsset so the frontend can preview it via opentake-media://.
#[tauri::command]
pub async fn register_recovery_candidate_preview(
    job_id: String,
    library: State<'_, MediaLibraryState>,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<MediaAsset, String> {
    // 1. Check if already registered
    {
        let candidates = repair_state.candidate_assets.read().await;
        if let Some(asset_id) = candidates.get(&job_id) {
            if let Some(asset) = library.session.get_asset_by_id(asset_id).await {
                return Ok(asset);
            }
        }
    }

    // 2. Look up staged output
    let (staged_path, _) = {
        let staged = repair_state.staged_outputs.read().await;
        staged
            .get(&job_id)
            .cloned()
            .ok_or_else(|| "No validated recovery output found for this job ID".to_string())?
    };

    if !staged_path.exists() {
        return Err("Staged recovery file does not exist on disk".to_string());
    }

    // 3. Register under StagedRecoveryCandidate provenance
    let asset = library
        .session
        .import_authorized_repair_file(
            &staged_path,
            SourceProvenance::StagedRecoveryCandidate,
            None,
        )
        .await
        .map_err(|e| format!("Failed to register recovery candidate for preview: {e}"))?;

    // 4. Save mapping
    {
        let mut candidates = repair_state.candidate_assets.write().await;
        candidates.insert(job_id, asset.id.clone());
    }

    Ok(asset)
}

/// Returns the unified RecoveryReview for a completed recovery or reconstruction job.
#[tauri::command]
pub async fn get_recovery_review(
    job_id: String,
    library: State<'_, MediaLibraryState>,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<RecoveryReview, String> {
    let candidate_asset_id = {
        let candidates = repair_state.candidate_assets.read().await;
        candidates.get(&job_id).cloned()
    };

    // 1. Check recovery jobs
    {
        let rec_jobs = repair_state.recovery_jobs.read().await;
        if let Some(rec_job) = rec_jobs.get(&job_id) {
            if let Some(report) = &rec_job.report {
                let source_is_playable = if let Some(source_asset) = library
                    .session
                    .get_asset_by_id(&report.source_asset_id)
                    .await
                {
                    source_asset.compatibility.overall == CompatibilityStatus::Supported
                        || source_asset.compatibility.overall
                            == CompatibilityStatus::SupportedWithLimitations
                } else {
                    false
                };

                return Ok(RecoveryReview::from_recovery_report(
                    report,
                    source_is_playable,
                    candidate_asset_id,
                ));
            }
        }
    }

    // 2. Check reconstruction jobs
    {
        let recon_jobs = repair_state.reconstruction_jobs.read().await;
        if let Some(recon_job) = recon_jobs.get(&job_id) {
            if let Some(report) = &recon_job.report {
                return Ok(RecoveryReview::from_reconstruction_report(
                    report,
                    candidate_asset_id,
                ));
            }
        }
    }

    Err(format!(
        "No completed recovery or reconstruction report found for job ID '{job_id}'"
    ))
}

/// Runs explicit on-demand validation (e.g. FullDecode) on a staged candidate file.
#[tauri::command]
pub async fn run_advanced_output_validation(
    job_id: String,
    level: String,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<ValidationSummary, String> {
    let (staged_path, _) = {
        let staged = repair_state.staged_outputs.read().await;
        staged
            .get(&job_id)
            .cloned()
            .ok_or_else(|| "No validated recovery output found for this job ID".to_string())?
    };

    let val_level = match level.as_str() {
        "metadataOnly" => ValidationLevel::MetadataOnly,
        "fullDecode" => ValidationLevel::FullDecode,
        _ => ValidationLevel::SampledDecode,
    };

    let outcome = validate_recovery_output(&staged_path, 1, 0, val_level, None).await;

    // Update stored report's validation outcome
    {
        let mut rec_jobs = repair_state.recovery_jobs.write().await;
        if let Some(job) = rec_jobs.get_mut(&job_id) {
            if let Some(report) = &mut job.report {
                report.validation = outcome.validation_result.clone();
                report.processing_status = outcome.status;
            }
        }
    }
    {
        let mut recon_jobs = repair_state.reconstruction_jobs.write().await;
        if let Some(job) = recon_jobs.get_mut(&job_id) {
            if let Some(report) = &mut job.report {
                report.validation = outcome.validation_result.clone();
            }
        }
    }

    let review = if let Some(rec_job) = repair_state.recovery_jobs.read().await.get(&job_id) {
        if let Some(rep) = &rec_job.report {
            RecoveryReview::from_recovery_report(rep, false, None)
        } else {
            return Err("Report missing".to_string());
        }
    } else if let Some(recon_job) = repair_state.reconstruction_jobs.read().await.get(&job_id) {
        if let Some(rep) = &recon_job.report {
            RecoveryReview::from_reconstruction_report(rep, None)
        } else {
            return Err("Report missing".to_string());
        }
    } else {
        return Err("Job not found".to_string());
    };

    Ok(review.validation_summary)
}

/// Exports a RecoveryReview as either standalone HTML or sanitized JSON.
#[tauri::command]
pub async fn export_recovery_review_report(
    job_id: String,
    format: String,
    app: tauri::AppHandle,
    library: State<'_, MediaLibraryState>,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<String, String> {
    let review = get_recovery_review(job_id.clone(), library, repair_state).await?;

    let is_html = format.to_lowercase() == "html";
    let (content, ext, filter_name) = if is_html {
        (
            generate_recovery_html_report(&review),
            "html",
            "HTML Document (*.html)",
        )
    } else {
        let json = generate_recovery_json_report(&review)
            .map_err(|e| format!("Failed to serialize report: {e}"))?;
        (json, "json", "JSON File (*.json)")
    };

    let default_filename = format!("recovery_report_{}.{}", review.recovery_job_id, ext);

    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Export Recovery Report")
        .set_file_name(&default_filename)
        .add_filter(filter_name, &[ext]);

    let handle = dialog.save_file().await;
    let Some(file) = handle else {
        return Err("Export cancelled by user".to_string());
    };

    let target_path = file.path().to_path_buf();
    fs::write(&target_path, content.as_bytes())
        .map_err(|e| format!("Failed to write export file: {e}"))?;

    mark_history_report_exported(Some(&app), &job_id);

    Ok(sanitize_privacy_string(&target_path.to_string_lossy()))
}

/// Retrieves persistent recovery history records.
#[tauri::command]
pub async fn get_recovery_history(
    app: tauri::AppHandle,
    repair_state: State<'_, RepairDiagnosticsState>,
) -> Result<Vec<RecoveryHistoryEntry>, String> {
    let history_file = get_history_file_path(Some(&app));
    let store = RecoveryHistoryStore::load_from_file(&history_file);
    let staged = repair_state.staged_outputs.read().await;

    let mut entries = store.entries;
    for entry in &mut entries {
        if let Some((staged_path, _)) = staged.get(&entry.job_id) {
            entry.output_available = staged_path.exists();
        }
    }

    Ok(entries)
}

/// Clears all past recovery history records from disk.
#[tauri::command]
pub async fn clear_recovery_history(app: tauri::AppHandle) -> Result<bool, String> {
    let history_file = get_history_file_path(Some(&app));
    let mut store = RecoveryHistoryStore::load_from_file(&history_file);
    store.clear();
    store
        .save_to_file(&history_file)
        .map_err(|e| format!("Failed to clear history: {e}"))?;
    Ok(true)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repair_diagnostics_state_initialization() {
        let state = RepairDiagnosticsState::default();
        let cancellations = state.active_cancellations.lock().await;
        assert!(cancellations.is_empty());
        let reports = state.latest_reports.read().await;
        assert!(reports.is_empty());
        let jobs = state.recovery_jobs.read().await;
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn test_repair_cancellation_tracking() {
        let state = RepairDiagnosticsState::default();
        let token = CancellationToken::new();

        {
            let mut cancellations = state.active_cancellations.lock().await;
            cancellations.insert("asset-test-cancel".to_string(), token.clone());
        }

        assert!(!token.is_cancelled());

        let cancellations = state.active_cancellations.lock().await;
        if let Some(t) = cancellations.get("asset-test-cancel") {
            t.cancel();
        }
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_recovery_job_cancellation_tracking() {
        let state = RepairDiagnosticsState::default();
        let token = CancellationToken::new();

        {
            let mut cancellations = state.recovery_cancellations.lock().await;
            cancellations.insert("job-test-cancel".to_string(), token.clone());
        }

        assert!(!token.is_cancelled());

        let cancellations = state.recovery_cancellations.lock().await;
        if let Some(t) = cancellations.get("job-test-cancel") {
            t.cancel();
        }
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_reconstruction_state_and_cancellation() {
        let state = RepairDiagnosticsState::default();
        let token = CancellationToken::new();

        {
            let mut cancellations = state.reconstruction_cancellations.lock().await;
            cancellations.insert("reconst-job-1".to_string(), token.clone());
        }

        assert!(!token.is_cancelled());

        let cancellations = state.reconstruction_cancellations.lock().await;
        if let Some(t) = cancellations.get("reconst-job-1") {
            t.cancel();
        }
        assert!(token.is_cancelled());

        let jobs = state.reconstruction_jobs.read().await;
        assert!(jobs.is_empty());
    }
}
