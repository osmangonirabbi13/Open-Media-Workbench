import React, { useState } from "react";
import type { MediaAsset } from "../../types/media";
import { MediaLibraryPanel } from "../../components/editor/MediaLibraryPanel";
import { PreviewPanel } from "../../components/editor/PreviewPanel";
import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { RecoveryWorkflowPanel } from "./RecoveryWorkflowPanel";
import { LAYOUT_BOUNDS } from "../../types/preferences";
import { FilmIcon, WrenchIcon } from "../../components/ui/Icons";
import type { DiagnosticReport } from "../../types/repair";

export interface RepairWorkspaceProps {
  mediaPanelWidth: number;
  timecodeFormat: "timecode" | "frames";
  selectedAsset: MediaAsset | null;
  onSelectAsset: (asset: MediaAsset | null) => void;
  onMediaPanelWidthChange: (width: number) => void;
}

export function RepairWorkspace({
  mediaPanelWidth,
  timecodeFormat,
  selectedAsset,
  onSelectAsset,
  onMediaPanelWidthChange,
}: RepairWorkspaceProps): React.ReactElement {
  const [activeWorkflowStep, setActiveWorkflowStep] = useState<number>(1);
  const [currentReport, setCurrentReport] = useState<DiagnosticReport | null>(null);
  const [isDraggingHorizontal, setIsDraggingHorizontal] = useState(false);

  // Sync workflow step when report completes
  const handleReportChange = (report: DiagnosticReport | null) => {
    setCurrentReport(report);
    if (report && report.analysisStatus === "completed") {
      setActiveWorkflowStep(3);
    }
  };

  // Determine playability
  const isPlayable =
    selectedAsset !== null &&
    (selectedAsset.compatibility.overall === "supported" ||
      selectedAsset.compatibility.overall === "supportedWithLimitations");

  // Horizontal splitter handler
  const handleHorizontalPointerDown = (e: React.PointerEvent<HTMLDivElement>): void => {
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    setIsDraggingHorizontal(true);

    const startX = e.clientX;
    const startWidth = mediaPanelWidth;

    const handlePointerMove = (moveEvent: PointerEvent): void => {
      const delta = moveEvent.clientX - startX;
      onMediaPanelWidthChange(startWidth + delta);
    };

    const handlePointerUp = (upEvent: PointerEvent): void => {
      setIsDraggingHorizontal(false);
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      try {
        e.currentTarget.releasePointerCapture(upEvent.pointerId);
      } catch {
        // Ignored if capture already released
      }
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  const handleHorizontalKeyDown = (e: React.KeyboardEvent<HTMLDivElement>): void => {
    const STEP = 16;
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      onMediaPanelWidthChange(mediaPanelWidth - STEP);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      onMediaPanelWidthChange(mediaPanelWidth + STEP);
    } else if (e.key === "Home") {
      e.preventDefault();
      onMediaPanelWidthChange(LAYOUT_BOUNDS.MEDIA_MIN_WIDTH);
    } else if (e.key === "End") {
      e.preventDefault();
      onMediaPanelWidthChange(LAYOUT_BOUNDS.MEDIA_MAX_WIDTH);
    }
  };

  return (
    <div
      id="workspace-panel-repair"
      role="tabpanel"
      aria-labelledby="workspace-tab-repair"
      className="flex-1 flex min-h-0 overflow-hidden bg-zinc-950 text-zinc-100"
    >
      {/* Left: Media Library Panel */}
      <div
        style={{ width: `${mediaPanelWidth}px` }}
        className="shrink-0 h-full overflow-hidden border-r border-zinc-800/80"
      >
        <MediaLibraryPanel
          selectedAsset={selectedAsset}
          onSelectAsset={onSelectAsset}
        />
      </div>

      {/* Horizontal Splitter */}
      <div
        role="separator"
        tabIndex={0}
        aria-label="Media Library panel width resize handle"
        aria-orientation="vertical"
        aria-valuenow={mediaPanelWidth}
        aria-valuemin={LAYOUT_BOUNDS.MEDIA_MIN_WIDTH}
        aria-valuemax={LAYOUT_BOUNDS.MEDIA_MAX_WIDTH}
        onPointerDown={handleHorizontalPointerDown}
        onKeyDown={handleHorizontalKeyDown}
        className={`w-1.5 shrink-0 bg-zinc-900 hover:bg-indigo-600/70 transition-colors cursor-col-resize relative flex items-center justify-center focus-visible:outline-none focus-visible:bg-indigo-500 ${
          isDraggingHorizontal ? "bg-indigo-600 shadow-md shadow-indigo-600/50" : ""
        }`}
      >
        <div className="w-0.5 h-6 rounded-full bg-zinc-700/60 pointer-events-none" />
      </div>

      {/* Main Repair Studio Canvas */}
      <div className="flex-1 min-w-0 h-full flex flex-col overflow-hidden">
        {/* Top Guided Workflow Steps */}
        <RecoveryWorkflowPanel
          asset={selectedAsset}
          activeStep={activeWorkflowStep}
          report={currentReport}
          onStepChange={setActiveWorkflowStep}
        />

        {/* Lower Split: Left Diagnostics & Right Preview Validation */}
        <div className="flex-1 min-h-0 grid grid-cols-1 lg:grid-cols-2 divide-y lg:divide-y-0 lg:divide-x divide-zinc-800 overflow-hidden">
          {/* Diagnostics Inspector */}
          <div className="h-full overflow-hidden flex flex-col">
            <div className="h-8 px-4 border-b border-zinc-800 bg-zinc-900/60 flex items-center justify-between shrink-0">
              <span className="text-xs font-semibold text-zinc-300">Container & Stream Diagnostics</span>
              <span className="text-[10px] text-zinc-500">Inspection View</span>
            </div>
            <div className="flex-1 min-h-0 overflow-y-auto">
              <DiagnosticsPanel
                asset={selectedAsset}
                onAssetImported={onSelectAsset}
                onReportChange={handleReportChange}
              />
            </div>
          </div>

          {/* Validation & Preview Area */}
          <div className="h-full overflow-hidden flex flex-col bg-zinc-950/60">
            <div className="h-8 px-4 border-b border-zinc-800 bg-zinc-900/60 flex items-center justify-between shrink-0">
              <span className="text-xs font-semibold text-zinc-300">Validation & Playback Preview</span>
              <span className="text-[10px] text-zinc-500">
                {isPlayable ? "Playable Stream" : "Awaiting Repair"}
              </span>
            </div>

            <div className="flex-1 min-h-0 relative flex items-center justify-center overflow-hidden p-2">
              {isPlayable ? (
                <div className="w-full h-full">
                  <PreviewPanel
                    timecodeFormat={timecodeFormat}
                    asset={selectedAsset}
                  />
                </div>
              ) : selectedAsset ? (
                <div className="max-w-md p-6 rounded-xl border border-amber-900/50 bg-amber-950/20 text-center space-y-3">
                  <div className="w-12 h-12 rounded-full bg-amber-900/40 text-amber-400 mx-auto flex items-center justify-center">
                    <WrenchIcon size={24} />
                  </div>
                  <h4 className="text-sm font-semibold text-amber-200">Unplayable or Corrupted Container</h4>
                  <p className="text-xs text-amber-300/80 leading-relaxed">
                    This file cannot be decoded directly by the playback bridge. Use the Recovery Workflow above to diagnose missing atom headers and reconstruct a playable stream (Scheduled for STEP 13).
                  </p>
                </div>
              ) : (
                <div className="text-center text-zinc-500 p-6 space-y-2">
                  <FilmIcon size={36} className="mx-auto text-zinc-600" />
                  <h4 className="text-xs font-semibold text-zinc-400">Preview Idle</h4>
                  <p className="text-[11px] max-w-xs">
                    Select a damaged or candidate video file from the library to inspect its contents.
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
