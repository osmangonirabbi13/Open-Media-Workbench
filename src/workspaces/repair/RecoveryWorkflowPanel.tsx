import React from "react";
import type { MediaAsset } from "../../types/media";
import { CheckIcon, WrenchIcon } from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";

export interface RecoveryWorkflowPanelProps {
  asset: MediaAsset | null;
  activeStep: number;
  onStepChange?: (step: number) => void;
}

export function RecoveryWorkflowPanel({
  asset,
  activeStep = 1,
  onStepChange,
}: RecoveryWorkflowPanelProps): React.ReactElement {
  const steps = [
    { number: 1, title: "Select Media", status: asset ? "complete" : activeStep === 1 ? "current" : "upcoming" },
    { number: 2, title: "Inspect Container", status: asset && activeStep > 2 ? "complete" : activeStep === 2 ? "current" : "upcoming" },
    { number: 3, title: "Choose Recovery Method", status: asset && activeStep > 3 ? "complete" : activeStep === 3 ? "current" : "upcoming" },
    { number: 4, title: "Validate Stream", status: activeStep === 4 ? "current" : "upcoming" },
    { number: 5, title: "Export Salvaged Media", status: activeStep === 5 ? "current" : "upcoming" },
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

      {/* Recovery Methods Selection (Disabled state for STEP 11) */}
      <div className="mt-4 pt-3 border-t border-zinc-800/60 flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center space-x-2">
          <span className="text-xs text-zinc-400 font-medium">Recovery Engine:</span>
          <select
            disabled
            className="text-xs bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-zinc-500 cursor-not-allowed"
          >
            <option>Reference MOOV Reconstruction (Scheduled STEP 13)</option>
            <option>FFmpeg Container Re-indexing (Scheduled STEP 13)</option>
            <option>Raw Stream Demux & Salvage (Scheduled STEP 14)</option>
          </select>
        </div>

        <div className="flex items-center space-x-2">
          <Button
            size="xs"
            variant="secondary"
            disabled
            title="Backend salvage engine will be implemented in STEP 13–14"
          >
            Execute Recovery (Scheduled STEP 13)
          </Button>
        </div>
      </div>
    </div>
  );
}
