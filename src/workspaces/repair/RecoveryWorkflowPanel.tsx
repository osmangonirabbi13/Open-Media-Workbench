import React, { useState, useEffect } from "react";
import type { MediaAsset } from "../../types/media";
import type {
  DiagnosticReport,
  RecoveryMethod,
  RecoveryReport,
  SalvageStreamType,
} from "../../types/repair";
import { CheckIcon, WrenchIcon, AlertTriangleIcon } from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";
import {
  startMediaRecovery,
  getRecoveryStatus,
  cancelMediaRecovery,
  finalizeRecoveredAsset,
  exportRecoveryReport,
} from "../../lib/repair";

export interface RecoveryWorkflowPanelProps {
  asset: MediaAsset | null;
  activeStep: number;
  report?: DiagnosticReport | null;
  onStepChange?: (step: number) => void;
  onAssetRecovered?: (newAsset: MediaAsset) => void;
}

export function RecoveryWorkflowPanel({
  asset,
  activeStep = 1,
  report,
  onStepChange,
  onAssetRecovered,
}: RecoveryWorkflowPanelProps): React.ReactElement {
  const isInspected = report !== null && report !== undefined;

  // Selected Recovery Configurations
  const [selectedMethod, setSelectedMethod] = useState<RecoveryMethod>("losslessRemux");
  const [outputContainer, setOutputContainer] = useState<string>("mp4");
  const [salvageStream, setSalvageStream] = useState<SalvageStreamType>("allReadable");

  // Job Execution State
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [stageMessage, setStageMessage] = useState<string>("");
  const [recoveryReport, setRecoveryReport] = useState<RecoveryReport | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [exportSuccessMessage, setExportSuccessMessage] = useState<string | null>(null);

  // Check if missing moov atom (unfinalized MP4/MOV)
  const isMissingMoov = Boolean(
    report?.findings.some(
      (f) => f.code === "MOOV_ATOM_MISSING" || f.code === "CONTAINER_PROBE_FAILED"
    ) &&
    report?.recommendations.some(
      (r) => r.strategyId === "REFERENCE_BASED_RECONSTRUCTION"
    )
  );

  // Check recommendations to auto-select best strategy
  useEffect(() => {
    if (report?.recommendations && report.recommendations.length > 0) {
      const topRec = report.recommendations[0];
      if (topRec.strategyId === "TIMESTAMP_REBUILD") {
        setSelectedMethod("timestampRebuild");
      } else if (topRec.strategyId === "STREAM_SALVAGE" || topRec.strategyId === "RAW_STREAM_SALVAGE") {
        setSelectedMethod("streamSalvage");
      } else if (topRec.strategyId === "CONTAINER_REMUX" || topRec.strategyId === "NO_REPAIR_NEEDED") {
        setSelectedMethod("losslessRemux");
      }
    }
  }, [report]);

  // Status Polling when running
  useEffect(() => {
    let interval: NodeJS.Timeout | null = null;
    if (isRunning && activeJobId) {
      interval = setInterval(async () => {
        try {
          const status = await getRecoveryStatus(activeJobId);
          if (status) {
            setStageMessage(status.stageMessage);
            if (
              status.status === "completed" ||
              status.status === "partial" ||
              status.status === "failed" ||
              status.status === "cancelled"
            ) {
              setIsRunning(false);
              if (status.report) {
                setRecoveryReport(status.report);
                onStepChange?.(4); // Advance to Validate Stream
              }
              if (status.errorMessage) {
                setErrorMessage(status.errorMessage);
              }
            }
          }
        } catch (err: unknown) {
          console.error("Failed to poll recovery status:", err);
        }
      }, 750);
    }
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [isRunning, activeJobId, onStepChange]);

  // Handle Recovery Start
  const handleStartRecovery = async () => {
    if (!asset) return;
    setErrorMessage(null);
    setExportSuccessMessage(null);
    setIsRunning(true);
    setStageMessage("Initiating recovery job in application staging...");

    try {
      const jobStatus = await startMediaRecovery({
        assetId: asset.id,
        method: selectedMethod,
        outputContainerPreference: outputContainer,
        salvageStreamType: selectedMethod === "streamSalvage" ? salvageStream : null,
        diagnosticReportId: report?.analysisId || null,
      });

      setActiveJobId(jobStatus.jobId);
      setStageMessage(jobStatus.stageMessage);
      if (jobStatus.report) {
        setRecoveryReport(jobStatus.report);
        setIsRunning(false);
        onStepChange?.(4);
      }
    } catch (err: unknown) {
      setIsRunning(false);
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(msg);
    }
  };

  // Handle Cancellation
  const handleCancel = async () => {
    if (!activeJobId) return;
    try {
      await cancelMediaRecovery(activeJobId);
      setIsRunning(false);
      setStageMessage("Recovery cancelled by user.");
    } catch (err) {
      console.error("Cancel failed:", err);
    }
  };

  // Handle Finalize & Save Recovered Media
  const handleFinalize = async () => {
    if (!activeJobId) return;
    setIsExporting(true);
    setErrorMessage(null);
    try {
      const newAsset = await finalizeRecoveredAsset(activeJobId);
      setIsExporting(false);
      setExportSuccessMessage(`Media successfully exported and authorized: ${newAsset.fileName}`);
      onAssetRecovered?.(newAsset);
      onStepChange?.(5); // Advance to Export Salvaged Media step
    } catch (err: unknown) {
      setIsExporting(false);
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(msg);
    }
  };

  // Handle JSON Report Export
  const handleExportReport = async () => {
    if (!recoveryReport) return;
    try {
      await exportRecoveryReport(recoveryReport);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(`Report export failed: ${msg}`);
    }
  };

  const steps = [
    {
      number: 1,
      title: "Select Media",
      status: asset ? "complete" : activeStep === 1 ? "current" : "upcoming",
    },
    {
      number: 2,
      title: "Inspect Container",
      status: isInspected ? "complete" : asset ? "current" : "upcoming",
    },
    {
      number: 3,
      title: "Choose Recovery Method",
      status: isInspected && !recoveryReport ? "current" : isInspected ? "complete" : "upcoming",
    },
    {
      number: 4,
      title: "Validate Stream",
      status: recoveryReport ? "complete" : isRunning ? "current" : "upcoming",
    },
    {
      number: 5,
      title: "Export Salvaged Media",
      status: exportSuccessMessage ? "complete" : activeStep === 5 ? "current" : "upcoming",
    },
  ];

  return (
    <div className="p-4 border-b border-zinc-800 bg-zinc-900/40 select-none">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-300 flex items-center space-x-1.5">
          <WrenchIcon size={14} className="text-indigo-400" />
          <span>Guided Media Recovery Workflow</span>
        </h3>
        <span className="text-[11px] text-zinc-500 font-mono">
          {asset ? `Target: ${asset.fileName}` : "Awaiting File Selection"}
        </span>
      </div>

      {/* Stepper Indicator */}
      <ol className="grid grid-cols-2 sm:grid-cols-5 gap-2">
        {steps.map((step) => {
          const isDone = step.status === "complete";
          const isCurrent = step.status === "current";

          return (
            <li
              key={step.number}
              onClick={() => onStepChange?.(step.number)}
              className={`flex items-center space-x-2 p-2 rounded-lg border text-xs transition-colors ${
                isDone
                  ? "border-emerald-800/40 bg-emerald-950/20 text-emerald-300 cursor-pointer"
                  : isCurrent
                    ? "border-indigo-600/50 bg-indigo-950/30 text-indigo-200"
                    : "border-zinc-800/60 bg-zinc-900/20 text-zinc-500"
              }`}
            >
              <div
                className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0 ${
                  isDone
                    ? "bg-emerald-500 text-zinc-950"
                    : isCurrent
                      ? "bg-indigo-600 text-white"
                      : "bg-zinc-800 text-zinc-500"
                }`}
              >
                {isDone ? <CheckIcon size={12} /> : step.number}
              </div>
              <span className="truncate text-[11px] font-medium">{step.title}</span>
            </li>
          );
        })}
      </ol>

      {/* Recovery Configuration & Execution Area */}
      <div className="mt-4 pt-3 border-t border-zinc-800/60">
        {isMissingMoov ? (
          <div className="p-3 rounded-lg border border-amber-800/40 bg-amber-950/20 text-xs text-amber-200 flex items-start space-x-2.5">
            <AlertTriangleIcon size={16} className="text-amber-400 shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-amber-300">
                Unfinalized Recording Missing Index Atom (moov)
              </p>
              <p className="mt-1 text-zinc-400 leading-relaxed">
                Standard stream copy remux cannot demux packets without track sample tables.
                Reference-based reconstruction using a donor file recorded on the same device is
                scheduled for <strong>STEP 14</strong>. Standard remux has been conservatively disabled.
              </p>
            </div>
          </div>
        ) : (
          <div className="flex flex-col lg:flex-row items-start lg:items-center justify-between gap-3">
            {/* Controls */}
            <div className="flex flex-wrap items-center gap-3">
              {/* Method Selection */}
              <div className="flex items-center space-x-2">
                <span className="text-xs text-zinc-400 font-medium">Recovery Method:</span>
                <select
                  disabled={isRunning || !asset}
                  className="text-xs bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-zinc-200 focus:outline-none focus:border-indigo-500 disabled:opacity-50"
                  value={selectedMethod}
                  onChange={(e) => setSelectedMethod(e.target.value as RecoveryMethod)}
                >
                  <option value="losslessRemux">Lossless Container Remux (Stream Copy)</option>
                  <option value="streamSalvage">Recoverable Stream Salvage</option>
                  <option value="timestampRebuild">Timestamp Continuity Rebuild (genpts)</option>
                </select>
              </div>

              {/* Stream Salvage Sub-option */}
              {selectedMethod === "streamSalvage" && (
                <div className="flex items-center space-x-2">
                  <span className="text-xs text-zinc-400 font-medium">Stream Target:</span>
                  <select
                    disabled={isRunning}
                    className="text-xs bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-200 focus:outline-none focus:border-indigo-500"
                    value={salvageStream}
                    onChange={(e) => setSalvageStream(e.target.value as SalvageStreamType)}
                  >
                    <option value="allReadable">All Readable Tracks</option>
                    <option value="videoOnly">Video Only (Drop Audio)</option>
                    <option value="audioOnly">Audio Only (Drop Video)</option>
                  </select>
                </div>
              )}

              {/* Output Container Extension */}
              <div className="flex items-center space-x-2">
                <span className="text-xs text-zinc-400 font-medium">Container:</span>
                <select
                  disabled={isRunning}
                  className="text-xs bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-200 focus:outline-none focus:border-indigo-500"
                  value={outputContainer}
                  onChange={(e) => setOutputContainer(e.target.value)}
                >
                  <option value="mp4">MP4 (.mp4)</option>
                  <option value="mkv">Matroska (.mkv)</option>
                  <option value="mov">QuickTime (.mov)</option>
                  <option value="avi">AVI (.avi)</option>
                </select>
              </div>
            </div>

            {/* Action Buttons */}
            <div className="flex items-center space-x-2 self-end lg:self-auto">
              {isRunning ? (
                <Button
                  size="xs"
                  variant="danger"
                  onClick={handleCancel}
                >
                  Cancel Recovery
                </Button>
              ) : (
                <Button
                  size="xs"
                  variant="primary"
                  disabled={!asset || isMissingMoov}
                  onClick={handleStartRecovery}
                >
                  Execute Recovery
                </Button>
              )}
            </div>
          </div>
        )}

        {/* Real-time Progress & Status Banner */}
        {isRunning && (
          <div className="mt-3 p-2.5 rounded bg-indigo-950/30 border border-indigo-800/40 flex items-center justify-between text-xs text-indigo-200">
            <div className="flex items-center space-x-2">
              <div className="w-3.5 h-3.5 border-2 border-indigo-400 border-t-transparent rounded-full animate-spin" />
              <span>{stageMessage || "Processing recovery pipeline..."}</span>
            </div>
            <span className="text-[11px] font-mono text-indigo-400">Isolated Staging</span>
          </div>
        )}

        {/* Error Notification */}
        {errorMessage && (
          <div className="mt-3 p-2.5 rounded bg-red-950/30 border border-red-800/40 text-xs text-red-300 flex items-center justify-between">
            <span>{errorMessage}</span>
            <button
              onClick={() => setErrorMessage(null)}
              className="text-red-400 hover:text-red-200 text-[10px] ml-2 underline"
            >
              Dismiss
            </button>
          </div>
        )}

        {/* Export Success Notification */}
        {exportSuccessMessage && (
          <div className="mt-3 p-2.5 rounded bg-emerald-950/30 border border-emerald-800/40 text-xs text-emerald-300 flex items-center justify-between">
            <div className="flex items-center space-x-1.5">
              <CheckIcon size={14} className="text-emerald-400" />
              <span>{exportSuccessMessage}</span>
            </div>
            <button
              onClick={() => setExportSuccessMessage(null)}
              className="text-emerald-400 hover:text-emerald-200 text-[10px] ml-2 underline"
            >
              Dismiss
            </button>
          </div>
        )}

        {/* Validated Output Result Card */}
        {recoveryReport && (
          <div className="mt-3 p-3 rounded-lg border border-zinc-800 bg-zinc-950/80 text-xs">
            <div className="flex flex-wrap items-center justify-between gap-2 border-b border-zinc-800 pb-2">
              <div className="flex items-center space-x-2">
                <span
                  className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider ${
                    recoveryReport.processingStatus === "completed"
                      ? "bg-emerald-950 border border-emerald-700/60 text-emerald-300"
                      : recoveryReport.processingStatus === "partial"
                        ? "bg-amber-950 border border-amber-700/60 text-amber-300"
                        : "bg-red-950 border border-red-700/60 text-red-300"
                  }`}
                >
                  {recoveryReport.processingStatus}
                </span>
                <span className="font-semibold text-zinc-200">
                  {recoveryReport.validation.passed ? "Playable Stream Validated" : "Validation Incomplete"}
                </span>
              </div>
              <span className="font-mono text-[11px] text-zinc-400">
                Job ID: {recoveryReport.jobId.slice(0, 8)}...
              </span>
            </div>

            <p className="mt-2 text-zinc-300 leading-relaxed">
              {recoveryReport.validation.details}
            </p>

            <div className="mt-2.5 grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] font-mono text-zinc-400 bg-zinc-900/50 p-2 rounded border border-zinc-800/60">
              <div>
                <span className="text-zinc-500 block text-[10px]">OUTPUT FORMAT</span>
                <span className="text-zinc-200">{recoveryReport.actualOutputContainer.toUpperCase()}</span>
              </div>
              <div>
                <span className="text-zinc-500 block text-[10px]">FILE SIZE</span>
                <span className="text-zinc-200">
                  {recoveryReport.outputFileSizeBytes
                    ? `${(recoveryReport.outputFileSizeBytes / 1024).toFixed(1)} KB`
                    : "N/A"}
                </span>
              </div>
              <div>
                <span className="text-zinc-500 block text-[10px]">DURATION</span>
                <span className="text-zinc-200">
                  {recoveryReport.outputDurationSeconds
                    ? `${recoveryReport.outputDurationSeconds.toFixed(2)}s`
                    : "N/A"}
                </span>
              </div>
              <div>
                <span className="text-zinc-500 block text-[10px]">STREAMS</span>
                <span className="text-zinc-200">{recoveryReport.outputStreams.length} Track(s)</span>
              </div>
            </div>

            {/* Action Bar for Validated Output */}
            <div className="mt-3 pt-2 border-t border-zinc-800/60 flex flex-wrap items-center justify-between gap-2">
              <Button
                size="xs"
                variant="subtle"
                onClick={handleExportReport}
              >
                Export Recovery Report (JSON)
              </Button>

              <div className="flex items-center space-x-2">
                <Button
                  size="xs"
                  variant="primary"
                  disabled={isExporting || !recoveryReport.validation.isPlayable}
                  onClick={handleFinalize}
                >
                  {isExporting ? "Exporting..." : "Save / Finalize Recovered Media"}
                </Button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
