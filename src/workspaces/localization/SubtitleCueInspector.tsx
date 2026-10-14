import React, { useState, useEffect } from "react";
import type { SubtitleCue } from "../../types/subtitles";
import {
  formatMicrosToHmsMs,
  parseHmsMsToMicros,
  calculateReadability,
} from "../../lib/subtitleOperations";
import { Button } from "../../components/ui/Button";
import {
  TrashIcon,
  CopyIcon,
  ShieldAlertIcon,
  SubtitlesIcon,
} from "../../components/ui/Icons";

export interface SubtitleCueInspectorProps {
  cue: SubtitleCue | null;
  selectedCues: SubtitleCue[];
  currentTimeMicros: number;
  onUpdateText: (cueId: string, text: string) => void;
  onUpdateTiming: (cueId: string, startUs: number, endUs: number) => void;
  onNudge: (cueIds: string[], deltaUs: number) => void;
  onSplit: (cueId: string, splitTimeUs: number) => void;
  onMergeWithNext: (firstCueId: string, secondCueId: string) => void;
  onDuplicate: (cueId: string) => void;
  onDeleteCues: (cueIds: string[]) => void;
  onSeek: (seconds: number) => void;
  nextCueId?: string;
}

export function SubtitleCueInspector({
  cue,
  selectedCues,
  currentTimeMicros,
  onUpdateText,
  onUpdateTiming,
  onNudge,
  onSplit,
  onMergeWithNext,
  onDuplicate,
  onDeleteCues,
  onSeek,
  nextCueId,
}: SubtitleCueInspectorProps): React.ReactElement {
  const isMultiSelect = selectedCues.length > 1;

  // Local draft state for timing inputs
  const [startInput, setStartInput] = useState("");
  const [endInput, setEndInput] = useState("");
  const [textInput, setTextInput] = useState("");
  const [timingError, setTimingError] = useState<string | null>(null);

  useEffect(() => {
    if (cue) {
      setStartInput(formatMicrosToHmsMs(cue.startUs));
      setEndInput(formatMicrosToHmsMs(cue.endUs));
      setTextInput(cue.text);
      setTimingError(null);
    }
  }, [cue?.id, cue?.startUs, cue?.endUs, cue?.text]);

  if (!cue && selectedCues.length === 0) {
    return (
      <div className="p-4 rounded-lg border border-dashed border-zinc-800 text-center text-zinc-500 text-xs">
        Select a subtitle cue on the timeline or list to edit timing and dialogue.
      </div>
    );
  }

  // Multi-cue batch editing view
  if (isMultiSelect) {
    const selectedIds = selectedCues.map((c) => c.id);
    return (
      <div className="p-3 bg-zinc-900/60 rounded-lg border border-indigo-900/50 space-y-3 text-xs">
        <div className="flex items-center justify-between">
          <span className="font-semibold text-indigo-300">
            {selectedCues.length} Cues Selected
          </span>
          <Button
            size="xs"
            variant="danger"
            onClick={() => onDeleteCues(selectedIds)}
            className="flex items-center space-x-1"
          >
            <TrashIcon size={12} />
            <span>Delete Batch</span>
          </Button>
        </div>

        <div className="space-y-1.5">
          <span className="text-[11px] text-zinc-400">Batch Nudge Timing:</span>
          <div className="grid grid-cols-4 gap-1.5">
            <button
              type="button"
              onClick={() => onNudge(selectedIds, -500_000)}
              className="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono"
            >
              -500ms
            </button>
            <button
              type="button"
              onClick={() => onNudge(selectedIds, -100_000)}
              className="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono"
            >
              -100ms
            </button>
            <button
              type="button"
              onClick={() => onNudge(selectedIds, 100_000)}
              className="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono"
            >
              +100ms
            </button>
            <button
              type="button"
              onClick={() => onNudge(selectedIds, 500_000)}
              className="px-2 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[11px] font-mono"
            >
              +500ms
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (!cue) return <></>;

  const readability = calculateReadability(cue);
  const canSplitAtPlayhead =
    currentTimeMicros > cue.startUs && currentTimeMicros < cue.endUs;

  const handleCommitTiming = () => {
    setTimingError(null);
    const parsedStart = parseHmsMsToMicros(startInput);
    const parsedEnd = parseHmsMsToMicros(endInput);

    if (parsedStart === null) {
      setTimingError("Invalid start time format (use HH:MM:SS.mmm)");
      return;
    }
    if (parsedEnd === null) {
      setTimingError("Invalid end time format (use HH:MM:SS.mmm)");
      return;
    }
    if (parsedStart < 0) {
      setTimingError("Start time cannot be negative");
      return;
    }
    if (parsedEnd <= parsedStart) {
      setTimingError("End time must be strictly greater than start time");
      return;
    }

    onUpdateTiming(cue.id, parsedStart, parsedEnd);
  };

  const handleCommitText = () => {
    if (textInput !== cue.text) {
      onUpdateText(cue.id, textInput);
    }
  };

  const handleSetStartFromPlayhead = () => {
    if (currentTimeMicros >= cue.endUs) {
      setTimingError("Playhead is at or after cue end time");
      return;
    }
    onUpdateTiming(cue.id, currentTimeMicros, cue.endUs);
  };

  const handleSetEndFromPlayhead = () => {
    if (currentTimeMicros <= cue.startUs) {
      setTimingError("Playhead is at or before cue start time");
      return;
    }
    onUpdateTiming(cue.id, cue.startUs, currentTimeMicros);
  };

  return (
    <div className="p-3 bg-zinc-900/60 rounded-lg border border-zinc-800 space-y-3 text-xs select-none">
      {/* Header Info & Quick Actions */}
      <div className="flex items-center justify-between pb-2 border-b border-zinc-800/80">
        <div className="flex items-center space-x-1.5">
          <SubtitlesIcon size={14} className="text-indigo-400" />
          <span className="font-semibold text-zinc-200">Cue Inspector</span>
          <span className="text-[10px] font-mono text-zinc-500">({cue.id.slice(0, 8)})</span>
        </div>

        <div className="flex items-center space-x-1">
          <Button
            size="xs"
            variant="subtle"
            onClick={() => onDuplicate(cue.id)}
            title="Duplicate cue"
            className="p-1"
          >

            <CopyIcon size={13} />
          </Button>
          <Button
            size="xs"
            variant="danger"
            onClick={() => onDeleteCues([cue.id])}
            title="Delete cue"
            className="p-1"
          >
            <TrashIcon size={13} />
          </Button>
        </div>
      </div>

      {/* Start / End Timing Controls */}
      <div className="space-y-1.5">
        <div className="grid grid-cols-2 gap-2 font-mono">
          <div>
            <label className="text-[10px] text-zinc-400 block mb-0.5">Start Time</label>
            <div className="flex space-x-1">
              <input
                type="text"
                value={startInput}
                onChange={(e) => setStartInput(e.target.value)}
                onBlur={handleCommitTiming}
                onKeyDown={(e) => e.key === "Enter" && handleCommitTiming()}
                className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
              />
              <button
                type="button"
                onClick={() => onSeek(cue.startUs / 1_000_000)}
                title="Jump playhead to cue start"
                className="px-1.5 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-[10px] text-zinc-300"
              >
                Go
              </button>
            </div>
          </div>

          <div>
            <label className="text-[10px] text-zinc-400 block mb-0.5">End Time</label>
            <div className="flex space-x-1">
              <input
                type="text"
                value={endInput}
                onChange={(e) => setEndInput(e.target.value)}
                onBlur={handleCommitTiming}
                onKeyDown={(e) => e.key === "Enter" && handleCommitTiming()}
                className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
              />
              <button
                type="button"
                onClick={() => onSeek(cue.endUs / 1_000_000)}
                title="Jump playhead to cue end"
                className="px-1.5 py-1 bg-zinc-800 hover:bg-zinc-700 rounded text-[10px] text-zinc-300"
              >
                Go
              </button>
            </div>
          </div>
        </div>

        {timingError && (
          <div className="text-[10px] text-rose-400 bg-rose-950/40 p-1 rounded border border-rose-900/60 flex items-center space-x-1">
            <ShieldAlertIcon size={12} className="shrink-0" />
            <span>{timingError}</span>
          </div>
        )}

        {/* Playhead Sync Buttons & Duration */}
        <div className="flex items-center justify-between text-[11px] pt-1">
          <div className="flex items-center space-x-1">
            <button
              type="button"
              onClick={handleSetStartFromPlayhead}
              className="px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-300 text-[10px]"
              title="Set cue start time to current playhead"
            >
              Start @ Playhead
            </button>
            <button
              type="button"
              onClick={handleSetEndFromPlayhead}
              className="px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-300 text-[10px]"
              title="Set cue end time to current playhead"
            >
              End @ Playhead
            </button>
          </div>

          <span className="font-mono text-[10px] text-zinc-400">
            {readability.durationSeconds.toFixed(2)}s
          </span>
        </div>
      </div>

      {/* Nudge Controls */}
      <div className="pt-1 border-t border-zinc-800/80">
        <label className="text-[10px] text-zinc-400 block mb-1">Nudge Timing</label>
        <div className="grid grid-cols-4 gap-1">
          <button
            type="button"
            onClick={() => onNudge([cue.id], -500_000)}
            className="px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[10px] font-mono text-center"
          >
            -500ms
          </button>
          <button
            type="button"
            onClick={() => onNudge([cue.id], -100_000)}
            className="px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[10px] font-mono text-center"
          >
            -100ms
          </button>
          <button
            type="button"
            onClick={() => onNudge([cue.id], 100_000)}
            className="px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[10px] font-mono text-center"
          >
            +100ms
          </button>
          <button
            type="button"
            onClick={() => onNudge([cue.id], 500_000)}
            className="px-1.5 py-0.5 bg-zinc-800 hover:bg-zinc-700 rounded text-zinc-200 text-[10px] font-mono text-center"
          >
            +500ms
          </button>
        </div>
      </div>

      {/* Multiline Dialogue Text Editor */}
      <div className="pt-1 border-t border-zinc-800/80 space-y-1">
        <div className="flex items-center justify-between">
          <label className="text-[10px] text-zinc-400">Subtitle Text</label>
          <span className="text-[10px] font-mono text-zinc-500">
            {readability.charCount} chars • {readability.charsPerSecond} cps
          </span>
        </div>

        <textarea
          rows={3}
          value={textInput}
          onChange={(e) => setTextInput(e.target.value)}
          onBlur={handleCommitText}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              handleCommitText();
            }
          }}
          className="w-full bg-zinc-950 border border-zinc-700 rounded p-2 text-xs text-zinc-100 focus:outline-none focus:border-indigo-500 font-sans leading-relaxed select-text"
          placeholder="Enter dialogue or subtitle text (Bengali & Unicode supported)..."
        />

        {readability.warning && (
          <div className="text-[10px] text-amber-400 bg-amber-950/30 px-2 py-0.5 rounded border border-amber-800/40">
            {readability.warning}
          </div>
        )}
      </div>

      {/* Split & Merge Operations */}
      <div className="pt-2 border-t border-zinc-800/80 flex items-center space-x-2">
        <Button
          size="xs"
          variant="secondary"
          onClick={() => onSplit(cue.id, currentTimeMicros)}
          disabled={!canSplitAtPlayhead}
          title={
            canSplitAtPlayhead
              ? "Split cue at current playhead position"
              : "Position playhead inside cue interval to split"
          }
          className="flex-1"
        >
          Split @ Playhead
        </Button>

        {nextCueId && (
          <Button
            size="xs"
            variant="secondary"
            onClick={() => onMergeWithNext(cue.id, nextCueId)}
            title="Merge with adjacent next cue"
            className="flex-1"
          >
            Merge Next
          </Button>
        )}
      </div>
    </div>
  );
}
