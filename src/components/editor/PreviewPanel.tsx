import React from "react";
import {
  MonitorIcon,
  PlayIcon,
  SkipBackIcon,
  SkipForwardIcon,
  StepBackIcon,
  StepForwardIcon,
} from "../ui/Icons";
import { IconButton } from "../ui/IconButton";
import { EmptyState } from "../ui/EmptyState";

export interface PreviewPanelProps {
  timecodeFormat: "timecode" | "frames";
}

export function PreviewPanel({ timecodeFormat }: PreviewPanelProps): React.ReactElement {
  const currentTimecode = timecodeFormat === "frames" ? "00000 f" : "00:00:00:00";
  const durationTimecode = timecodeFormat === "frames" ? "00000 f" : "00:00:00:00";

  return (
    <section
      aria-label="Preview Monitor"
      className="flex flex-col h-full bg-zinc-950 overflow-hidden select-none"
    >
      {/* Panel Header */}
      <div className="h-8 shrink-0 flex items-center justify-between px-3 border-b border-zinc-800/70 bg-zinc-900/40">
        <div className="flex items-center space-x-2">
          <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
            Preview Monitor
          </span>
          <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-zinc-800 text-zinc-400">
            16:9 • 1080p
          </span>
        </div>

        <div className="font-mono text-xs text-zinc-400 px-2 py-0.5 rounded bg-zinc-900 border border-zinc-800/60">
          {currentTimecode}
        </div>
      </div>

      {/* Main Preview Surface (Canvas) */}
      <div className="flex-1 flex items-center justify-center p-3 bg-zinc-950/90 overflow-hidden">
        {/* 16:9 Letterbox Container */}
        <div className="w-full h-full max-w-4xl max-h-full aspect-video rounded-lg border border-zinc-800/70 bg-zinc-900/30 flex items-center justify-center relative shadow-2xl overflow-hidden">
          {/* Subtle Grid Background */}
          <div
            className="absolute inset-0 opacity-5 pointer-events-none"
            style={{
              backgroundImage:
                "linear-gradient(to right, #ffffff 1px, transparent 1px), linear-gradient(to bottom, #ffffff 1px, transparent 1px)",
              backgroundSize: "24px 24px",
            }}
          />

          <EmptyState
            icon={<MonitorIcon size={28} />}
            badge="Roadmap: Step 7"
            title="Preview Surface Inactive"
            description="Real-time HTML5 video playback, frame seeking, and proxy caching will be introduced in Step 7. No active clip loaded."
          />
        </div>
      </div>

      {/* Transport Controls Bar */}
      <div className="h-10 shrink-0 flex items-center justify-between px-4 border-t border-zinc-800/70 bg-zinc-900/60">
        {/* Left: Timecode Readout */}
        <div className="flex items-center space-x-2 text-xs font-mono text-zinc-400">
          <span className="text-zinc-200 font-medium">{currentTimecode}</span>
          <span className="text-zinc-600">/</span>
          <span>{durationTimecode}</span>
        </div>

        {/* Center: Disabled Transport Buttons */}
        <div className="flex items-center space-x-1">
          <IconButton
            icon={<SkipBackIcon size={14} />}
            label="Jump to Start (Disabled — Step 7)"
            shortcut="Home"
            size="sm"
            disabled
            aria-disabled="true"
          />
          <IconButton
            icon={<StepBackIcon size={14} />}
            label="Step Back 1 Frame (Disabled — Step 7)"
            shortcut="Left Arrow"
            size="sm"
            disabled
            aria-disabled="true"
          />
          <IconButton
            icon={<PlayIcon size={14} />}
            label="Play / Pause (Disabled — Step 7)"
            shortcut="Space"
            size="md"
            variant="default"
            disabled
            aria-disabled="true"
            className="text-zinc-500 bg-zinc-800/40"
          />
          <IconButton
            icon={<StepForwardIcon size={14} />}
            label="Step Forward 1 Frame (Disabled — Step 7)"
            shortcut="Right Arrow"
            size="sm"
            disabled
            aria-disabled="true"
          />
          <IconButton
            icon={<SkipForwardIcon size={14} />}
            label="Jump to End (Disabled — Step 7)"
            shortcut="End"
            size="sm"
            disabled
            aria-disabled="true"
          />
        </div>

        {/* Right: Format Info */}
        <div className="text-[11px] text-zinc-500 font-medium">
          CFR 30.00 fps
        </div>
      </div>
    </section>
  );
}
