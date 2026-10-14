import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type {
  AnalysisMode,
  DiagnosticReport,
  RecoveryJobStatus,
  RecoveryReport,
  RecoveryRequest,
} from "../types/repair";
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

// ==========================================
// STEP 13 — Recovery Pipeline Client API
// ==========================================

/**
 * Initiates an asynchronous media recovery job in an isolated application-managed staging area.
 */
export async function startMediaRecovery(
  request: RecoveryRequest
): Promise<RecoveryJobStatus> {
  if (!isTauriEnvironment()) {
    // Development fallback mock
    return {
      jobId: `mock-job-${Date.now()}`,
      assetId: request.assetId,
      status: "completed",
      stageMessage: "Recovery completed in browser mock environment.",
      report: {
        schemaVersion: 1,
        jobId: `mock-job-${Date.now()}`,
        sourceAssetId: request.assetId,
        sourceFileName: "mock_asset.mp4",
        recoveryMethod: request.method,
        originalContainer: "mp4",
        intendedOutputContainer: request.outputContainerPreference || "mp4",
        actualOutputContainer: "mp4",
        inputStreams: [
          {
            index: 0,
            streamType: "video",
            codecName: "h264",
            durationSeconds: 10.0,
            resolution: "1920x1080",
          },
        ],
        outputStreams: [
          {
            index: 0,
            streamType: "video",
            codecName: "h264",
            durationSeconds: 10.0,
            resolution: "1920x1080",
          },
        ],
        processingStatus: "completed",
        validation: {
          passed: true,
          isPlayable: true,
          validationLevel: "sampledDecode",
          streamsPresent: ["Video stream (codec: h264, 1920x1080)"],
          streamsExpected: ["1 video stream(s)"],
          streamLossDetected: false,
          outputDurationSeconds: 10.0,
          details: "Verified 1 stream in mock environment.",
          warnings: [],
        },
        warnings: [],
        observedFfmpegErrors: [],
        outputFileSizeBytes: 1024 * 1024,
        outputFilePath: "/mock/staging/recovered.mp4",
        sourceDurationSeconds: 10.0,
        outputDurationSeconds: 10.0,
        durationDifferenceSeconds: 0.0,
        startedAtEpochMs: Date.now() - 1000,
        completedAtEpochMs: Date.now(),
        limitations: [],
      },
      errorMessage: null,
    };
  }

  try {
    return await invoke<RecoveryJobStatus>("start_media_recovery", { request });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to start recovery: ${message}`);
  }
}

/**
 * Polls the current status of an ongoing or completed recovery job.
 */
export async function getRecoveryStatus(
  jobId: string
): Promise<RecoveryJobStatus | null> {
  if (!isTauriEnvironment()) {
    return null;
  }

  try {
    return await invoke<RecoveryJobStatus | null>("get_recovery_status", {
      jobId,
    });
  } catch (err: unknown) {
    console.error("Failed to query recovery status:", err);
    return null;
  }
}

/**
 * Cancels a running media recovery job.
 */
export async function cancelMediaRecovery(jobId: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    return true;
  }

  try {
    return await invoke<boolean>("cancel_media_recovery", { jobId });
  } catch (err: unknown) {
    console.error("Failed to cancel recovery job:", err);
    return false;
  }
}

/**
 * Finalizes the staged recovery output to a user-chosen destination and registers
 * it as a newly authorized asset in the library session.
 */
export async function finalizeRecoveredAsset(
  jobId: string,
  destinationPath?: string
): Promise<MediaAsset> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri desktop environment required to finalize recovered media.");
  }

  try {
    return await invoke<MediaAsset>("finalize_recovered_asset", {
      jobId,
      destinationPath: destinationPath || null,
    });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Finalize recovered asset failed: ${message}`);
  }
}

/**
 * Exports a versioned, privacy-sanitized recovery report as a standalone JSON file.
 */
export async function exportRecoveryReport(
  report: RecoveryReport
): Promise<string> {
  if (!isTauriEnvironment()) {
    const dataStr =
      "data:text/json;charset=utf-8," +
      encodeURIComponent(JSON.stringify(report, null, 2));
    const downloadAnchor = document.createElement("a");
    downloadAnchor.setAttribute("href", dataStr);
    downloadAnchor.setAttribute(
      "download",
      `${report.jobId}_recovery_report.json`
    );
    document.body.appendChild(downloadAnchor);
    downloadAnchor.click();
    downloadAnchor.remove();
    return "browser_download";
  }

  try {
    return await invoke<string>("export_recovery_report", { report });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Recovery report export failed: ${message}`);
  }
}
