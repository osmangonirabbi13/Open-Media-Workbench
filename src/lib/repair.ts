import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type { AnalysisMode, DiagnosticReport } from "../types/repair";
import type { MediaAsset } from "../types/media";

/**
 * Runs deterministic video corruption diagnostics on an authorized asset.
 */
export async function runMediaDiagnostics(
  assetId: string,
  mode: AnalysisMode = "quick"
): Promise<DiagnosticReport> {
  if (!isTauriEnvironment()) {
    // Development fallback mock
    return {
      schemaVersion: 1,
      analysisId: `mock-diag-${Date.now()}`,
      assetId,
      fileName: "mock_asset.mp4",
      detectedContainer: {
        formatType: "MP4 (ISO BMFF)",
        extension: "mp4",
        signatureMatched: true,
        signatureDetails: "Mock ISO BMFF ftyp box",
        mismatchWarning: null,
      },
      analysisMode: mode,
      startedAtEpochMs: Date.now() - 500,
      completedAtEpochMs: Date.now(),
      analysisStatus: "completed",
      findings: [
        {
          code: "DECODE_SAMPLE_CLEAN",
          severity: "info",
          category: "decodeSample",
          summary: "Browser development preview mode (mock)",
          evidence: "Tauri IPC is not connected in this browser session.",
          affectedStream: null,
          mediaTimeRangeMs: null,
          byteRange: null,
          confidence: "High",
          recommendedAction: null,
        },
      ],
      recommendations: [
        {
          strategyId: "NO_REPAIR_NEEDED",
          title: "Media Appears Structurally Healthy",
          description: "No container structural defects detected in mock mode.",
          confidence: "High",
          requiredEvidence: ["Mock inspection completed"],
          targetStep: 12,
          availableNow: true,
        },
      ],
      inspectionCoverage: {
        headerScannedBytes: 256,
        totalFileBytes: 1048576,
        boxCountFound: 3,
        probeExecuted: true,
        decodeSampled: mode === "deep",
        decodeSampleFrames: mode === "deep" ? 30 : null,
      },
      toolVersions: "Mock Diagnostic Engine v1.0",
    };
  }

  try {
    return await invoke<DiagnosticReport>("run_media_diagnostics", {
      assetId,
      mode,
    });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Media diagnostics failed: ${message}`);
  }
}

/**
 * Cancels an in-progress diagnostics scan for the given asset.
 */
export async function cancelMediaDiagnostics(assetId: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    return true;
  }

  try {
    return await invoke<boolean>("cancel_media_diagnostics", { assetId });
  } catch (err: unknown) {
    console.error("Failed to cancel diagnostics:", err);
    return false;
  }
}

/**
 * Retrieves the latest cached diagnostic report for an asset if available.
 */
export async function getDiagnosticReport(
  assetId: string
): Promise<DiagnosticReport | null> {
  if (!isTauriEnvironment()) {
    return null;
  }

  try {
    return await invoke<DiagnosticReport | null>("get_diagnostic_report", {
      assetId,
    });
  } catch (err: unknown) {
    console.error("Failed to get diagnostic report:", err);
    return null;
  }
}

/**
 * Exports the diagnostic report to a user-chosen JSON file.
 */
export async function exportDiagnosticReport(
  report: DiagnosticReport
): Promise<string> {
  if (!isTauriEnvironment()) {
    const dataStr =
      "data:text/json;charset=utf-8," +
      encodeURIComponent(JSON.stringify(report, null, 2));
    const downloadAnchor = document.createElement("a");
    downloadAnchor.setAttribute("href", dataStr);
    downloadAnchor.setAttribute(
      "download",
      `${report.fileName}_diagnostic_report.json`
    );
    document.body.appendChild(downloadAnchor);
    downloadAnchor.click();
    downloadAnchor.remove();
    return "browser_download";
  }

  try {
    return await invoke<string>("export_diagnostic_report", { report });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Report export failed: ${message}`);
  }
}

/**
 * Opens native file dialog and imports a media file specifically for repair inspection.
 */
export async function pickAndImportRepairAsset(): Promise<MediaAsset | null> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri desktop environment required for repair file import.");
  }

  try {
    return await invoke<MediaAsset | null>("pick_and_import_repair_asset");
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Repair import failed: ${message}`);
  }
}
