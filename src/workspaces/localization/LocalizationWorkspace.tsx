import React, { useMemo, useRef, useState, useEffect, useCallback } from "react";
import type { MediaAsset } from "../../types/media";
import type { SubtitleDocument, SubtitleOperation } from "../../types/subtitles";
import { MediaLibraryPanel } from "../../components/editor/MediaLibraryPanel";
import { PreviewPanel } from "../../components/editor/PreviewPanel";
import { TimelinePanel } from "../../components/editor/TimelinePanel";
import { LocalizationInspector } from "./LocalizationInspector";
import { LAYOUT_BOUNDS } from "../../types/preferences";
import { findActiveCues } from "../../lib/subtitles";
import { applySubtitleOperation } from "../../lib/subtitleOperations";
import { saveAutosaveSnapshot } from "../../lib/subtitleAutosave";

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

const MAX_UNDO_STACK_SIZE = 50;

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
  const [inspectorWidth, setInspectorWidth] = useState<number>(380);

  const [isDraggingHorizontal, setIsDraggingHorizontal] = useState(false);
  const [isDraggingVertical, setIsDraggingVertical] = useState(false);
  const [isDraggingInspector, setIsDraggingInspector] = useState(false);

  // Subtitle document, track state, and multi-selection
  const [subtitleDoc, setSubtitleDoc] = useState<SubtitleDocument | null>(null);
  const [activeTrackId, setActiveTrackId] = useState<string | null>(null);
  const [subtitlesVisible, setSubtitlesVisible] = useState<boolean>(true);
  const [selectedCueIds, setSelectedCueIds] = useState<string[]>([]);
  const [enableKaraoke, setEnableKaraoke] = useState<boolean>(true);
  const [showWordMarkers, setShowWordMarkers] = useState<boolean>(true);

  // Multi-level undo/redo history stacks
  const [undoStack, setUndoStack] = useState<SubtitleDocument[]>([]);
  const [redoStack, setRedoStack] = useState<SubtitleDocument[]>([]);

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

  // Active track resolved from document
  const activeTrack = useMemo(() => {
    if (!subtitleDoc || subtitleDoc.tracks.length === 0) return null;
    if (activeTrackId) {
      const found = subtitleDoc.tracks.find((t) => t.id === activeTrackId);
      if (found) return found;
    }
    return subtitleDoc.tracks[0] || null;
  }, [subtitleDoc, activeTrackId]);

  // Real-time active cues lookup for video preview caption overlay (O(log N))
  const activeCues = useMemo(() => {
    if (!activeTrack || !activeTrack.isVisible) return [];
    return findActiveCues(activeTrack.cues, playbackTime.currentTime);
  }, [activeTrack, playbackTime.currentTime]);

  const handleToggleTrackVisibility = (trackId: string) => {
    if (!subtitleDoc) return;
    const updatedTracks = subtitleDoc.tracks.map((t) =>
      t.id === trackId ? { ...t, isVisible: !t.isVisible } : t
    );
    setSubtitleDoc({ ...subtitleDoc, tracks: updatedTracks });
  };

  // Applies an operation to the subtitle document with undo recording & autosave
  const handleApplyOperation = useCallback(
    (op: SubtitleOperation) => {
      if (!subtitleDoc) return;

      try {
        const nextDoc = applySubtitleOperation(subtitleDoc, op);

        // Push current state to undo history
        setUndoStack((prev) => [...prev.slice(-MAX_UNDO_STACK_SIZE + 1), subtitleDoc]);
        setRedoStack([]); // Clear redo stack on new action
        setSubtitleDoc(nextDoc);

        // Autosave snapshot
        saveAutosaveSnapshot(nextDoc, selectedAsset?.id);
      } catch (err) {
        console.error("Failed to apply subtitle operation:", err);
      }
    },
    [subtitleDoc, selectedAsset?.id]
  );

  // Undo edit
  const handleUndo = useCallback(() => {
    if (undoStack.length === 0 || !subtitleDoc) return;

    const previousDoc = undoStack[undoStack.length - 1];
    setUndoStack((prev) => prev.slice(0, -1));
    setRedoStack((prev) => [...prev, subtitleDoc]);
    setSubtitleDoc(previousDoc);

    saveAutosaveSnapshot(previousDoc, selectedAsset?.id);
  }, [undoStack, subtitleDoc, selectedAsset?.id]);

  // Redo edit
  const handleRedo = useCallback(() => {
    if (redoStack.length === 0 || !subtitleDoc) return;

    const nextDoc = redoStack[redoStack.length - 1];
    setRedoStack((prev) => prev.slice(0, -1));
    setUndoStack((prev) => [...prev, subtitleDoc]);
    setSubtitleDoc(nextDoc);

    saveAutosaveSnapshot(nextDoc, selectedAsset?.id);
  }, [redoStack, subtitleDoc, selectedAsset?.id]);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Protect native inputs/textareas
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }

      // Undo / Redo
      if (e.ctrlKey || e.metaKey) {
        if (e.key.toLowerCase() === "z") {
          e.preventDefault();
          if (e.shiftKey) {
            handleRedo();
          } else {
            handleUndo();
          }
        } else if (e.key.toLowerCase() === "y") {
          e.preventDefault();
          handleRedo();
        }
      }

      // Delete selected cues
      if (e.key === "Delete" || e.key === "Backspace") {
        if (selectedCueIds.length > 0 && activeTrack) {
          e.preventDefault();
          if (selectedCueIds.length === 1) {
            handleApplyOperation({
              type: "deleteCue",
              payload: { trackId: activeTrack.id, cueId: selectedCueIds[0] },
            });
          } else {
            handleApplyOperation({
              type: "deleteCues",
              payload: { trackId: activeTrack.id, cueIds: selectedCueIds },
            });
          }
          setSelectedCueIds([]);
        }
      }

      // Split shortcut ('c' or Ctrl+\)
      if (
        (e.key.toLowerCase() === "c" && !e.ctrlKey && !e.metaKey) ||
        ((e.ctrlKey || e.metaKey) && e.key === "\\")
      ) {
        if (selectedCueIds.length === 1 && activeTrack) {
          const cue = activeTrack.cues.find((c) => c.id === selectedCueIds[0]);
          const currentMicros = Math.round(playbackTime.currentTime * 1_000_000);
          if (cue && currentMicros > cue.startUs && currentMicros < cue.endUs) {
            e.preventDefault();
            handleApplyOperation({
              type: "splitCue",
              payload: {
                trackId: activeTrack.id,
                cueId: cue.id,
                splitTimeUs: currentMicros,
              },
            });
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    activeTrack,
    handleApplyOperation,
    handleRedo,
    handleUndo,
    playbackTime.currentTime,
    selectedCueIds,
  ]);

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
      try {
        (e.target as HTMLDivElement).releasePointerCapture(upEvent.pointerId);
      } catch {
        // Ignored
      }
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
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
      try {
        (e.target as HTMLDivElement).releasePointerCapture(upEvent.pointerId);
      } catch {
        // Ignored
      }
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  // Inspector Splitter Dragging (Inspector Width)
  const handleInspectorPointerDown = (e: React.PointerEvent<HTMLDivElement>): void => {
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    setIsDraggingInspector(true);

    const startX = e.clientX;
    const startWidth = inspectorWidth;

    const handlePointerMove = (moveEvent: PointerEvent): void => {
      const delta = startX - moveEvent.clientX;
      const newWidth = Math.max(280, Math.min(520, startWidth + delta));
      setInspectorWidth(newWidth);
    };

    const handlePointerUp = (upEvent: PointerEvent): void => {
      setIsDraggingInspector(false);
      try {
        (e.target as HTMLDivElement).releasePointerCapture(upEvent.pointerId);
      } catch {
        // Ignored
      }
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  return (
    <div
      ref={containerRef}
      className="flex h-full w-full bg-zinc-950 overflow-hidden select-none"
    >
      {/* 1. Left: Media Library Panel */}
      <div
        style={{ width: `${mediaPanelWidth}px` }}
        className="shrink-0 h-full overflow-hidden border-r border-zinc-800/80"
      >
        <MediaLibraryPanel
          selectedAsset={selectedAsset}
          onSelectAsset={onSelectAsset}
        />
      </div>

      {/* Horizontal Splitter Handle */}
      <div
        role="separator"
        tabIndex={0}
        aria-label="Media library width resize handle"
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

      {/* 2. Center: Video Preview & Timeline */}
      <div className="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
        {/* Top Video Preview Panel */}
        <div className="flex-1 min-h-0 overflow-hidden">
          <PreviewPanel
            asset={selectedAsset}
            timecodeFormat={timecodeFormat}
            onPlaybackTimeChange={handlePlaybackTimeChange}
            onRegisterSeek={handleRegisterSeek}
            activeSubtitleCues={activeCues}
            subtitlesVisible={subtitlesVisible}
            onToggleSubtitlesVisible={() => setSubtitlesVisible(!subtitlesVisible)}
            enableKaraoke={enableKaraoke}
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
            subtitleTrack={activeTrack}
            selectedCueIds={selectedCueIds}
            onSelectCues={setSelectedCueIds}
            onMoveCue={(cueId, deltaUs) => {
              if (activeTrack) {
                handleApplyOperation({
                  type: "moveCue",
                  payload: { trackId: activeTrack.id, cueId, deltaUs },
                });
              }
            }}
            onResizeCueStart={(cueId, newStartUs) => {
              if (activeTrack) {
                handleApplyOperation({
                  type: "resizeCueStart",
                  payload: { trackId: activeTrack.id, cueId, newStartUs },
                });
              }
            }}
            onResizeCueEnd={(cueId, newEndUs) => {
              if (activeTrack) {
                handleApplyOperation({
                  type: "resizeCueEnd",
                  payload: { trackId: activeTrack.id, cueId, newEndUs },
                });
              }
            }}
            showWordMarkers={showWordMarkers}
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
        aria-valuemin={280}
        aria-valuemax={520}
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
        <LocalizationInspector
          asset={selectedAsset}
          document={subtitleDoc}
          activeTrack={activeTrack}
          currentTime={playbackTime.currentTime}
          selectedCueIds={selectedCueIds}
          onSelectCues={setSelectedCueIds}
          onApplyOperation={handleApplyOperation}
          canUndo={undoStack.length > 0}
          canRedo={redoStack.length > 0}
          onUndo={handleUndo}
          onRedo={handleRedo}
          onDocumentChange={setSubtitleDoc}
          onActiveTrackChange={setActiveTrackId}
          onToggleTrackVisibility={handleToggleTrackVisibility}
          onSeek={handleTimelineSeek}
          enableKaraoke={enableKaraoke}
          onToggleKaraoke={() => setEnableKaraoke(!enableKaraoke)}
          showWordMarkers={showWordMarkers}
          onToggleWordMarkers={() => setShowWordMarkers(!showWordMarkers)}
        />
      </div>
    </div>
  );
}
