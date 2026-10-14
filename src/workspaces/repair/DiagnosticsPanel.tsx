import React, { useState, useEffect, useCallback } from "react";
import type { MediaAsset } from "../../types/media";
import { formatFileSize } from "../../types/media";
import type {
  AnalysisMode,
  DiagnosticReport,
  Severity,
  DiagnosticFinding,
} from "../../types/repair";
import {
  runMediaDiagnostics,
  cancelMediaDiagnostics,
  getDiagnosticReport,
  exportDiagnosticReport,
  pickAndImportRepairAsset,
} from "../../lib/repair";
import {
  ActivityIcon,
  FileSearchIcon,
  ShieldCheckIcon,
  WrenchIcon,
  CheckIcon,
  CloseIcon,
  UploadIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";

export interface DiagnosticsPanelProps {
  asset: MediaAsset | null;
  onAssetImported?: (asset: MediaAsset) => void;
  onReportChange?: (report: DiagnosticReport | null) => void;
}

export function DiagnosticsPanel({
  asset,
  onAssetImported,
  onReportChange,
}: DiagnosticsPanelProps): React.ReactElement {
  const [analysisMode, setAnalysisMode] = useState<AnalysisMode>("quick");
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [exportNotice, setExportNotice] = useState<string | null>(null);
  const [severityFilter, setSeverityFilter] = useState<Severity | "all">("all");
  const [currentStage, setCurrentStage] = useState<string>("");

  // Load existing report if available when asset changes
  useEffect(() => {
    let isMounted = true;
    if (!asset) {
      setReport(null);
      onReportChange?.(null);
      return;
    }

    async function loadCached() {
      if (!asset) return;
      const cached = await getDiagnosticReport(asset.id);
      if (isMounted && cached) {
        setReport(cached);
        onReportChange?.(cached);
      } else if (isMounted) {
        setReport(null);
        onReportChange?.(null);
      }
    }
    loadCached();

    return () => {
      isMounted = false;
    };
  }, [asset?.id, onReportChange]);

  // Run diagnostics handler
  const handleStartDiagnostics = useCallback(async () => {
    if (!asset) return;
    setIsRunning(true);
    setErrorMsg(null);
    setExportNotice(null);
    setCurrentStage(
      analysisMode === "deep"
        ? "Running structural checks & bitstream decode inspection..."
        : "Scanning container signature, ISO BMFF hierarchy & stream tracks..."
    );

    try {
      const result = await runMediaDiagnostics(asset.id, analysisMode);
      setReport(result);
      onReportChange?.(result);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMsg(msg);
    } finally {
      setIsRunning(false);
      setCurrentStage("");
    }
  }, [asset, analysisMode, onReportChange]);

  // Cancel handler
  const handleCancelDiagnostics = useCallback(async () => {
    if (!asset) return;
    await cancelMediaDiagnostics(asset.id);
    setIsRunning(false);
    setCurrentStage("");
  }, [asset]);

  // Export report handler
  const handleExportReport = useCallback(async () => {
    if (!report) return;
    try {
      const savedPath = await exportDiagnosticReport(report);
      setExportNotice(`Report saved to: ${savedPath}`);
      setTimeout(() => setExportNotice(null), 6000);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMsg(msg);
    }
  }, [report]);

  // Import repair asset directly from diagnostics panel
  const handleImportDamaged = useCallback(async () => {
    try {
      const imported = await pickAndImportRepairAsset();
      if (imported) {
        onAssetImported?.(imported);
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMsg(msg);
    }
  }, [onAssetImported]);

  if (!asset) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-6 text-center text-zinc-500 space-y-3">
        <FileSearchIcon size={36} className="text-zinc-600" />
        <div>
          <h4 className="text-xs font-semibold text-zinc-300">No Media Asset Selected</h4>
          <p className="text-[11px] mt-1 max-w-xs text-zinc-500">
            Select a candidate from the library or import a potentially damaged file to diagnose.
          </p>
        </div>
        <Button
          size="sm"
          variant="secondary"
          onClick={handleImportDamaged}
          className="flex items-center space-x-1.5"
        >
          <UploadIcon size={14} />
          <span>Import File to Diagnose</span>
        </Button>
      </div>
    );
  }

  // Filter findings
  const findings = report?.findings || [];
  const filteredFindings =
    severityFilter === "all"
      ? findings
      : findings.filter((f) => f.severity === severityFilter);

  const criticalCount = findings.filter((f) => f.severity === "critical").length;
  const errorCount = findings.filter((f) => f.severity === "error").length;
  const warningCount = findings.filter((f) => f.severity === "warning").length;
  const infoCount = findings.filter((f) => f.severity === "info").length;

  return (
    <div className="h-full flex flex-col p-4 overflow-y-auto space-y-4 select-text">
      {/* Target Asset Summary Card */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-2.5">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2 min-w-0">
            <span
              className="text-xs font-semibold text-zinc-100 truncate max-w-[200px]"
              title={asset.fileName}
            >
              {asset.fileName}
            </span>
            <span className="text-[10px] font-mono text-zinc-500">
              ({formatFileSize(asset.source.fileSize)})
            </span>
          </div>

          <div className="flex items-center space-x-1.5">
            {report && (
              <span
                className={`px-2 py-0.5 rounded text-[10px] font-medium border ${
                  report.analysisStatus === "completed"
                    ? criticalCount > 0 || errorCount > 0
                      ? "bg-rose-950/60 text-rose-300 border-rose-800/40"
                      : "bg-emerald-950/60 text-emerald-300 border-emerald-800/40"
                    : report.analysisStatus === "cancelled"
                      ? "bg-zinc-800 text-zinc-400 border-zinc-700"
                      : "bg-amber-950/60 text-amber-300 border-amber-800/40"
                }`}
              >
                {report.analysisStatus.toUpperCase()}
              </span>
            )}
          </div>
        </div>

        {/* Action Controls & Mode Switch */}
        <div className="flex flex-wrap items-center justify-between gap-2 pt-2 border-t border-zinc-800/60">
          <div className="flex items-center space-x-1 bg-zinc-950 p-0.5 rounded border border-zinc-800 text-xs">
            <button
              type="button"
              disabled={isRunning}
              onClick={() => setAnalysisMode("quick")}
              className={`px-2 py-1 rounded text-[11px] font-medium transition-colors ${
                analysisMode === "quick"
                  ? "bg-indigo-600 text-white shadow-sm"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              Quick Scan
            </button>
            <button
              type="button"
              disabled={isRunning}
              onClick={() => setAnalysisMode("deep")}
              className={`px-2 py-1 rounded text-[11px] font-medium transition-colors ${
                analysisMode === "deep"
                  ? "bg-indigo-600 text-white shadow-sm"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              Deep Decode (30s)
            </button>
          </div>

          <div className="flex items-center space-x-2">
            {isRunning ? (
              <Button
                size="xs"
                variant="danger"
                onClick={handleCancelDiagnostics}
                className="flex items-center space-x-1"
              >
                <CloseIcon size={12} />
                <span>Cancel Analysis</span>
              </Button>
            ) : (
              <Button
                size="xs"
                variant="primary"
                onClick={handleStartDiagnostics}
                className="flex items-center space-x-1"
              >
                <ActivityIcon size={12} />
                <span>{report ? "Re-Run Diagnostics" : "Start Diagnostics"}</span>
              </Button>
            )}

            {report && (
              <Button
                size="xs"
                variant="secondary"
                onClick={handleExportReport}
                disabled={isRunning}
                title="Export detailed JSON diagnostic report"
              >
                Export JSON
              </Button>
            )}
          </div>
        </div>

        {/* Live Running Status Banner */}
        {isRunning && (
          <div className="p-2.5 rounded bg-indigo-950/30 border border-indigo-800/40 text-indigo-300 text-xs flex items-center space-x-2 animate-pulse">
            <div className="w-3.5 h-3.5 rounded-full border-2 border-indigo-400 border-t-transparent animate-spin shrink-0" />
            <span className="text-[11px]">{currentStage}</span>
          </div>
        )}

        {/* Error or Export Notice Banner */}
        {errorMsg && (
          <div className="p-2.5 rounded bg-rose-950/40 border border-rose-800/50 text-rose-300 text-xs flex items-start justify-between">
            <span className="text-[11px]">{errorMsg}</span>
            <button
              type="button"
              onClick={() => setErrorMsg(null)}
              className="text-rose-400 hover:text-rose-200 shrink-0 ml-2"
            >
              <CloseIcon size={12} />
            </button>
          </div>
        )}

        {exportNotice && (
          <div className="p-2.5 rounded bg-emerald-950/40 border border-emerald-800/50 text-emerald-300 text-xs flex items-start justify-between">
            <span className="text-[11px]">{exportNotice}</span>
            <button
              type="button"
              onClick={() => setExportNotice(null)}
              className="text-emerald-400 hover:text-emerald-200 shrink-0 ml-2"
            >
              <CloseIcon size={12} />
            </button>
          </div>
        )}
      </div>

      {/* Diagnostics Inspection Coverage Banner */}
      {report && (
        <div className="p-3 rounded-lg border border-zinc-800/80 bg-zinc-900/30 space-y-2 text-xs">
          <div className="flex items-center justify-between text-zinc-400">
            <span className="font-semibold text-zinc-200">Inspection Coverage</span>
            <span className="text-[10px] font-mono text-zinc-500">
              ID: {report.analysisId.slice(0, 16)}...
            </span>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-[11px] font-mono text-zinc-300">
            <div className="p-2 rounded bg-zinc-950/60 border border-zinc-800/60">
              <span className="text-[10px] text-zinc-500 block">Identified Type</span>
              <span className="font-semibold text-zinc-200 truncate block">
                {report.detectedContainer.formatType}
              </span>
            </div>

            <div className="p-2 rounded bg-zinc-950/60 border border-zinc-800/60">
              <span className="text-[10px] text-zinc-500 block">Boxes Scanned</span>
              <span className="font-semibold text-zinc-200">
                {report.inspectionCoverage.boxCountFound}
              </span>
            </div>

            <div className="p-2 rounded bg-zinc-950/60 border border-zinc-800/60">
              <span className="text-[10px] text-zinc-500 block">Stream Probing</span>
              <span
                className={
                  report.inspectionCoverage.probeExecuted
                    ? "text-emerald-400 font-semibold"
                    : "text-amber-400 font-semibold"
                }
              >
                {report.inspectionCoverage.probeExecuted ? "Probed" : "Failed / Absent"}
              </span>
            </div>

            <div className="p-2 rounded bg-zinc-950/60 border border-zinc-800/60">
              <span className="text-[10px] text-zinc-500 block">Decode Sample</span>
              <span
                className={
                  report.inspectionCoverage.decodeSampled
                    ? "text-indigo-300 font-semibold"
                    : "text-zinc-500"
                }
              >
                {report.inspectionCoverage.decodeSampled
                  ? `${report.inspectionCoverage.decodeSampleFrames}s sampled`
                  : "Skipped (Quick)"}
              </span>
            </div>
          </div>

          {report.detectedContainer.signatureDetails && (
            <p className="text-[11px] text-zinc-400 font-mono">
              Signature: {report.detectedContainer.signatureDetails}
            </p>
          )}

          {report.detectedContainer.mismatchWarning && (
            <p className="text-[11px] text-amber-300/90 font-medium">
              Notice: {report.detectedContainer.mismatchWarning}
            </p>
          )}
        </div>
      )}

      {/* Recommended Recovery Actions */}
      {report && report.recommendations.length > 0 && (
        <div className="space-y-2">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-zinc-300 flex items-center space-x-1.5">
            <WrenchIcon size={13} className="text-indigo-400" />
            <span>Evidence-Based Recovery Recommendations</span>
          </h4>

          {report.recommendations.map((rec) => (
            <div
              key={rec.strategyId}
              className="p-3.5 rounded-lg border border-indigo-900/40 bg-indigo-950/20 space-y-2 text-xs"
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold text-indigo-200">{rec.title}</span>
                <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-indigo-900/60 text-indigo-300 border border-indigo-700/50">
                  Target: STEP {rec.targetStep}
                </span>
              </div>
              <p className="text-[11px] text-indigo-200/80 leading-relaxed">
                {rec.description}
              </p>

              <div className="flex flex-wrap gap-1.5 pt-1">
                {rec.requiredEvidence.map((ev, i) => (
                  <span
                    key={i}
                    className="px-2 py-0.5 rounded text-[10px] bg-zinc-900/90 text-zinc-300 border border-zinc-800"
                  >
                    ✓ {ev}
                  </span>
                ))}
              </div>

              {!rec.availableNow && (
                <div className="pt-2 text-[10px] text-zinc-500 font-medium border-t border-indigo-900/30">
                  * Action scheduled for upcoming STEP {rec.targetStep}. Original media remains read-only and untouched.
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Findings List Section */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-zinc-400 flex items-center space-x-1.5">
            <ActivityIcon size={13} className="text-indigo-400" />
            <span>Inspection Findings ({findings.length})</span>
          </h4>

          {findings.length > 0 && (
            <div className="flex items-center space-x-1 text-[10px] font-mono">
              <button
                type="button"
                onClick={() => setSeverityFilter("all")}
                className={`px-1.5 py-0.5 rounded border ${
                  severityFilter === "all"
                    ? "bg-zinc-700 text-white border-zinc-600"
                    : "bg-zinc-900 text-zinc-400 border-zinc-800"
                }`}
              >
                All ({findings.length})
              </button>
              {criticalCount > 0 && (
                <button
                  type="button"
                  onClick={() => setSeverityFilter("critical")}
                  className={`px-1.5 py-0.5 rounded border ${
                    severityFilter === "critical"
                      ? "bg-rose-900 text-rose-100 border-rose-700"
                      : "bg-zinc-900 text-rose-400 border-zinc-800"
                  }`}
                >
                  Critical ({criticalCount})
                </button>
              )}
              {errorCount > 0 && (
                <button
                  type="button"
                  onClick={() => setSeverityFilter("error")}
                  className={`px-1.5 py-0.5 rounded border ${
                    severityFilter === "error"
                      ? "bg-amber-900 text-amber-100 border-amber-700"
                      : "bg-zinc-900 text-amber-400 border-zinc-800"
                  }`}
                >
                  Errors ({errorCount})
                </button>
              )}
              {warningCount > 0 && (
                <button
                  type="button"
                  onClick={() => setSeverityFilter("warning")}
                  className={`px-1.5 py-0.5 rounded border ${
                    severityFilter === "warning"
                      ? "bg-yellow-900 text-yellow-100 border-yellow-700"
                      : "bg-zinc-900 text-yellow-400 border-zinc-800"
                  }`}
                >
                  Warnings ({warningCount})
                </button>
              )}
              {infoCount > 0 && (
                <button
                  type="button"
                  onClick={() => setSeverityFilter("info")}
                  className={`px-1.5 py-0.5 rounded border ${
                    severityFilter === "info"
                      ? "bg-sky-900 text-sky-100 border-sky-700"
                      : "bg-zinc-900 text-sky-400 border-zinc-800"
                  }`}
                >
                  Info ({infoCount})
                </button>
              )}
            </div>
          )}
        </div>

        {report === null ? (
          <div className="p-4 rounded-lg border border-zinc-800/80 bg-zinc-900/30 text-center space-y-2">
            <p className="text-xs text-zinc-400">
              No diagnostics have been executed for this file yet.
            </p>
            <p className="text-[11px] text-zinc-500">
              Select Quick or Deep Scan above and click "Start Diagnostics".
            </p>
          </div>
        ) : filteredFindings.length === 0 ? (
          <div className="p-4 rounded-lg border border-emerald-900/30 bg-emerald-950/10 text-center space-y-1">
            <CheckIcon size={20} className="mx-auto text-emerald-400" />
            <h5 className="text-xs font-semibold text-emerald-300">
              {severityFilter === "all"
                ? "No Issues Detected"
                : `No ${severityFilter} findings`}
            </h5>
            <p className="text-[11px] text-zinc-400">
              Inspection completed with zero items matching the selected filter.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {filteredFindings.map((finding, idx) => (
              <FindingCard key={`${finding.code}-${idx}`} finding={finding} />
            ))}
          </div>
        )}
      </div>

      {/* Safety Notice */}
      <div className="p-3 rounded-lg border border-zinc-800/80 bg-zinc-900/40 text-zinc-400 text-xs flex items-start space-x-2.5">
        <ShieldCheckIcon size={16} className="shrink-0 mt-0.5 text-zinc-500" />
        <div className="space-y-0.5 leading-relaxed">
          <span className="font-semibold text-zinc-300 block">Diagnostic Engine Safety Guarantee</span>
          <p className="text-[11px] text-zinc-500">
            All analysis operations are strictly read-only. Original source files are never altered, overwritten, or moved. No speculative health percentages are displayed.
          </p>
        </div>
      </div>
    </div>
  );
}

function FindingCard({ finding }: { finding: DiagnosticFinding }): React.ReactElement {
  const severityColors = {
    critical: "border-rose-900/60 bg-rose-950/20 text-rose-200",
    error: "border-amber-900/60 bg-amber-950/20 text-amber-200",
    warning: "border-yellow-900/50 bg-yellow-950/20 text-yellow-200",
    info: "border-sky-900/50 bg-sky-950/20 text-sky-200",
  };

  const badgeColors = {
    critical: "bg-rose-900 text-rose-100 border-rose-700",
    error: "bg-amber-900 text-amber-100 border-amber-700",
    warning: "bg-yellow-900 text-yellow-100 border-yellow-700",
    info: "bg-sky-900 text-sky-100 border-sky-700",
  };

  return (
    <div
      className={`p-3 rounded-lg border space-y-1.5 transition-colors ${
        severityColors[finding.severity]
      }`}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center space-x-2 min-w-0">
          <span
            className={`px-1.5 py-0.2 rounded text-[9px] font-mono uppercase font-bold border shrink-0 ${
              badgeColors[finding.severity]
            }`}
          >
            {finding.severity}
          </span>
          <span className="text-xs font-semibold truncate text-zinc-100">
            {finding.summary}
          </span>
        </div>
        <span className="text-[10px] font-mono text-zinc-500 shrink-0">
          {finding.code}
        </span>
      </div>

      <div className="text-[11px] text-zinc-400 font-mono bg-zinc-950/60 p-2 rounded border border-zinc-900">
        <span className="text-zinc-500 block text-[10px] uppercase font-sans font-semibold mb-0.5">
          Observed Evidence:
        </span>
        <span className="text-zinc-300 break-words">{finding.evidence}</span>
      </div>

      {finding.recommendedAction && (
        <div className="text-[11px] text-zinc-300 flex items-start space-x-1.5 pt-0.5">
          <span className="text-indigo-400 font-medium shrink-0">Next Action:</span>
          <span className="text-zinc-400">{finding.recommendedAction}</span>
        </div>
      )}
    </div>
  );
}
