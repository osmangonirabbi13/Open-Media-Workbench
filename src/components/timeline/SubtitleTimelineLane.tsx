import React, { useState, useRef, useCallback } from "react";

import type { SnapResult, SubtitleCue, SubtitleTrack } from "../../types/subtitles";
import { calculateSnap } from "../../lib/subtitleOperations";

export interface SubtitleTimelineLaneProps {
  track: SubtitleTrack | null;
  currentTimeMicros: number;
  viewportStartMicros: number;
  viewportEndMicros: number;
  durationMicros: number;
  snapEnabled: boolean;
  selectedCueIds: string[];
  onSelectCues: (cueIds: string[]) => void;
  onMoveCue: (cueId: string, deltaUs: number) => void;
  onResizeCueStart: (cueId: string, newStartUs: number) => void;
  onResizeCueEnd: (cueId: string, newEndUs: number) => void;
  onSeek: (seconds: number) => void;
  showWordMarkers?: boolean;
}

type DragMode = "move" | "resizeStart" | "resizeEnd" | null;

export function SubtitleTimelineLane({
  track,
  currentTimeMicros,
  viewportStartMicros,
  viewportEndMicros,
  durationMicros,
  snapEnabled,
  selectedCueIds,
  onSelectCues,
  onMoveCue,
  onResizeCueStart,
  onResizeCueEnd,
  onSeek,
  showWordMarkers = true,
}: SubtitleTimelineLaneProps): React.ReactElement {
  const laneRef = useRef<HTMLDivElement>(null);
  const [activeSnap, setActiveSnap] = useState<SnapResult | null>(null);
  const [dragState, setDragState] = useState<{
    cueId: string;
    mode: DragMode;
    initialPointerX: number;
    initialStartUs: number;
    initialEndUs: number;
    currentDeltaUs: number;
  } | null>(null);

  const spanMicros = Math.max(1, viewportEndMicros - viewportStartMicros);
  const cues = track?.cues || [];

  // Handle pointer down on a cue (body or resize handle)
  const handleCuePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>, cue: SubtitleCue, mode: DragMode) => {
      e.stopPropagation();
      e.preventDefault();

      const lane = laneRef.current;
      if (!lane) return;

      // Update selection
      if (e.metaKey || e.ctrlKey) {
        // Toggle selection
        if (selectedCueIds.includes(cue.id)) {
          onSelectCues(selectedCueIds.filter((id) => id !== cue.id));
        } else {
          onSelectCues([...selectedCueIds, cue.id]);
        }
      } else if (e.shiftKey && selectedCueIds.length > 0) {
        // Range select
        const lastSelected = selectedCueIds[selectedCueIds.length - 1];
        const lastIdx = cues.findIndex((c) => c.id === lastSelected);
        const currIdx = cues.findIndex((c) => c.id === cue.id);
        if (lastIdx !== -1 && currIdx !== -1) {
          const start = Math.min(lastIdx, currIdx);
          const end = Math.max(lastIdx, currIdx);
          const rangeIds = cues.slice(start, end + 1).map((c) => c.id);
          onSelectCues(Array.from(new Set([...selectedCueIds, ...rangeIds])));
        }
      } else if (!selectedCueIds.includes(cue.id)) {
        onSelectCues([cue.id]);
      }

      setDragState({
        cueId: cue.id,
        mode,
        initialPointerX: e.clientX,
        initialStartUs: cue.startUs,
        initialEndUs: cue.endUs,
        currentDeltaUs: 0,
      });

      const handlePointerMove = (moveEv: PointerEvent) => {
        const laneRect = lane.getBoundingClientRect();
        if (laneRect.width <= 0) return;

        const deltaPx = moveEv.clientX - e.clientX;
        const deltaUs = Math.round((deltaPx / laneRect.width) * spanMicros);

        if (mode === "move") {
          let targetStart = cue.startUs + deltaUs;
          if (targetStart < 0) targetStart = 0;

          // Compute snap against playhead and other cues
          const snap = calculateSnap({
            targetTimeUs: targetStart,
            playheadUs: currentTimeMicros,
            cues,
            excludeCueIds: [cue.id],
            viewportStartUs: viewportStartMicros,
            viewportEndUs: viewportEndMicros,
            pixelWidth: laneRect.width,
            snapThresholdPx: 10,
            enabled: snapEnabled,
          });

          setActiveSnap(snap);
          const finalDeltaUs = snap ? snap.snappedTimeUs - cue.startUs : deltaUs;
          setDragState((prev) => (prev ? { ...prev, currentDeltaUs: finalDeltaUs } : null));
        } else if (mode === "resizeStart") {
          let targetStart = cue.startUs + deltaUs;
          if (targetStart < 0) targetStart = 0;
          if (targetStart >= cue.endUs - 100_000) targetStart = cue.endUs - 100_000;

          const snap = calculateSnap({
            targetTimeUs: targetStart,
            playheadUs: currentTimeMicros,
            cues,
            excludeCueIds: [cue.id],
            viewportStartUs: viewportStartMicros,
            viewportEndUs: viewportEndMicros,
            pixelWidth: laneRect.width,
            snapThresholdPx: 10,
            enabled: snapEnabled,
          });

          setActiveSnap(snap);
          const finalStartUs = snap ? snap.snappedTimeUs : targetStart;
          setDragState((prev) =>
            prev ? { ...prev, currentDeltaUs: finalStartUs - cue.startUs } : null
          );
        } else if (mode === "resizeEnd") {
          let targetEnd = cue.endUs + deltaUs;
          if (targetEnd <= cue.startUs + 100_000) targetEnd = cue.startUs + 100_000;

          const snap = calculateSnap({
            targetTimeUs: targetEnd,
            playheadUs: currentTimeMicros,
            cues,
            excludeCueIds: [cue.id],
            viewportStartUs: viewportStartMicros,
            viewportEndUs: viewportEndMicros,
            pixelWidth: laneRect.width,
            snapThresholdPx: 10,
            enabled: snapEnabled,
          });

          setActiveSnap(snap);
          const finalEndUs = snap ? snap.snappedTimeUs : targetEnd;
          setDragState((prev) =>
            prev ? { ...prev, currentDeltaUs: finalEndUs - cue.endUs } : null
          );
        }
      };

      const handlePointerUp = () => {
        window.removeEventListener("pointermove", handlePointerMove);
        window.removeEventListener("pointerup", handlePointerUp);

        setDragState((latest) => {
          if (latest && latest.currentDeltaUs !== 0) {
            if (latest.mode === "move") {
              onMoveCue(latest.cueId, latest.currentDeltaUs);
            } else if (latest.mode === "resizeStart") {
              const newStart = Math.max(0, latest.initialStartUs + latest.currentDeltaUs);
              if (newStart < latest.initialEndUs) {
                onResizeCueStart(latest.cueId, newStart);
              }
            } else if (latest.mode === "resizeEnd") {
              const newEnd = Math.max(
                latest.initialStartUs + 100_000,
                latest.initialEndUs + latest.currentDeltaUs
              );
              onResizeCueEnd(latest.cueId, newEnd);
            }
          }
          return null;
        });

        setActiveSnap(null);
      };

      window.addEventListener("pointermove", handlePointerMove);
      window.addEventListener("pointerup", handlePointerUp);
    },
    [
      cues,
      currentTimeMicros,
      onMoveCue,
      onResizeCueEnd,
      onResizeCueStart,
      onSelectCues,
      selectedCueIds,
      snapEnabled,
      spanMicros,
      viewportEndMicros,
      viewportStartMicros,
    ]
  );

  // Background lane click (deselect or seek)
  const handleLanePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.target === laneRef.current) {
      onSelectCues([]);
      const lane = laneRef.current;
      if (!lane) return;
      const rect = lane.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const targetMicros = viewportStartMicros + ratio * spanMicros;
      onSeek(Math.min(targetMicros, durationMicros) / 1_000_000);
    }
  };

  return (
    <div
      ref={laneRef}
      onPointerDown={handleLanePointerDown}
      className="h-16 relative bg-zinc-950/70 border-b border-zinc-800/60 overflow-hidden select-none cursor-crosshair"
    >
      {/* Background timeline grid tracks */}
      <div className="absolute inset-0 bg-repeat bg-[radial-gradient(#27272a_1px,transparent_1px)] [background-size:16px_16px] opacity-40 pointer-events-none" />

      {/* Snap Indicator Line */}
      {activeSnap && (
        <div
          className="absolute top-0 bottom-0 w-[1px] bg-amber-400 z-40 pointer-events-none shadow-[0_0_8px_rgba(251,191,36,0.8)]"
          style={{
            left: `${((activeSnap.snappedTimeUs - viewportStartMicros) / spanMicros) * 100}%`,
          }}
        >
          <span className="absolute top-1 left-1.5 px-1 py-0.5 rounded text-[9px] font-mono font-bold bg-amber-950 text-amber-200 border border-amber-600/80 shadow">
            {activeSnap.label}
          </span>
        </div>
      )}

      {/* Render Cue Blocks */}
      {cues.map((cue, idx) => {
        let startUs = cue.startUs;
        let endUs = cue.endUs;

        // Apply interactive preview delta if this cue is being dragged
        if (dragState && dragState.cueId === cue.id) {
          if (dragState.mode === "move") {
            startUs = Math.max(0, cue.startUs + dragState.currentDeltaUs);
            endUs = startUs + (cue.endUs - cue.startUs);
          } else if (dragState.mode === "resizeStart") {
            startUs = Math.max(0, cue.startUs + dragState.currentDeltaUs);
          } else if (dragState.mode === "resizeEnd") {
            endUs = Math.max(cue.startUs + 100_000, cue.endUs + dragState.currentDeltaUs);
          }
        }

        // Clip to viewport
        if (endUs < viewportStartMicros || startUs > viewportEndMicros) {
          return null;
        }

        const leftPct = ((startUs - viewportStartMicros) / spanMicros) * 100;
        const widthPct = Math.max(0.4, ((endUs - startUs) / spanMicros) * 100);

        const isSelected = selectedCueIds.includes(cue.id);
        const isActive = cue.startUs <= currentTimeMicros && currentTimeMicros < cue.endUs;

        // Check if cue overlaps previous or next cue
        const prev = idx > 0 ? cues[idx - 1] : null;
        const next = idx < cues.length - 1 ? cues[idx + 1] : null;
        const isOverlapping =
          (prev && prev.endUs > cue.startUs) || (next && next.startUs < cue.endUs);

        return (
          <div
            key={cue.id}
            data-testid={`timeline-cue-${cue.id}`}
            style={{
              left: `${leftPct}%`,
              width: `${widthPct}%`,
            }}
            onPointerDown={(e) => handleCuePointerDown(e, cue, "move")}
            className={`group absolute top-1.5 bottom-1.5 rounded flex items-center justify-between px-1 text-xs cursor-move transition-shadow z-20 border ${
              isSelected
                ? "bg-indigo-600/90 border-indigo-300 text-white shadow-md ring-2 ring-indigo-400/60"
                : isActive
                  ? "bg-indigo-900/80 border-indigo-500 text-indigo-100 shadow-sm"
                  : isOverlapping
                    ? "bg-amber-950/70 border-amber-600/80 text-amber-200"
                    : "bg-zinc-800/90 border-zinc-700/80 text-zinc-300 hover:bg-zinc-700/90"
            }`}
          >
            {/* Left Resize Handle */}
            <div
              onPointerDown={(e) => handleCuePointerDown(e, cue, "resizeStart")}
              className="absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-indigo-400/80 rounded-l flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
              title="Resize cue start time"
            >
              <div className="w-[1.5px] h-3 bg-white/80 rounded-full" />
            </div>

            {/* Word boundary markers when zoomed in */}
            {showWordMarkers && cue.words && cue.words.length > 1 && widthPct >= 5 && (
              <div
                data-testid={`cue-words-${cue.id}`}
                className="absolute inset-0 pointer-events-none overflow-hidden rounded opacity-60 z-0"
              >
                {cue.words.map((w, wIdx) => {
                  const cueDur = endUs - startUs;
                  if (cueDur <= 0) return null;
                  const wLeftPct = ((w.startUs - startUs) / cueDur) * 100;
                  const wWidthPct = ((w.endUs - w.startUs) / cueDur) * 100;
                  return (
                    <div
                      key={w.id || `wm-${wIdx}`}
                      data-testid={`timeline-word-marker-${w.id}`}
                      style={{ left: `${wLeftPct}%`, width: `${wWidthPct}%` }}
                      className="absolute top-0 bottom-0 border-r border-indigo-300/30 flex items-end pb-0.5 px-0.5"
                    >
                      {widthPct >= 20 && (
                        <span className="text-[8px] font-mono opacity-70 truncate select-none leading-none">
                          {w.text}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            )}

            {/* Cue Label / Text snippet */}
            <div className="flex-1 min-w-0 px-2 flex items-center space-x-1.5 pointer-events-none overflow-hidden z-10">
              <span className="font-mono text-[9px] font-bold opacity-70 shrink-0">
                #{idx + 1}
              </span>
              <span className="truncate text-[11px] font-medium leading-none">
                {cue.text.replace(/\n/g, " ")}
              </span>
            </div>

            {/* Right Resize Handle */}
            <div
              onPointerDown={(e) => handleCuePointerDown(e, cue, "resizeEnd")}
              className="absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-indigo-400/80 rounded-r flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
              title="Resize cue end time"
            >
              <div className="w-[1.5px] h-3 bg-white/80 rounded-full" />
            </div>
          </div>
        );
      })}

      {/* Playhead Vertical Line */}
      {durationMicros > 0 && (
        <div
          className="absolute top-0 bottom-0 w-[1.5px] bg-red-500 pointer-events-none z-30"
          style={{
            left: `${((currentTimeMicros - viewportStartMicros) / spanMicros) * 100}%`,
          }}
        />
      )}
    </div>
  );
}
