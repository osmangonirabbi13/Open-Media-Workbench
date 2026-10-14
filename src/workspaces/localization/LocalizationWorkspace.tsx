import React, { useRef, useState } from "react";
import type { MediaAsset } from "../../types/media";
import { MediaLibraryPanel } from "../../components/editor/MediaLibraryPanel";
import { PreviewPanel } from "../../components/editor/PreviewPanel";
import { TimelinePanel } from "../../components/editor/TimelinePanel";
import { LocalizationInspector } from "./LocalizationInspector";
import { LAYOUT_BOUNDS } from "../../types/preferences";

export interface LocalizationWorkspaceProps {
  mediaPanelWidth: number;
  timelineDeckHeight: number;
  timecodeFormat: "timecode" | "frames";
  snapEnabled: boolean;
  selectedAsset: MediaAsset | null;
  onSelectAsset: (asset: MediaAsset | null) => void;
  onMediaPanelWidthChange: (width: number) => void;
  onTimelineDeckHeightChange: (height: number) => void;
  onToggleSnap: () => void;
}

export function LocalizationWorkspace({
  mediaPanelWidth,
  timelineDeckHeight,
  timecodeFormat,
  snapEnabled,
  selectedAsset,
  onSelectAsset,
  onMediaPanelWidthChange,
  onTimelineDeckHeightChange,
  onToggleSnap,
}: LocalizationWorkspaceProps): React.ReactElement {
  const containerRef = useRef<HTMLDivElement>(null);
  const [inspectorWidth, setInspectorWidth] = useState<number>(320);

  const [isDraggingHorizontal, setIsDraggingHorizontal] = useState(false);
  const [isDraggingVertical, setIsDraggingVertical] = useState(false);
  const [isDraggingInspector, setIsDraggingInspector] = useState(false);

  // Synchronized playback state shared between PreviewPanel and TimelinePanel
  const [playbackTime, setPlaybackTime] = useState<{
    currentTime: number;
    duration: number;
    isPlaying: boolean;
  }>({
    currentTime: 0,
    duration: 0,
    isPlaying: false,
  });

  const seekHandlerRef = useRef<((seconds: number) => void) | null>(null);

  const handleRegisterSeek = (seekFn: (seconds: number) => void) => {
    seekHandlerRef.current = seekFn;
  };

  const handlePlaybackTimeChange = (currentTime: number, duration: number, isPlaying: boolean) => {
    setPlaybackTime({ currentTime, duration, isPlaying });
  };

  const handleTimelineSeek = (seconds: number) => {
    if (seekHandlerRef.current) {
      seekHandlerRef.current(seconds);
    }
  };

  // Horizontal Splitter Dragging (Media Panel Width)
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
        // Ignored
      }
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  // Vertical Splitter Dragging (Timeline Deck Height)
  const handleVerticalPointerDown = (e: React.PointerEvent<HTMLDivElement>): void => {
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    setIsDraggingVertical(true);

    const startY = e.clientY;
    const startHeight = timelineDeckHeight;

    const handlePointerMove = (moveEvent: PointerEvent): void => {
      const delta = startY - moveEvent.clientY;
      onTimelineDeckHeightChange(startHeight + delta);
    };

    const handlePointerUp = (upEvent: PointerEvent): void => {
      setIsDraggingVertical(false);
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      try {
        e.currentTarget.releasePointerCapture(upEvent.pointerId);
      } catch {
        // Ignored
      }
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  // Right Inspector Splitter Dragging
  const handleInspectorPointerDown = (e: React.PointerEvent<HTMLDivElement>): void => {
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    setIsDraggingInspector(true);

    const startX = e.clientX;
    const startWidth = inspectorWidth;

    const handlePointerMove = (moveEvent: PointerEvent): void => {
      // Dragging left increases inspector width (right-anchored)
      const delta = startX - moveEvent.clientX;
      const clamped = Math.min(Math.max(startWidth + delta, 260), 480);
      setInspectorWidth(clamped);
    };

    const handlePointerUp = (upEvent: PointerEvent): void => {
      setIsDraggingInspector(false);
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      try {
        e.currentTarget.releasePointerCapture(upEvent.pointerId);
      } catch {
        // Ignored
      }
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  return (
    <div
      id="workspace-panel-localization"
      role="tabpanel"
      aria-labelledby="workspace-tab-localization"
      ref={containerRef}
      className={`flex-1 flex min-h-0 relative select-none bg-zinc-950 text-zinc-100 ${
        isDraggingHorizontal || isDraggingInspector
          ? "cursor-col-resize"
          : isDraggingVertical
            ? "cursor-row-resize"
            : ""
      }`}
    >
      {/* 1. Left: Media Library Panel */}
      <div
        style={{ width: `${mediaPanelWidth}px` }}
        className="shrink-0 h-full overflow-hidden"
      >
        <MediaLibraryPanel
          selectedAsset={selectedAsset}
          onSelectAsset={onSelectAsset}
        />
      </div>

      {/* Horizontal Splitter (Media Library) */}
      <div
        role="separator"
        tabIndex={0}
        aria-label="Media Library panel width resize handle"
        aria-orientation="vertical"
        aria-valuenow={mediaPanelWidth}
        aria-valuemin={LAYOUT_BOUNDS.MEDIA_MIN_WIDTH}
        aria-valuemax={LAYOUT_BOUNDS.MEDIA_MAX_WIDTH}
        onPointerDown={handleHorizontalPointerDown}
        className={`w-1.5 shrink-0 bg-zinc-900 hover:bg-indigo-600/70 transition-colors cursor-col-resize relative flex items-center justify-center focus-visible:outline-none focus-visible:bg-indigo-500 ${
          isDraggingHorizontal ? "bg-indigo-600 shadow-md shadow-indigo-600/50" : ""
        }`}
      >
        <div className="w-0.5 h-6 rounded-full bg-zinc-700/60 pointer-events-none" />
      </div>

      {/* 2. Center: Video Preview & Waveform Timeline */}
      <div className="flex-1 min-w-0 h-full flex flex-col overflow-hidden">
        {/* Upper Preview Canvas */}
        <div className="flex-1 min-h-0 overflow-hidden">
          <PreviewPanel
            timecodeFormat={timecodeFormat}
            asset={selectedAsset}
            onPlaybackTimeChange={handlePlaybackTimeChange}
            onRegisterSeek={handleRegisterSeek}
          />
        </div>

        {/* Vertical Splitter Handle */}
        <div
          role="separator"
          tabIndex={0}
          aria-label="Timeline deck height resize handle"
          aria-orientation="horizontal"
          aria-valuenow={timelineDeckHeight}
          aria-valuemin={LAYOUT_BOUNDS.TIMELINE_MIN_HEIGHT}
          aria-valuemax={LAYOUT_BOUNDS.TIMELINE_MAX_HEIGHT}
          onPointerDown={handleVerticalPointerDown}
          className={`h-1.5 shrink-0 bg-zinc-900 hover:bg-indigo-600/70 transition-colors cursor-row-resize relative flex items-center justify-center focus-visible:outline-none focus-visible:bg-indigo-500 ${
            isDraggingVertical ? "bg-indigo-600 shadow-md shadow-indigo-600/50" : ""
          }`}
        >
          <div className="h-0.5 w-8 rounded-full bg-zinc-700/60 pointer-events-none" />
        </div>

        {/* Bottom Timeline Deck */}
        <div
          style={{ height: `${timelineDeckHeight}px` }}
          className="shrink-0 overflow-hidden"
        >
          <TimelinePanel
            snapEnabled={snapEnabled}
            onToggleSnap={onToggleSnap}
            timecodeFormat={timecodeFormat}
            asset={selectedAsset}
            currentTime={playbackTime.currentTime}
            duration={playbackTime.duration}
            onSeek={handleTimelineSeek}
          />
        </div>
      </div>

      {/* Right Inspector Splitter */}
      <div
        role="separator"
        tabIndex={0}
        aria-label="Localization inspector width resize handle"
        aria-orientation="vertical"
        aria-valuenow={inspectorWidth}
        aria-valuemin={260}
        aria-valuemax={480}
        onPointerDown={handleInspectorPointerDown}
        className={`w-1.5 shrink-0 bg-zinc-900 hover:bg-indigo-600/70 transition-colors cursor-col-resize relative flex items-center justify-center focus-visible:outline-none focus-visible:bg-indigo-500 ${
          isDraggingInspector ? "bg-indigo-600 shadow-md shadow-indigo-600/50" : ""
        }`}
      >
        <div className="w-0.5 h-6 rounded-full bg-zinc-700/60 pointer-events-none" />
      </div>

      {/* 3. Right: Localization Inspector */}
      <div
        style={{ width: `${inspectorWidth}px` }}
        className="shrink-0 h-full overflow-hidden border-l border-zinc-800/80"
      >
        <LocalizationInspector asset={selectedAsset} />
      </div>
    </div>
  );
}
