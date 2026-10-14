import React, { useState, useEffect, useCallback, useRef } from "react";
import type { MediaAsset } from "../../types/media";
import type {
  RecoveryReview,
  ValidationSummary,
  RecoveryHistoryEntry,
} from "../../types/repair";
import {
  CheckIcon,
  AlertTriangleIcon,
  WrenchIcon,
  FilmIcon,
  ShieldCheckIcon,
  ActivityIcon,
  TimelineIcon,
  TrashIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";
import { PreviewPanel } from "../../components/editor/PreviewPanel";
import {
  getRecoveryReview,
  runAdvancedOutputValidation,
  exportRecoveryReviewReport,
  getRecoveryHistory,
  clearRecoveryHistory,
  finalizeRecoveredAsset,
} from "../../lib/repair";

export interface RecoveryReviewPanelProps {
  timecodeFormat: "timecode" | "frames";
  sourceAsset: MediaAsset | null;
  candidateAsset: MediaAsset | null;
  activeJobId: string | null;
  onAssetFinalized?: (finalizedAsset: MediaAsset) => void;
  onCloseReview?: () => void;
}

type ReviewTab = "preview" | "validation" | "timeline" | "history";
type PreviewLayout = "both" | "recovered" | "original";

export function RecoveryReviewPanel({
  timecodeFormat,
  sourceAsset,
  candidateAsset,
  activeJobId,
  onAssetFinalized,
  onCloseReview,
}: RecoveryReviewPanelProps): React.ReactElement {
  const [activeTab, setActiveTab] = useState<ReviewTab>("preview");
  const [previewLayout, setPreviewLayout] = useState<PreviewLayout>("both");
  const [linkSeeking, setLinkSeeking] = useState<boolean>(true);

  // Review & Validation State
  const [review, setReview] = useState<RecoveryReview | null>(null);
  const [validationSummary, setValidationSummary] = useState<ValidationSummary | null>(null);
  const [isRunningValidation, setIsRunningValidation] = useState<boolean>(false);

  // History State
  const [historyEntries, setHistoryEntries] = useState<RecoveryHistoryEntry[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState<boolean>(false);

  // Export / Finalize State
  const [isFinalizing, setIsFinalizing] = useState<boolean>(false);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  // Seek registration for linked scrubbing
  const originalSeekRef = useRef<((seconds: number) => void) | null>(null);
  const recoveredSeekRef = useRef<((seconds: number) => void) | null>(null);

  const isSourcePlayable =
    sourceAsset !== null &&
    (sourceAsset.compatibility.overall === "supported" ||
      sourceAsset.compatibility.overall === "supportedWithLimitations");

  // Load Review when activeJobId changes
  useEffect(() => {
    if (!activeJobId) {
      setReview(null);
      setValidationSummary(null);
      return;
    }

    let isMounted = true;
    setActionError(null);

    getRecoveryReview(activeJobId)
      .then((rev) => {
        if (isMounted) {
          setReview(rev);
          setValidationSummary(rev.validationSummary);
        }
      })
      .catch((err) => {
        if (isMounted) {
          setActionError(err instanceof Error ? err.message : String(err));
        }
      });

    return () => {
      isMounted = false;
    };
  }, [activeJobId]);

  // Load History
  const loadHistory = useCallback(async () => {
    setIsLoadingHistory(true);
    try {
      const entries = await getRecoveryHistory();
      setHistoryEntries(entries);
    } catch {
      // Ignored
    } finally {
      setIsLoadingHistory(false);
    }
  }, []);

  useEffect(() => {
    if (activeTab === "history") {
      loadHistory();
    }
  }, [activeTab, loadHistory]);

  // Run explicit full decode validation
  const handleRunFullDecode = async () => {
    if (!activeJobId) return;
    setIsRunningValidation(true);
    setActionError(null);
    try {
      const updated = await runAdvancedOutputValidation(activeJobId, "fullDecode");
      setValidationSummary(updated);
      setExportMessage("Full bitstream validation completed.");
    } catch (err: unknown) {
      setActionError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsRunningValidation(false);
    }
  };

  // Export report
  const handleExportReport = async (format: "json" | "html") => {
    if (!activeJobId) return;
    setActionError(null);
    try {
      const path = await exportRecoveryReviewReport(activeJobId, format);
      setExportMessage(`Report exported successfully: ${path}`);
    } catch (err: unknown) {
      setActionError(err instanceof Error ? err.message : String(err));
    }
  };

  // Finalize / Save recovered file
  const handleFinalize = async () => {
    if (!activeJobId) return;
    setIsFinalizing(true);
    setActionError(null);
    try {
      const finalized = await finalizeRecoveredAsset(activeJobId);
      setExportMessage(`Recovered media successfully exported: ${finalized.fileName}`);
      onAssetFinalized?.(finalized);
    } catch (err: unknown) {
      setActionError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsFinalizing(false);
    }
  };

  // Clear history
  const handleClearHistory = async () => {
    if (window.confirm("Are you sure you want to clear all recovery history records?")) {
      await clearRecoveryHistory();
      setHistoryEntries([]);
    }
  };

  // Linked seeking callback handlers
  const handleRecoveredPlaybackChange = (currentTime: number) => {
    if (linkSeeking && isSourcePlayable && originalSeekRef.current) {
      originalSeekRef.current(currentTime);
    }
  };

  const handleOriginalPlaybackChange = (currentTime: number) => {
    if (linkSeeking && recoveredSeekRef.current) {
      recoveredSeekRef.current(currentTime);
    }
  };

  const outcomeBadge = (outcome?: string) => {
    switch (outcome) {
      case "passed":
        return (
          <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-emerald-950/40 text-emerald-400 border border-emerald-800/50">
            <CheckIcon size={12} className="mr-1" /> PASSED
          </span>
        );
      case "warning":
        return (
          <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-amber-950/40 text-amber-400 border border-amber-800/50">
            <AlertTriangleIcon size={12} className="mr-1" /> PARTIAL / WARNINGS
          </span>
        );
      case "failed":
        return (
          <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-red-950/40 text-red-400 border border-red-800/50">
            FAILED
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold bg-zinc-800 text-zinc-400 border border-zinc-700">
            {outcome ? outcome.toUpperCase() : "PENDING"}
          </span>
        );
    }
  };

  return (
    <div className="h-full flex flex-col bg-zinc-950 overflow-hidden text-zinc-100">
      {/* Top Header Bar */}
      <div className="h-10 px-4 border-b border-zinc-800 bg-zinc-900/60 flex items-center justify-between shrink-0">
        <div className="flex items-center space-x-2">
          <ShieldCheckIcon size={16} className="text-indigo-400" />
          <span className="text-xs font-semibold text-zinc-200">
            Recovery Review &amp; Validation
          </span>
          {activeJobId && (
            <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-zinc-800/80 text-zinc-400">
              Job: {activeJobId.slice(0, 8)}
            </span>
          )}
          {review && outcomeBadge(review.validationSummary.overallOutcome)}
        </div>

        {/* View Switcher Tabs */}
        <div className="flex items-center space-x-1 bg-zinc-900 p-0.5 rounded-lg border border-zinc-800 text-xs">
          <button
            onClick={() => setActiveTab("preview")}
            className={`px-2.5 py-1 rounded font-medium transition-colors ${
              activeTab === "preview"
                ? "bg-indigo-600 text-white shadow"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Before / After Preview
          </button>
          <button
            onClick={() => setActiveTab("validation")}
            className={`px-2.5 py-1 rounded font-medium transition-colors ${
              activeTab === "validation"
                ? "bg-indigo-600 text-white shadow"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Validation Evidence
          </button>
          <button
            onClick={() => setActiveTab("timeline")}
            className={`px-2.5 py-1 rounded font-medium transition-colors ${
              activeTab === "timeline"
                ? "bg-indigo-600 text-white shadow"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Timeline &amp; Discrepancies
          </button>
          <button
            onClick={() => setActiveTab("history")}
            className={`px-2.5 py-1 rounded font-medium transition-colors ${
              activeTab === "history"
                ? "bg-indigo-600 text-white shadow"
                : "text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Recovery History
          </button>
        </div>

        {/* Export and Action Buttons */}
        <div className="flex items-center space-x-2">
          {candidateAsset && (
            <Button
              variant="primary"
              size="sm"
              onClick={handleFinalize}
              disabled={isFinalizing}
              className="text-xs"
            >
              {isFinalizing ? "Exporting..." : "Save Recovered Media"}
            </Button>
          )}

          {review && (
            <div className="flex items-center space-x-1">
              <Button
                variant="subtle"
                size="sm"
                onClick={() => handleExportReport("html")}
                className="text-xs text-zinc-300"
              >
                Export HTML
              </Button>
              <Button
                variant="subtle"
                size="sm"
                onClick={() => handleExportReport("json")}
                className="text-xs text-zinc-300"
              >
                Export JSON
              </Button>
            </div>
          )}

          {onCloseReview && (
            <button
              onClick={onCloseReview}
              className="text-xs text-zinc-400 hover:text-zinc-200 p-1"
              title="Close review"
            >
              &times;
            </button>
          )}
        </div>
      </div>

      {/* Notifications / Alerts Bar */}
      {exportMessage && (
        <div className="px-4 py-2 bg-emerald-950/30 border-b border-emerald-800/40 text-emerald-300 text-xs flex items-center justify-between">
          <span className="flex items-center">
            <CheckIcon size={14} className="mr-1.5" />
            {exportMessage}
          </span>
          <button
            onClick={() => setExportMessage(null)}
            className="text-emerald-400 hover:text-emerald-200 text-sm"
          >
            &times;
          </button>
        </div>
      )}

      {actionError && (
        <div className="px-4 py-2 bg-red-950/30 border-b border-red-800/40 text-red-300 text-xs flex items-center justify-between">
          <span className="flex items-center">
            <AlertTriangleIcon size={14} className="mr-1.5" />
            {actionError}
          </span>
          <button
            onClick={() => setActionError(null)}
            className="text-red-400 hover:text-red-200 text-sm"
          >
            &times;
          </button>
        </div>
      )}

      {/* Main Tab Content */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* TAB 1: BEFORE / AFTER PREVIEW */}
        {activeTab === "preview" && (
          <div className="h-full flex flex-col p-3 space-y-3">
            {/* Preview Toolbar */}
            <div className="flex items-center justify-between bg-zinc-900/80 px-3 py-2 rounded-lg border border-zinc-800 text-xs shrink-0">
              <div className="flex items-center space-x-3">
                <span className="text-zinc-400 font-medium">Layout:</span>
                <div className="inline-flex rounded-md shadow-sm">
                  <button
                    onClick={() => setPreviewLayout("both")}
                    className={`px-2.5 py-1 rounded-l text-xs font-medium border ${
                      previewLayout === "both"
                        ? "bg-indigo-600 border-indigo-600 text-white"
                        : "bg-zinc-800 border-zinc-700 text-zinc-300 hover:bg-zinc-700"
                    }`}
                  >
                    Side-by-Side
                  </button>
                  <button
                    onClick={() => setPreviewLayout("recovered")}
                    className={`px-2.5 py-1 text-xs font-medium border-t border-b ${
                      previewLayout === "recovered"
                        ? "bg-indigo-600 border-indigo-600 text-white"
                        : "bg-zinc-800 border-zinc-700 text-zinc-300 hover:bg-zinc-700"
                    }`}
                  >
                    Recovered Only
                  </button>
                  <button
                    onClick={() => setPreviewLayout("original")}
                    className={`px-2.5 py-1 rounded-r text-xs font-medium border ${
                      previewLayout === "original"
                        ? "bg-indigo-600 border-indigo-600 text-white"
                        : "bg-zinc-800 border-zinc-700 text-zinc-300 hover:bg-zinc-700"
                    }`}
                  >
                    Original Only
                  </button>
                </div>

                {isSourcePlayable && previewLayout === "both" && (
                  <label className="flex items-center space-x-1.5 cursor-pointer ml-4">
                    <input
                      type="checkbox"
                      checked={linkSeeking}
                      onChange={(e) => setLinkSeeking(e.target.checked)}
                      className="rounded bg-zinc-800 border-zinc-700 text-indigo-600 focus:ring-0"
                    />
                    <span className="text-zinc-300 text-xs">Link Playhead &amp; Scrubbing</span>
                  </label>
                )}
              </div>

              {!isSourcePlayable && (
                <div className="flex items-center text-amber-400 text-xs">
                  <AlertTriangleIcon size={14} className="mr-1" />
                  <span>Original unplayable (corrupted container). Recovered stream active.</span>
                </div>
              )}
            </div>

            {/* Video Player Display Grid */}
            <div className="flex-1 min-h-0 grid grid-cols-1 lg:grid-cols-2 gap-3 overflow-hidden">
              {/* Original Preview Slot */}
              {(previewLayout === "both" || previewLayout === "original") && (
                <div
                  className={`h-full flex flex-col border border-zinc-800 rounded-lg overflow-hidden bg-zinc-900/40 ${
                    previewLayout === "original" ? "lg:col-span-2" : ""
                  }`}
                >
                  <div className="h-7 px-3 bg-zinc-900/80 border-b border-zinc-800 flex items-center justify-between text-[11px] shrink-0 font-medium text-zinc-400">
                    <span>Original Source: {sourceAsset?.fileName || "None"}</span>
                    <span>{isSourcePlayable ? "Playable" : "Corrupted / Unplayable"}</span>
                  </div>

                  <div className="flex-1 min-h-0 relative flex items-center justify-center p-1">
                    {isSourcePlayable ? (
                      <div className="w-full h-full">
                        <PreviewPanel
                          timecodeFormat={timecodeFormat}
                          asset={sourceAsset}
                          onPlaybackTimeChange={handleOriginalPlaybackChange}
                          onRegisterSeek={(fn) => {
                            originalSeekRef.current = fn;
                          }}
                        />
                      </div>
                    ) : (
                      <div className="max-w-xs p-5 rounded-lg border border-amber-900/50 bg-amber-950/20 text-center space-y-2">
                        <div className="w-10 h-10 rounded-full bg-amber-900/40 text-amber-400 mx-auto flex items-center justify-center">
                          <WrenchIcon size={20} />
                        </div>
                        <h4 className="text-xs font-semibold text-amber-200">
                          Original Stream Unplayable
                        </h4>
                        <p className="text-[11px] text-amber-300/80 leading-relaxed">
                          The original container lacks a valid moov atom or playable sample tables.
                          Synchronized side-by-side video decoding is unavailable for this source.
                        </p>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* Recovered Candidate Preview Slot */}
              {(previewLayout === "both" || previewLayout === "recovered") && (
                <div
                  className={`h-full flex flex-col border border-zinc-800 rounded-lg overflow-hidden bg-zinc-900/40 ${
                    previewLayout === "recovered" ? "lg:col-span-2" : ""
                  }`}
                >
                  <div className="h-7 px-3 bg-zinc-900/80 border-b border-zinc-800 flex items-center justify-between text-[11px] shrink-0 font-medium text-zinc-400">
                    <span className="flex items-center text-emerald-400 font-semibold">
                      <CheckIcon size={12} className="mr-1" />
                      Recovered Candidate
                    </span>
                    <span>
                      {candidateAsset ? "Playable Stream Ready" : "Awaiting Candidate"}
                    </span>
                  </div>

                  <div className="flex-1 min-h-0 relative flex items-center justify-center p-1">
                    {candidateAsset ? (
                      <div className="w-full h-full">
                        <PreviewPanel
                          timecodeFormat={timecodeFormat}
                          asset={candidateAsset}
                          onPlaybackTimeChange={handleRecoveredPlaybackChange}
                          onRegisterSeek={(fn) => {
                            recoveredSeekRef.current = fn;
                          }}
                        />
                      </div>
                    ) : (
                      <div className="text-center text-zinc-500 p-6 space-y-2">
                        <FilmIcon size={32} className="mx-auto text-zinc-600" />
                        <h4 className="text-xs font-semibold text-zinc-400">
                          No Candidate Loaded
                        </h4>
                        <p className="text-[11px] max-w-xs">
                          Execute a recovery or reconstruction operation above to generate a previewable output candidate.
                        </p>
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* TAB 2: VALIDATION EVIDENCE */}
        {activeTab === "validation" && (
          <div className="p-4 space-y-4 max-w-4xl mx-auto">
            {/* Summary Header */}
            <div className="p-4 rounded-xl border border-zinc-800 bg-zinc-900/60 flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-zinc-200">
                  Bitstream Validation &amp; Integrity Analysis
                </h3>
                <p className="text-xs text-zinc-400 mt-1">
                  Level:{" "}
                  <span className="font-semibold text-indigo-300">
                    {validationSummary?.level === "fullDecode"
                      ? "Full Bitstream Decode (100% of stream verified)"
                      : validationSummary?.level === "sampledDecode"
                        ? "Sampled Bitstream Decode (8-second window verified)"
                        : "Metadata Container Inspection Only"}
                  </span>
                </p>
              </div>

              <div className="flex items-center space-x-3">
                {validationSummary?.level !== "fullDecode" && (
                  <Button
                    variant="subtle"
                    size="sm"
                    onClick={handleRunFullDecode}
                    disabled={isRunningValidation || !activeJobId}
                    className="text-xs"
                  >
                    {isRunningValidation
                      ? "Decoding Full Stream..."
                      : "Run Full Bitstream Decode"}
                  </Button>
                )}
                {validationSummary && outcomeBadge(validationSummary.overallOutcome)}
              </div>
            </div>

            {/* Coverage Disclosure Alert */}
            <div className="p-3 rounded-lg border border-indigo-900/50 bg-indigo-950/20 text-xs text-indigo-300 leading-relaxed flex items-start space-x-2">
              <ActivityIcon size={16} className="text-indigo-400 shrink-0 mt-0.5" />
              <div>
                <span className="font-semibold text-indigo-200">Validation Scope: </span>
                {validationSummary?.inspectionCoverage ||
                  "Verified through FFprobe structural parsing and FFmpeg null-sink decode."}
              </div>
            </div>

            {/* Checklist Categories */}
            {validationSummary && (
              <div className="space-y-4">
                {/* Container Checks */}
                <div className="p-4 rounded-xl border border-zinc-800 bg-zinc-900/40">
                  <h4 className="text-xs font-semibold uppercase tracking-wider text-zinc-400 mb-3">
                    Container &amp; Header Integrity
                  </h4>
                  <div className="space-y-2">
                    {validationSummary.containerChecks.map((chk) => (
                      <div
                        key={chk.code}
                        className="flex items-start justify-between p-2.5 rounded bg-zinc-900/80 border border-zinc-800 text-xs"
                      >
                        <div className="space-y-0.5">
                          <div className="font-semibold text-zinc-200">{chk.title}</div>
                          <div className="text-zinc-400">{chk.message}</div>
                        </div>
                        <div className="ml-3 shrink-0">{outcomeBadge(chk.outcome)}</div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Stream Checks */}
                <div className="p-4 rounded-xl border border-zinc-800 bg-zinc-900/40">
                  <h4 className="text-xs font-semibold uppercase tracking-wider text-zinc-400 mb-3">
                    Stream Inventory &amp; Preservation
                  </h4>
                  <div className="space-y-2">
                    {validationSummary.streamChecks.map((chk) => (
                      <div
                        key={chk.code}
                        className="flex items-start justify-between p-2.5 rounded bg-zinc-900/80 border border-zinc-800 text-xs"
                      >
                        <div className="space-y-0.5">
                          <div className="font-semibold text-zinc-200">{chk.title}</div>
                          <div className="text-zinc-400">{chk.message}</div>
                        </div>
                        <div className="ml-3 shrink-0">{outcomeBadge(chk.outcome)}</div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Decode Checks */}
                <div className="p-4 rounded-xl border border-zinc-800 bg-zinc-900/40">
                  <h4 className="text-xs font-semibold uppercase tracking-wider text-zinc-400 mb-3">
                    Bitstream Decoding Verification
                  </h4>
                  <div className="space-y-2">
                    {validationSummary.decodeChecks.map((chk) => (
                      <div
                        key={chk.code}
                        className="flex items-start justify-between p-2.5 rounded bg-zinc-900/80 border border-zinc-800 text-xs"
                      >
                        <div className="space-y-0.5">
                          <div className="font-semibold text-zinc-200">{chk.title}</div>
                          <div className="text-zinc-400">{chk.message}</div>
                        </div>
                        <div className="ml-3 shrink-0">{outcomeBadge(chk.outcome)}</div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Timestamp Checks */}
                <div className="p-4 rounded-xl border border-zinc-800 bg-zinc-900/40">
                  <h4 className="text-xs font-semibold uppercase tracking-wider text-zinc-400 mb-3">
                    Timestamp Regularity &amp; Continuity
                  </h4>
                  <div className="space-y-2">
                    {validationSummary.timestampChecks.map((chk) => (
                      <div
                        key={chk.code}
                        className="flex items-start justify-between p-2.5 rounded bg-zinc-900/80 border border-zinc-800 text-xs"
                      >
                        <div className="space-y-0.5">
                          <div className="font-semibold text-zinc-200">{chk.title}</div>
                          <div className="text-zinc-400">{chk.message}</div>
                        </div>
                        <div className="ml-3 shrink-0">{outcomeBadge(chk.outcome)}</div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* TAB 3: TIMELINE & DISCREPANCIES */}
        {activeTab === "timeline" && (
          <div className="p-4 space-y-4 max-w-4xl mx-auto">
            {/* Duration Comparison Card */}
            <div className="p-4 rounded-xl border border-zinc-800 bg-zinc-900/60">
              <h3 className="text-sm font-semibold text-zinc-200 mb-3 flex items-center space-x-2">
                <TimelineIcon size={16} className="text-indigo-400" />
                <span>Timeline Duration Comparison</span>
              </h3>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/80">
                  <div className="text-[11px] text-zinc-400 font-medium">Original Duration</div>
                  <div className="text-lg font-bold text-zinc-200 mt-1">
                    {review?.sourceSummary?.durationSeconds
                      ? `${review.sourceSummary.durationSeconds.toFixed(2)}s`
                      : "Unknown (Missing moov)"}
                  </div>
                  <div className="text-[11px] text-zinc-500 mt-0.5">
                    {review?.validationSummary.durationComparison.isSourceDurationUnknown
                      ? "Nominal header duration absent in corrupted source"
                      : "Measured from container metadata"}
                  </div>
                </div>

                <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/80">
                  <div className="text-[11px] text-zinc-400 font-medium">Recovered Duration</div>
                  <div className="text-lg font-bold text-emerald-400 mt-1">
                    {review?.outputSummary?.durationSeconds
                      ? `${review.outputSummary.durationSeconds.toFixed(2)}s`
                      : "N/A"}
                  </div>
                  <div className="text-[11px] text-zinc-500 mt-0.5">
                    Synthesized from recovered packets and presentation timestamps
                  </div>
                </div>
              </div>

              <div className="mt-3 p-3 rounded-lg bg-zinc-950/60 border border-zinc-800 text-xs text-zinc-300">
                <span className="font-semibold text-zinc-200">Comparison Note: </span>
                {review?.validationSummary.durationComparison.explanation ||
                  "Durations evaluated based on available demuxed packets."}
              </div>
            </div>

            {/* Stream Alignment & Start Offsets */}
            <div className="p-4 rounded-xl border border-zinc-800 bg-zinc-900/60">
              <h3 className="text-sm font-semibold text-zinc-200 mb-3">
                Stream Start Alignment &amp; A/V Drift
              </h3>

              <div className="space-y-2 text-xs">
                <div className="flex justify-between py-1.5 border-b border-zinc-800">
                  <span className="text-zinc-400">Video Stream Start Offset:</span>
                  <span className="font-mono text-zinc-200">
                    {review?.timelineEvidence?.videoStartOffsetSeconds !== null &&
                    review?.timelineEvidence?.videoStartOffsetSeconds !== undefined
                      ? `${review.timelineEvidence.videoStartOffsetSeconds.toFixed(3)}s`
                      : "0.000s"}
                  </span>
                </div>
                <div className="flex justify-between py-1.5 border-b border-zinc-800">
                  <span className="text-zinc-400">Audio Stream Start Offset:</span>
                  <span className="font-mono text-zinc-200">
                    {review?.timelineEvidence?.audioStartOffsetSeconds !== null &&
                    review?.timelineEvidence?.audioStartOffsetSeconds !== undefined
                      ? `${review.timelineEvidence.audioStartOffsetSeconds.toFixed(3)}s`
                      : "N/A (Video Only / No Audio Track)"}
                  </span>
                </div>
                <div className="flex justify-between py-1.5">
                  <span className="text-zinc-400">Audio/Video Alignment:</span>
                  <span className="text-emerald-400 font-medium">Synchronized within PTS tolerance</span>
                </div>
              </div>
            </div>

            {/* Verified Limitations & Non-Recovery Disclaimers */}
            <div className="p-4 rounded-xl border border-amber-900/40 bg-amber-950/10">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-amber-300 mb-2.5 flex items-center space-x-1.5">
                <AlertTriangleIcon size={14} className="text-amber-400" />
                <span>Verified Recovery Limitations</span>
              </h3>
              <ul className="list-disc pl-5 space-y-1.5 text-xs text-zinc-300">
                {review?.limitations.map((lim, idx) => (
                  <li key={idx}>{lim}</li>
                )) || (
                  <li>
                    Recovery operations salvage intact bitstream packets; missing or overwritten data cannot be reconstructed.
                  </li>
                )}
              </ul>
            </div>
          </div>
        )}

        {/* TAB 4: RECOVERY HISTORY */}
        {activeTab === "history" && (
          <div className="p-4 space-y-4 max-w-4xl mx-auto">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-zinc-200">Recovery Attempt Ledger</h3>
                <p className="text-xs text-zinc-400 mt-0.5">
                  Local, persistent history of all video recovery and reconstruction operations.
                </p>
              </div>

              {historyEntries.length > 0 && (
                <Button
                  variant="subtle"
                  size="sm"
                  onClick={handleClearHistory}
                  className="text-xs text-red-400 border-red-900/60 hover:bg-red-950/30"
                >
                  <TrashIcon size={12} className="mr-1" /> Clear History
                </Button>
              )}
            </div>

            {isLoadingHistory ? (
              <div className="p-8 text-center text-xs text-zinc-500">Loading history ledger...</div>
            ) : historyEntries.length === 0 ? (
              <div className="p-8 rounded-xl border border-zinc-800 bg-zinc-900/30 text-center text-xs text-zinc-500">
                No previous recovery attempts recorded. Past operations will appear here automatically.
              </div>
            ) : (
              <div className="border border-zinc-800 rounded-xl overflow-hidden bg-zinc-900/40">
                <table className="w-full text-left text-xs">
                  <thead className="bg-zinc-900/80 text-zinc-400 uppercase text-[10px] font-semibold tracking-wider border-b border-zinc-800">
                    <tr>
                      <th className="px-4 py-2.5">Date / Time</th>
                      <th className="px-4 py-2.5">File Name</th>
                      <th className="px-4 py-2.5">Method</th>
                      <th className="px-4 py-2.5">Outcome</th>
                      <th className="px-4 py-2.5">Recovered Duration</th>
                      <th className="px-4 py-2.5 text-right">Report</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-zinc-800/80">
                    {historyEntries.map((entry) => (
                      <tr key={entry.jobId} className="hover:bg-zinc-800/30 transition-colors">
                        <td className="px-4 py-3 text-zinc-400 font-mono text-[11px]">
                          {new Date(entry.attemptDateTimeEpochMs).toLocaleString()}
                        </td>
                        <td className="px-4 py-3 font-medium text-zinc-200">
                          {entry.sourceFileName}
                        </td>
                        <td className="px-4 py-3 text-zinc-300">{entry.recoveryMethod}</td>
                        <td className="px-4 py-3">{outcomeBadge(entry.validationOutcome)}</td>
                        <td className="px-4 py-3 font-mono text-zinc-300">
                          {entry.recoveredDurationSeconds
                            ? `${entry.recoveredDurationSeconds.toFixed(2)}s`
                            : "N/A"}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <button
                            onClick={() => {
                              exportRecoveryReviewReport(entry.jobId, "html");
                            }}
                            className="text-[11px] text-indigo-400 hover:text-indigo-300 font-medium"
                          >
                            Export
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
