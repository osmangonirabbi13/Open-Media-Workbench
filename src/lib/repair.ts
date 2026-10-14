import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type {
  AnalysisMode,
  DiagnosticReport,
  ReconstructionEngineCapability,
  ReconstructionJobStatus,
  ReconstructionReport,
  ReconstructionRequest,
  RecoveryJobStatus,
  RecoveryReport,
  RecoveryRequest,
  ReferenceCompatibilityReport,
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

// ==========================================
// STEP 14 — MP4/MOV Reconstruction Functions
// ==========================================

/**
 * Prompts user to pick an authorized healthy reference recording for reconstruction.
 */
export async function pickAndImportReferenceAsset(): Promise<MediaAsset | null> {
  if (!isTauriEnvironment()) {
    return {
      id: "mock-ref-asset-id",
      fileName: "mock_reference.mp4",
      source: {
        canonicalPath: "/mock/media/mock_reference.mp4",
        fileSize: 5242880,
        modifiedEpochMs: Date.now() - 3600000,
        identityHint: "mock-ref-hint",
      },
      metadata: {
        container: {
          formatName: "mov,mp4,m4a,3gp,3g2,mj2",
          formatLongName: "QuickTime / MOV",
          durationMicros: 10000000,
          startTimeMicros: 0,
          bitrateBps: 4194304,
          sizeBytes: 5242880,
          streamCount: 2,
        },
        videoStreams: [
          {
            streamIndex: 0,
            codecName: "h264",
            codecLongName: "H.264 / AVC / MPEG-4 AVC / MPEG-4 part 10",
            width: 1920,
            height: 1080,
            pixelFormat: "yuv420p",
            averageFrameRate: { num: 30, den: 1 },
            nominalFrameRate: { num: 30, den: 1 },
            timebase: { num: 1, den: 15360 },
            durationMicros: 10000000,
            profile: "High",
            isVfrSuspect: false,
          },
        ],
        audioStreams: [
          {
            streamIndex: 1,
            codecName: "aac",
            codecLongName: "AAC (Advanced Audio Coding)",
            sampleRate: 48000,
            channels: 2,
            channelLayout: "stereo",
            bitrateBps: 128000,
            durationMicros: 10000000,
          },
        ],
        otherStreams: [],
      },
      compatibility: {
        overall: "supported",
        containerStatus: "supported",
        videoStatus: "supported",
        audioStatus: "supported",
        canInspect: true,
        canEditFuture: true,
        warnings: [],
      },
      importedAtEpochMs: Date.now(),
    };
  }

  try {
    return await invoke<MediaAsset | null>("pick_and_import_reference_asset");
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to import reference asset: ${message}`);
  }
}

/**
 * Analyzes compatibility of a healthy reference donor against a damaged recording.
 */
export async function analyzeReferenceCompatibility(
  damagedAssetId: string,
  referenceAssetId: string
): Promise<ReferenceCompatibilityReport> {
  if (!isTauriEnvironment()) {
    return {
      schemaVersion: 1,
      referenceAssetId,
      damagedAssetId,
      referenceContainer: "MP4 (ISO BMFF)",
      damagedContainer: "MP4 (ISO BMFF)",
      tracks: [
        {
          trackId: 0,
          mediaType: "video",
          codecName: "h264",
          codecProfile: "High",
          resolution: "1920x1080",
          frameRate: 30,
          timebase: "1/15360",
          sampleRate: null,
          channels: null,
          extradataSize: 54,
        },
      ],
      compatibility: "compatible",
      summary: "Donor file matches damaged media container and video codec profile.",
      warnings: [],
      unknownFields: [],
    };
  }

  try {
    return await invoke<ReferenceCompatibilityReport>(
      "analyze_reference_compatibility",
      { damagedAssetId, referenceAssetId }
    );
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Reference compatibility analysis failed: ${message}`);
  }
}

/**
 * Initiates reference-based MP4/MOV reconstruction in application staging.
 */
export async function startMp4Reconstruction(
  request: ReconstructionRequest
): Promise<ReconstructionJobStatus> {
  if (!isTauriEnvironment()) {
    const mockJobId = `mock-reconst-${Date.now()}`;
    return {
      jobId: mockJobId,
      damagedAssetId: request.damagedAssetId,
      referenceAssetId: request.referenceAssetId,
      status: "completed",
      stageMessage: "Reconstruction completed (Browser mock mode)",
      report: {
        schemaVersion: 1,
        operationId: mockJobId,
        engineName: "Mock Native Reconstructor",
        engineVersion: "1.0.0-mock",
        damagedAssetId: request.damagedAssetId,
        referenceAssetId: request.referenceAssetId,
        damagedFileName: "unfinalized_damaged.mp4",
        referenceFileName: "healthy_reference.mp4",
        referenceCompatibility: {
          schemaVersion: 1,
          referenceAssetId: request.referenceAssetId,
          damagedAssetId: request.damagedAssetId,
          referenceContainer: "MP4",
          damagedContainer: "MP4",
          tracks: [],
          compatibility: "compatible",
          summary: "Compatible",
          warnings: [],
          unknownFields: [],
        },
        attemptedMode: "Mock moov synthesis",
        reconstructedStreams: [
          {
            index: 0,
            streamType: "video",
            codecName: "h264",
            durationSeconds: 10.0,
            resolution: "1920x1080",
          },
        ],
        recoveredDurationSeconds: 10.0,
        validation: {
          passed: true,
          isPlayable: true,
          validationLevel: "sampledDecode",
          streamsPresent: ["h264"],
          streamsExpected: ["h264"],
          streamLossDetected: false,
          outputDurationSeconds: 10.0,
          details: "Mock output validated successfully",
          warnings: [],
        },
        outputFileSizeBytes: 1048576,
        outputFilePath: "/mock/staging/reconstructed_output.mp4",
        processingStatus: "completed",
        failureDetails: null,
        warnings: [],
        uncertainties: ["Mock reconstruction mode"],
        startedAtEpochMs: Date.now() - 1000,
        completedAtEpochMs: Date.now(),
      },
      errorMessage: null,
    };
  }

  try {
    return await invoke<ReconstructionJobStatus>("start_mp4_reconstruction", {
      request,
    });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to start reconstruction: ${message}`);
  }
}

/**
 * Polls status of an active reconstruction job.
 */
export async function getReconstructionStatus(
  jobId: string
): Promise<ReconstructionJobStatus | null> {
  if (!isTauriEnvironment()) {
    return null;
  }

  try {
    return await invoke<ReconstructionJobStatus | null>(
      "get_reconstruction_status",
      { jobId }
    );
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to query reconstruction status: ${message}`);
  }
}

/**
 * Cancels an active MP4/MOV reconstruction job.
 */
export async function cancelMp4Reconstruction(jobId: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    return true;
  }

  try {
    return await invoke<boolean>("cancel_mp4_reconstruction", { jobId });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to cancel reconstruction: ${message}`);
  }
}

/**
 * Discovers available reconstruction engines on the host system.
 */
export async function getReconstructionCapabilities(): Promise<
  ReconstructionEngineCapability[]
> {
  if (!isTauriEnvironment()) {
    return [
      {
        engineId: "native",
        name: "Built-in Safe Rust Reconstructor",
        version: "1.0.0-mock",
        isAvailable: true,
        description: "Mock native reconstructor",
      },
    ];
  }

  try {
    return await invoke<ReconstructionEngineCapability[]>(
      "get_reconstruction_capabilities"
    );
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to discover reconstruction capabilities: ${message}`);
  }
}

/**
 * Exports a reconstruction report as a standalone JSON file.
 */
export async function exportReconstructionReport(
  report: ReconstructionReport
): Promise<string> {
  if (!isTauriEnvironment()) {
    const dataStr =
      "data:text/json;charset=utf-8," +
      encodeURIComponent(JSON.stringify(report, null, 2));
    const downloadAnchor = document.createElement("a");
    downloadAnchor.setAttribute("href", dataStr);
    downloadAnchor.setAttribute(
      "download",
      `${report.operationId}_reconstruction_report.json`
    );
    document.body.appendChild(downloadAnchor);
    downloadAnchor.click();
    downloadAnchor.remove();
    return "browser_download";
  }

  try {
    return await invoke<string>("export_reconstruction_report", { report });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Reconstruction report export failed: ${message}`);
  }
}

