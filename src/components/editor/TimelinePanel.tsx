import React, { useState, useRef, useEffect, useCallback } from "react";
import {
  MagnetIcon,
  TimelineIcon,
  ZoomInIcon,
  ZoomOutIcon,
  FilmIcon,
  ResetIcon,
} from "../ui/Icons";
import { IconButton } from "../ui/IconButton";
import { EmptyState } from "../ui/EmptyState";
import type { MediaAsset } from "../../types/media";
import type { SubtitleTrack } from "../../types/subtitles";
import { WaveformCanvas } from "../timeline/WaveformCanvas";
import { SubtitleTimelineLane } from "../timeline/SubtitleTimelineLane";
import { useWaveform } from "../../hooks/useWaveform";
import { formatTimecode, formatFrames } from "../../lib/playback";

export interface TimelinePanelProps {
  snapEnabled: boolean;
  onToggleSnap: () => void;
  timecodeFormat: "timecode" | "frames";
  asset: MediaAsset | null;
  currentTime: number;
  duration: number;
  onSeek: (seconds: number) => void;
  subtitleTrack?: SubtitleTrack | null;
  selectedCueIds?: string[];
  onSelectCues?: (cueIds: string[]) => void;
  onMoveCue?: (cueId: string, deltaUs: number) => void;
  onResizeCueStart?: (cueId: string, newStartUs: number) => void;
  onResizeCueEnd?: (cueId: string, newEndUs: number) => void;
}

const ZOOM_LEVELS = [1, 2, 4, 8, 16];

export function TimelinePanel({
  snapEnabled,
  onToggleSnap,
  timecodeFormat,
  asset,
  currentTime,
  duration,
  onSeek,
  subtitleTrack,
  selectedCueIds = [],
  onSelectCues,
  onMoveCue,
  onResizeCueStart,
  onResizeCueEnd,
}: TimelinePanelProps): React.ReactElement {
  const [zoomIdx, setZoomIdx] = useState<number>(0);
  const [timelineWidth, setTimelineWidth] = useState<number>(800);
  const surfaceRef = useRef<HTMLDivElement>(null);

  const rulerRef = useRef<HTMLDivElement>(null);

  const zoomMultiplier = ZOOM_LEVELS[zoomIdx];

  const durationMicros =
    asset?.metadata.container.durationMicros ??
    (duration > 0 ? Math.round(duration * 1_000_000) : 0);

  const currentTimeMicros = Math.round(currentTime * 1_000_000);

  // Viewport time window
  const viewportStartMicros = 0;
  const viewportEndMicros = Math.max(
    1_000_000,
    Math.round(durationMicros / zoomMultiplier)
  );

  // Observe width changes
  useEffect(() => {
    const el = surfaceRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.width > 0) {
          setTimelineWidth(entry.contentRect.width);
        }
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const { status, segment, retryAnalysis, cancelAnalysis } = useWaveform({
    asset,
    viewportStartMicros,
    viewportEndMicros,
    pixelWidth: timelineWidth,
  });

  const fps =
    asset?.metadata.videoStreams[0]?.averageFrameRate?.num
      ? (asset.metadata.videoStreams[0].averageFrameRate.num /
          (asset.metadata.videoStreams[0].averageFrameRate.den || 1))
      : 30;

  // Zoom handlers
  const handleZoomIn = () => {
    if (zoomIdx < ZOOM_LEVELS.length - 1) {
      setZoomIdx((prev) => prev + 1);
    }
  };

  const handleZoomOut = () => {
    if (zoomIdx > 0) {
      setZoomIdx((prev) => prev - 1);
    }
  };

  const handleZoomFit = () => {
    setZoomIdx(0);
  };

  // Generate dynamic ruler ticks across visible viewport
  const tickCount = 8;
  const spanMicros = Math.max(1, viewportEndMicros - viewportStartMicros);
  const rulerTicks = Array.from({ length: tickCount + 1 }, (_, i) => {
    const ratio = i / tickCount;
    const tMicros = viewportStartMicros + ratio * spanMicros;
    const tSec = tMicros / 1_000_000.0;
    const label =
      timecodeFormat === "frames"
        ? formatFrames(tSec, fps)
        : formatTimecode(tSec, fps);
    return {
      label,
      percent: `${(ratio * 100).toFixed(2)}%`,
    };
  });

  // Ruler pointer seek interaction
  const handleRulerPointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      const ruler = rulerRef.current;
      if (!ruler || durationMicros <= 0) return;

      e.preventDefault();
      ruler.setPointerCapture(e.pointerId);

      const updateSeek = (clientX: number) => {
        const rect = ruler.getBoundingClientRect();
        if (rect.width <= 0) return;
        const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
        const targetMicros = viewportStartMicros + ratio * spanMicros;
        const clampedSec = Math.max(
          0,
          Math.min(targetMicros / 1_000_000, durationMicros / 1_000_000)
        );
        onSeek(clampedSec);
      };

      updateSeek(e.clientX);

      const handlePointerMove = (moveEvent: PointerEvent) => {
        updateSeek(moveEvent.clientX);
      };

      const handlePointerUp = (upEvent: PointerEvent) => {
        try {
          ruler.releasePointerCapture(upEvent.pointerId);
        } catch {
          // Ignored
        }
        window.removeEventListener("pointermove", handlePointerMove);
        window.removeEventListener("pointerup", handlePointerUp);
      };

      window.addEventListener("pointermove", handlePointerMove);
      window.addEventListener("pointerup", handlePointerUp);
    },
    [durationMicros, viewportStartMicros, spanMicros, onSeek]
  );

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
              {asset ? (subtitleTrack ? "3 Tracks Active (1 Subtitle)" : "2 Tracks Active (Read-Only)") : "0 Tracks Active"}
            </span>
            {zoomMultiplier > 1 && (
              <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-indigo-950 text-indigo-300 border border-indigo-700/50">
                {zoomMultiplier}&times; Zoom
              </span>
            )}
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
          {zoomMultiplier > 1 && (
            <button
              type="button"
              onClick={handleZoomFit}
              title="Fit timeline to view"
              className="px-2 py-0.5 rounded text-[10px] text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors mr-1"
            >
              Fit
            </button>
          )}
          <IconButton
            icon={<ZoomOutIcon size={13} />}
            label="Zoom Out"
            shortcut="-"
            size="xs"
            disabled={!asset || zoomIdx === 0}
            onClick={handleZoomOut}
          />
          <IconButton
            icon={<ZoomInIcon size={13} />}
            label="Zoom In"
            shortcut="+"
            size="xs"
            disabled={!asset || zoomIdx === ZOOM_LEVELS.length - 1}
            onClick={handleZoomIn}
          />
        </div>
      </div>

      {/* Time Ruler Bar */}
      <div className="h-6 shrink-0 flex border-b border-zinc-800/70 bg-zinc-900/30 relative">
        {/* Track Header gutter offset */}
        <div className="w-24 shrink-0 border-r border-zinc-800/70 bg-zinc-900/60 flex items-center px-2">
          <span className="text-[10px] font-mono text-zinc-500">RULER</span>
        </div>

        {/* Dynamic Ruler graduations */}
        <div
          ref={rulerRef}
          onPointerDown={handleRulerPointerDown}
          className={`flex-1 relative overflow-hidden cursor-pointer ${
            !asset ? "pointer-events-none opacity-40" : ""
          }`}
        >
          {rulerTicks.map((tick, i) => (
            <div
              key={`${tick.label}-${i}`}
              className="absolute top-0 bottom-0 flex flex-col justify-end"
              style={{ left: tick.percent }}
            >
              <span className="text-[9px] font-mono text-zinc-400 mb-0.5 select-none -translate-x-1/2">
                {tick.label}
              </span>
              <div className="h-1.5 w-[1px] bg-zinc-700/60" />
            </div>
          ))}

          {/* Playhead indicator needle on ruler */}
          {durationMicros > 0 && (
            <div
              className="absolute top-0 bottom-0 w-[1.5px] bg-red-500 pointer-events-none z-30"
              style={{
                left: `${((currentTimeMicros - viewportStartMicros) / spanMicros) * 100}%`,
              }}
            >
              <div className="w-2.5 h-2 bg-red-500 -ml-1 rounded-b shadow" />
            </div>
          )}
        </div>
      </div>

      {/* Track Lanes & Waveform Surface */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Track Headers Column */}
        <div className="w-24 shrink-0 border-r border-zinc-800/70 bg-zinc-900/40 flex flex-col">
          {/* Track V1 Header */}
          <div className="h-14 border-b border-zinc-800/60 flex items-center justify-between px-2.5 text-xs">
            <span className="font-semibold text-zinc-300">V1</span>
            <span className="text-[10px] text-zinc-500">Video</span>
          </div>

          {/* Track A1 Header */}
          <div className="h-20 border-b border-zinc-800/60 flex flex-col justify-center px-2.5 text-xs">
            <div className="flex items-center justify-between">
              <span className="font-semibold text-zinc-300">A1</span>
              <span className="text-[10px] text-zinc-500">Audio</span>
            </div>
            <span className="text-[9px] font-mono text-zinc-500 mt-1">16 kHz • MONO</span>
          </div>

          {/* Track S1 Header (Subtitles) */}
          {subtitleTrack && (
            <div className="h-16 border-b border-zinc-800/60 flex flex-col justify-center px-2.5 text-xs bg-indigo-950/20">
              <div className="flex items-center justify-between">
                <span className="font-semibold text-indigo-400">S1</span>
                <span className="text-[10px] text-zinc-500">Subtitle</span>
              </div>
              <span className="text-[9px] font-mono text-zinc-400 mt-1 truncate">
                {subtitleTrack.language.toUpperCase()} • {subtitleTrack.cues.length} cues
              </span>
            </div>
          )}
        </div>

        {/* Right: Timeline Surface */}
        <div
          ref={surfaceRef}
          className="flex-1 flex flex-col relative overflow-hidden bg-zinc-950/40"
        >
          {!asset ? (
            <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
              <EmptyState
                icon={<TimelineIcon size={24} />}
                badge="Timeline Deck"
                title="No Media Loaded"
                description="Import or select a video from the Media Library to display its video track strip and audio waveform."
              />
            </div>
          ) : (
            <>
              {/* Video Track Lane (V1) */}
              <div className="h-14 border-b border-zinc-800/40 relative bg-zinc-900/20 flex items-center px-2">
                <div
                  className="h-10 rounded border border-indigo-700/50 bg-indigo-950/40 flex items-center px-3 space-x-2 overflow-hidden shadow-sm"
                  style={{ width: "100%" }}
                >
                  <FilmIcon size={14} className="text-indigo-400 shrink-0" />
                  <span className="text-xs font-medium text-zinc-200 truncate">
                    {asset.fileName}
                  </span>
                  {asset.metadata.videoStreams[0] && (
                    <span className="text-[10px] font-mono text-indigo-300/80 shrink-0">
                      {asset.metadata.videoStreams[0].width}&times;
                      {asset.metadata.videoStreams[0].height}
                    </span>
                  )}
                </div>

                {/* V1 Playhead line */}
                {durationMicros > 0 && (
                  <div
                    className="absolute top-0 bottom-0 w-[1.5px] bg-red-500 pointer-events-none z-20"
                    style={{
                      left: `${((currentTimeMicros - viewportStartMicros) / spanMicros) * 100}%`,
                    }}
                  />
                )}
              </div>

              {/* Audio Waveform Lane (A1) */}
              <div className="h-20 border-b border-zinc-800/40 relative bg-zinc-950/60 overflow-hidden">
                <WaveformCanvas
                  segment={segment}
                  status={status}
                  currentTimeMicros={currentTimeMicros}
                  viewportStartMicros={viewportStartMicros}
                  viewportEndMicros={viewportEndMicros}
                  durationMicros={durationMicros}
                  onSeek={onSeek}
                />

                {/* Retry action if failed */}
                {status.state === "failed" && (
                  <div className="absolute right-3 top-2 bottom-2 flex items-center space-x-2">
                    <button
                      type="button"
                      onClick={retryAnalysis}
                      className="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-xs font-medium text-zinc-200 transition-colors flex items-center space-x-1"
                    >
                      <ResetIcon size={12} />
                      <span>Retry</span>
                    </button>
                  </div>
                )}

                {/* Cancel action if extracting */}
                {status.state === "extracting" && (
                  <div className="absolute right-3 top-2 bottom-2 flex items-center space-x-2">
                    <button
                      type="button"
                      onClick={cancelAnalysis}
                      className="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-xs font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
                    >
                      Cancel
                    </button>
                  </div>
                )}
              </div>

              {/* Subtitle Track Lane (S1) */}
              {subtitleTrack && (
                <SubtitleTimelineLane
                  track={subtitleTrack}
                  currentTimeMicros={currentTimeMicros}
                  viewportStartMicros={viewportStartMicros}
                  viewportEndMicros={viewportEndMicros}
                  durationMicros={durationMicros}
                  snapEnabled={snapEnabled}
                  selectedCueIds={selectedCueIds}
                  onSelectCues={onSelectCues || (() => {})}
                  onMoveCue={onMoveCue || (() => {})}
                  onResizeCueStart={onResizeCueStart || (() => {})}
                  onResizeCueEnd={onResizeCueEnd || (() => {})}
                  onSeek={onSeek}
                />
              )}
            </>
          )}
        </div>
      </div>

    </section>
  );
}
