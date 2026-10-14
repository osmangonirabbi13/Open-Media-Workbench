import React, { useRef, useEffect, useCallback } from "react";
import type { WaveformSegment, WaveformStatus } from "../../types/waveform";

export interface WaveformCanvasProps {
  segment: WaveformSegment | null;
  status: WaveformStatus;
  currentTimeMicros: number;
  viewportStartMicros: number;
  viewportEndMicros: number;
  durationMicros: number;
  onSeek: (seconds: number) => void;
  className?: string;
}

export function WaveformCanvas({
  segment,
  status,
  currentTimeMicros,
  viewportStartMicros,
  viewportEndMicros,
  durationMicros,
  onSeek,
  className = "",
}: WaveformCanvasProps): React.ReactElement {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Redraw canvas whenever segment, viewport, or playhead updates
  useEffect(() => {
    const container = containerRef.current;
    const canvas = canvasRef.current;
    if (!container || !canvas) return;

    const width = container.clientWidth;
    const height = container.clientHeight;
    if (width <= 0 || height <= 0) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.save();
    ctx.scale(dpr, dpr);

    // 1. Clear background
    ctx.clearRect(0, 0, width, height);

    // 2. Centerline
    const midY = height / 2;
    ctx.strokeStyle = "rgba(63, 63, 70, 0.4)"; // zinc-700
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, midY);
    ctx.lineTo(width, midY);
    ctx.stroke();

    // 3. Render waveform bins if segment is present
    if (segment && segment.bins.length > 0) {
      const bins = segment.bins;
      const binCount = bins.length;
      const spanMicros = Math.max(1, viewportEndMicros - viewportStartMicros);

      // Create rich vertical gradients
      const peakGradient = ctx.createLinearGradient(0, 0, 0, height);
      peakGradient.addColorStop(0, "#818cf8"); // indigo-400
      peakGradient.addColorStop(0.5, "#6366f1"); // indigo-500
      peakGradient.addColorStop(1, "#818cf8");

      const rmsGradient = ctx.createLinearGradient(0, 0, 0, height);
      rmsGradient.addColorStop(0, "#38bdf8"); // sky-400
      rmsGradient.addColorStop(0.5, "#0284c7"); // sky-600
      rmsGradient.addColorStop(1, "#38bdf8");

      const maxAmplitudeScale = midY * 0.92;

      // Draw peaks per horizontal pixel
      for (let px = 0; px < width; px++) {
        // Map pixel x to source time in microseconds
        const tMicros = viewportStartMicros + (px / width) * spanMicros;

        // Relative offset in segment
        const segmentOffsetSec = (tMicros - segment.startMicros) / 1_000_000.0;
        const binIdx = Math.floor(segmentOffsetSec * segment.binsPerSecond);

        if (binIdx >= 0 && binIdx < binCount) {
          const bin = bins[binIdx];

          // Min/Max envelope
          const topY = midY - bin.max * maxAmplitudeScale;
          const bottomY = midY - bin.min * maxAmplitudeScale;
          const barHeight = Math.max(1.5, bottomY - topY);

          ctx.fillStyle = peakGradient;
          ctx.fillRect(px, topY, 1.2, barHeight);

          // RMS core
          const rmsHeight = bin.rms * maxAmplitudeScale * 0.85;
          if (rmsHeight > 0.5) {
            ctx.fillStyle = rmsGradient;
            ctx.fillRect(px, midY - rmsHeight, 1.2, rmsHeight * 2);
          }
        }
      }
    }

    // 4. Render Playhead vertical indicator line
    if (durationMicros > 0) {
      const spanMicros = Math.max(1, viewportEndMicros - viewportStartMicros);
      const playheadRatio = (currentTimeMicros - viewportStartMicros) / spanMicros;
      if (playheadRatio >= 0 && playheadRatio <= 1) {
        const playheadX = playheadRatio * width;

        // Glowing cursor line
        ctx.strokeStyle = "#ef4444"; // red-500
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(playheadX, 0);
        ctx.lineTo(playheadX, height);
        ctx.stroke();

        // Top and bottom accents
        ctx.fillStyle = "#ef4444";
        ctx.fillRect(playheadX - 2, 0, 5, 3);
        ctx.fillRect(playheadX - 2, height - 3, 5, 3);
      }
    }

    ctx.restore();
  }, [segment, currentTimeMicros, viewportStartMicros, viewportEndMicros, durationMicros]);

  // Click & scrub interaction
  const handlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      const container = containerRef.current;
      if (!container || durationMicros <= 0) return;

      e.preventDefault();
      container.setPointerCapture(e.pointerId);

      const updateSeek = (clientX: number) => {
        const rect = container.getBoundingClientRect();
        if (rect.width <= 0) return;
        const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
        const spanMicros = Math.max(1, viewportEndMicros - viewportStartMicros);
        const targetMicros = viewportStartMicros + ratio * spanMicros;
        const clampedSec = Math.max(0, Math.min(targetMicros / 1_000_000, durationMicros / 1_000_000));
        onSeek(clampedSec);
      };

      updateSeek(e.clientX);

      const handlePointerMove = (moveEvent: PointerEvent) => {
        updateSeek(moveEvent.clientX);
      };

      const handlePointerUp = (upEvent: PointerEvent) => {
        try {
          container.releasePointerCapture(upEvent.pointerId);
        } catch {
          // Ignored
        }
        window.removeEventListener("pointermove", handlePointerMove);
        window.removeEventListener("pointerup", handlePointerUp);
      };

      window.addEventListener("pointermove", handlePointerMove);
      window.addEventListener("pointerup", handlePointerUp);
    },
    [durationMicros, viewportStartMicros, viewportEndMicros, onSeek]
  );

  return (
    <div
      ref={containerRef}
      onPointerDown={handlePointerDown}
      className={`relative w-full h-full select-none cursor-pointer overflow-hidden ${className}`}
    >
      <canvas ref={canvasRef} className="w-full h-full block pointer-events-none" />

      {/* Status Overlays */}
      {status.state === "extracting" && (
        <div className="absolute inset-0 flex items-center justify-center bg-zinc-950/50 backdrop-blur-[1px] pointer-events-none">
          <div className="flex items-center space-x-2 px-3 py-1.5 rounded-full bg-zinc-900/90 border border-indigo-500/40 text-xs text-indigo-300 shadow-lg">
            <div className="w-3.5 h-3.5 rounded-full border-2 border-indigo-400 border-t-transparent animate-spin" />
            <span>Analyzing audio waveform...</span>
          </div>
        </div>
      )}

      {status.state === "noAudio" && (
        <div className="absolute inset-0 flex items-center justify-center pointer-events-none text-zinc-500 text-xs font-mono">
          <span>No Audio Stream Found</span>
        </div>
      )}

      {status.state === "failed" && (
        <div className="absolute inset-0 flex items-center justify-center pointer-events-none text-rose-400 text-xs font-medium">
          <span>Waveform analysis failed</span>
        </div>
      )}
    </div>
  );
}
