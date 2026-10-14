use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use tauri::State;
use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use opentake_core::media::authorization::SourceProvenance;
use opentake_core::media::identity::MediaAsset;
use opentake_core::repair::{run_diagnostics, AnalysisMode, DiagnosticReport};

use super::media_import::MediaLibraryState;

/// Shared application state for repair diagnostics and cancellation tracking.
#[derive(Clone, Default)]
pub struct RepairDiagnosticsState {
    pub active_cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
    pub latest_reports: Arc<RwLock<HashMap<String, DiagnosticReport>>>,
}

/// Request options for media diagnostics.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseAssetOptions {
    pub asset_id: String,
    pub mode: Option<String>, // "quick" | "deep"
}

/// Opens native file dialog to import a media file specifically for repair diagnosis.
///
/// Unlike standard import, if FFprobe fails because of container defects (e.g. missing moov atom),
/// the file is still safely registered with verified provenance so diagnostics can inspect it.
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

/// Exports a sanitized diagnostic report as a standalone JSON document chosen by the user.
#[tauri::command]
pub async fn export_diagnostic_report(report: DiagnosticReport) -> Result<String, String> {
    // Sanitize any internal absolute paths in findings or evidence to protect user privacy
    let mut sanitized = report.clone();

    // Ensure file_name is strictly a clean basename
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
}
