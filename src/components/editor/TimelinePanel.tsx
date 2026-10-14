import React from "react";
import {
  MagnetIcon,
  TimelineIcon,
  ZoomInIcon,
  ZoomOutIcon,
} from "../ui/Icons";
import { IconButton } from "../ui/IconButton";
import { EmptyState } from "../ui/EmptyState";

export interface TimelinePanelProps {
  snapEnabled: boolean;
  onToggleSnap: () => void;
  timecodeFormat: "timecode" | "frames";
}

export function TimelinePanel({
  snapEnabled,
  onToggleSnap,
  timecodeFormat,
}: TimelinePanelProps): React.ReactElement {
  // Generate sample tick positions for the static ruler placeholder
  const rulerTicks = [
    { label: timecodeFormat === "frames" ? "000 f" : "00:00", percent: "0%" },
    { label: timecodeFormat === "frames" ? "150 f" : "00:05", percent: "12.5%" },
    { label: timecodeFormat === "frames" ? "300 f" : "00:10", percent: "25%" },
    { label: timecodeFormat === "frames" ? "450 f" : "00:15", percent: "37.5%" },
    { label: timecodeFormat === "frames" ? "600 f" : "00:20", percent: "50%" },
    { label: timecodeFormat === "frames" ? "750 f" : "00:25", percent: "62.5%" },
    { label: timecodeFormat === "frames" ? "900 f" : "00:30", percent: "75%" },
    { label: timecodeFormat === "frames" ? "1050 f" : "00:35", percent: "87.5%" },
  ];

  return (
    <section
      aria-label="Timeline Deck"
      className="flex flex-col h-full bg-zinc-950 border-t border-zinc-800/80 overflow-hidden select-none"
    >
      {/* Timeline Toolbar Header */}
      <div className="h-8 shrink-0 flex items-center justify-between px-3 border-b border-zinc-800/70 bg-zinc-900/50">
        <div className="flex items-center space-x-3">
          <div className="flex items-center space-x-2">
            <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
              Timeline
            </span>
            <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-zinc-800 text-zinc-400">
              0 Tracks Active
            </span>
          </div>

          <div className="h-3.5 w-[1px] bg-zinc-800" aria-hidden="true" />

          {/* Snapping Toggle */}
          <IconButton
            icon={<MagnetIcon size={13} />}
            label={snapEnabled ? "Snapping Enabled" : "Snapping Disabled"}
            shortcut="S"
            size="xs"
            variant={snapEnabled ? "active" : "subtle"}
            onClick={onToggleSnap}
          />
        </div>

        {/* Right: Zoom Controls */}
        <div className="flex items-center space-x-1">
          <IconButton
            icon={<ZoomOutIcon size={13} />}
            label="Zoom Out (Disabled — Step 10)"
            shortcut="-"
            size="xs"
            disabled
            aria-disabled="true"
          />
          <IconButton
            icon={<ZoomInIcon size={13} />}
            label="Zoom In (Disabled — Step 10)"
            shortcut="+"
            size="xs"
            disabled
            aria-disabled="true"
          />
        </div>
      </div>

      {/* Time Ruler Bar */}
      <div className="h-6 shrink-0 flex border-b border-zinc-800/70 bg-zinc-900/30 relative">
        {/* Track Header gutter offset */}
        <div className="w-24 shrink-0 border-r border-zinc-800/70 bg-zinc-900/60 flex items-center px-2">
          <span className="text-[10px] font-mono text-zinc-500">RULER</span>
        </div>

        {/* Ruler graduations container */}
        <div className="flex-1 relative overflow-hidden">
          {rulerTicks.map((tick) => (
            <div
              key={tick.label}
              className="absolute top-0 bottom-0 flex flex-col justify-end"
              style={{ left: tick.percent }}
            >
              <span className="text-[9px] font-mono text-zinc-500 mb-0.5 select-none">
                {tick.label}
              </span>
              <div className="h-1.5 w-[1px] bg-zinc-700/60" />
            </div>
          ))}
        </div>
      </div>

      {/* Track Lanes & Empty Timeline Area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Track Headers Column */}
        <div className="w-24 shrink-0 border-r border-zinc-800/70 bg-zinc-900/40 flex flex-col">
          {/* Track V1 */}
          <div className="h-14 border-b border-zinc-800/60 flex items-center justify-between px-2.5 text-xs">
            <span className="font-semibold text-zinc-400">V1</span>
            <span className="text-[10px] text-zinc-500">Video</span>
          </div>

          {/* Track A1 */}
          <div className="h-14 border-b border-zinc-800/60 flex items-center justify-between px-2.5 text-xs">
            <span className="font-semibold text-zinc-400">A1</span>
            <span className="text-[10px] text-zinc-500">Audio</span>
          </div>
        </div>

        {/* Right: Timeline Surface / Empty State */}
        <div className="flex-1 flex flex-col relative overflow-hidden bg-zinc-950/40">
          {/* Subtle Track Grid Lines */}
          <div className="h-14 border-b border-zinc-800/30" />
          <div className="h-14 border-b border-zinc-800/30" />

          {/* Centered Empty State */}
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
            <EmptyState
              icon={<TimelineIcon size={24} />}
              badge="Roadmap: Step 10"
              title="Timeline Empty"
              description="Visual non-destructive timeline, clip interval trimming, and waveform rendering will be implemented in Step 10."
            />
          </div>
        </div>
      </div>
    </section>
  );
}
