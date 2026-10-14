import React, { useRef, useState } from "react";
import { LAYOUT_BOUNDS } from "../../types/preferences";
import type { MediaAsset } from "../../types/media";
import { MediaLibraryPanel } from "../editor/MediaLibraryPanel";
import { PreviewPanel } from "../editor/PreviewPanel";
import { TimelinePanel } from "../editor/TimelinePanel";

export interface ResizableWorkspaceProps {
  mediaPanelWidth: number;
  timelineDeckHeight: number;
  timecodeFormat: "timecode" | "frames";
  snapEnabled: boolean;
  onMediaPanelWidthChange: (width: number) => void;
  onTimelineDeckHeightChange: (height: number) => void;
  onToggleSnap: () => void;
}

export function ResizableWorkspace({
  mediaPanelWidth,
  timelineDeckHeight,
  timecodeFormat,
  snapEnabled,
  onMediaPanelWidthChange,
  onTimelineDeckHeightChange,
  onToggleSnap,
}: ResizableWorkspaceProps): React.ReactElement {
  const containerRef = useRef<HTMLDivElement>(null);
  const [selectedAsset, setSelectedAsset] = useState<MediaAsset | null>(null);
  const [isDraggingHorizontal, setIsDraggingHorizontal] = useState(false);
  const [isDraggingVertical, setIsDraggingVertical] = useState(false);

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
        // Ignored if capture already released
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
      // Moving up increases timeline height (bottom-anchored)
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
        // Ignored if capture already released
      }
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  // Accessible keyboard controls for horizontal splitter
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

  // Accessible keyboard controls for vertical splitter
  const handleVerticalKeyDown = (e: React.KeyboardEvent<HTMLDivElement>): void => {
    const STEP = 16;
    if (e.key === "ArrowUp") {
      e.preventDefault();
      onTimelineDeckHeightChange(timelineDeckHeight + STEP);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      onTimelineDeckHeightChange(timelineDeckHeight - STEP);
    } else if (e.key === "Home") {
      e.preventDefault();
      onTimelineDeckHeightChange(LAYOUT_BOUNDS.TIMELINE_MIN_HEIGHT);
    } else if (e.key === "End") {
      e.preventDefault();
      onTimelineDeckHeightChange(LAYOUT_BOUNDS.TIMELINE_MAX_HEIGHT);
    }
  };

  return (
    <div
      ref={containerRef}
      className={`flex-1 flex flex-col min-h-0 relative select-none ${
        isDraggingHorizontal ? "cursor-col-resize" : isDraggingVertical ? "cursor-row-resize" : ""
      }`}
    >
      {/* Upper Deck: Media Library (Left) + Preview Monitor (Right) */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Media Library Panel */}
        <div
          style={{ width: `${mediaPanelWidth}px` }}
          className="shrink-0 h-full overflow-hidden"
        >
          <MediaLibraryPanel
            selectedAsset={selectedAsset}
            onSelectAsset={setSelectedAsset}
          />
        </div>

        {/* Horizontal Splitter Handle */}
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
          {/* Subtle grab dots */}
          <div className="w-0.5 h-6 rounded-full bg-zinc-700/60 pointer-events-none" />
        </div>

        {/* Preview Monitor Panel */}
        <div className="flex-1 min-w-0 h-full overflow-hidden">
          <PreviewPanel
            timecodeFormat={timecodeFormat}
            asset={selectedAsset}
          />
        </div>
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
        onKeyDown={handleVerticalKeyDown}
        className={`h-1.5 shrink-0 bg-zinc-900 hover:bg-indigo-600/70 transition-colors cursor-row-resize relative flex items-center justify-center focus-visible:outline-none focus-visible:bg-indigo-500 ${
          isDraggingVertical ? "bg-indigo-600 shadow-md shadow-indigo-600/50" : ""
        }`}
      >
        {/* Subtle grab bar */}
        <div className="h-0.5 w-8 rounded-full bg-zinc-700/60 pointer-events-none" />
      </div>

      {/* Bottom Deck: Timeline Panel */}
      <div
        style={{ height: `${timelineDeckHeight}px` }}
        className="shrink-0 overflow-hidden"
      >
        <TimelinePanel
          snapEnabled={snapEnabled}
          onToggleSnap={onToggleSnap}
          timecodeFormat={timecodeFormat}
        />
      </div>
    </div>
  );
}
