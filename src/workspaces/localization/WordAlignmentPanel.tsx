import React, { useState, useMemo } from "react";
import type {
  AlignedWord,
  AlignmentResult,
  SegmentationPolicy,
  TimingQuality,
} from "../../types/alignment";
import { DEFAULT_SEGMENTATION_POLICY } from "../../types/alignment";
import type { SubtitleCue, SubtitleDocument, SubtitleTrack } from "../../types/subtitles";
import {
  segmentWordsToCues,
  refineSubtitleCues,
} from "../../lib/alignment";
import { Button } from "../../components/ui/Button";
import {
  SparklesIcon,
  CheckIcon,
  AlertTriangleIcon,
  PlayIcon,
  ShieldAlertIcon,
  ActivityIcon,
  WrenchIcon,
  FileSearchIcon,
} from "../../components/ui/Icons";

export interface WordAlignmentPanelProps {
  alignmentResult: AlignmentResult | null;
  activeTrack?: SubtitleTrack | null;
  document?: SubtitleDocument | null;
  onApplyTrack: (doc: SubtitleDocument) => void;
  onSeek?: (seconds: number) => void;
  enableKaraoke: boolean;
  onToggleKaraoke: () => void;
  showWordMarkers: boolean;
  onToggleWordMarkers: () => void;
  currentTimeUs?: number;
}

export function WordAlignmentPanel({
  alignmentResult,
  activeTrack,
  document,
  onApplyTrack,
  onSeek,
  enableKaraoke,
  onToggleKaraoke,
  showWordMarkers,
  onToggleWordMarkers,
}: WordAlignmentPanelProps): React.ReactElement {
  const [policy, setPolicy] = useState<SegmentationPolicy>(DEFAULT_SEGMENTATION_POLICY);
  const [previewCues, setPreviewCues] = useState<SubtitleCue[] | null>(null);
  const [isSegmenting, setIsSegmenting] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [selectedWord, setSelectedWord] = useState<AlignedWord | null>(null);
  const [selectedCueId, setSelectedCueId] = useState<string | null>(null);
  const [wordEditStartMs, setWordEditStartMs] = useState<string>("");
  const [wordEditEndMs, setWordEditEndMs] = useState<string>("");
  const [editError, setEditError] = useState<string | null>(null);
  const [trackNameInput, setTrackNameInput] = useState<string>("");

  // Active words source: either from the alignment result or from selected active track's cues
  const words: AlignedWord[] = useMemo(() => {
    if (alignmentResult?.words && alignmentResult.words.length > 0) {
      return alignmentResult.words;
    }
    if (activeTrack?.cues) {
      const extracted: AlignedWord[] = [];
      for (const cue of activeTrack.cues) {
        if (cue.words) {
          extracted.push(...cue.words);
        }
      }
      return extracted;
    }
    return [];
  }, [alignmentResult, activeTrack]);

  const timingQuality: TimingQuality = useMemo(() => {
    if (alignmentResult) {
      return alignmentResult.timingQuality;
    }
    if (words.length > 0) {
      const hasUnaligned = words.some((w) => w.provenance === "unaligned");
      return hasUnaligned ? "partiallyAligned" : "estimated";
    }
    return "unavailable";
  }, [alignmentResult, words]);

  const alignedCount = useMemo(
    () => words.filter((w) => w.provenance !== "unaligned").length,
    [words]
  );
  const unalignedCount = words.length - alignedCount;

  // Handle generating readable cues
  const handleGenerateCues = async () => {
    if (words.length === 0) return;
    setIsSegmenting(true);
    setEditError(null);
    try {
      const lang = alignmentResult?.language || activeTrack?.language || "en";
      let cues = await segmentWordsToCues(words, policy, lang);
      cues = await refineSubtitleCues(cues, policy.minCueDurationUs);
      setPreviewCues(cues);
    } catch (err) {
      setEditError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSegmenting(false);
    }
  };

  // Handle word selection for manual inspection/editing
  const handleSelectWord = (word: AlignedWord, cueId?: string) => {
    setSelectedWord(word);
    setSelectedCueId(cueId || null);
    setWordEditStartMs((word.startUs / 1000).toFixed(0));
    setWordEditEndMs((word.endUs / 1000).toFixed(0));
    setEditError(null);
  };

  // Handle manual timing adjustment save
  const handleSaveWordTiming = async () => {
    if (!selectedWord) return;
    const startMs = parseFloat(wordEditStartMs);
    const endMs = parseFloat(wordEditEndMs);

    if (isNaN(startMs) || isNaN(endMs)) {
      setEditError("Timestamps must be valid numeric millisecond values.");
      return;
    }
    if (startMs < 0) {
      setEditError("Start timestamp cannot be negative.");
      return;
    }
    if (endMs <= startMs) {
      setEditError("End timestamp must be strictly greater than start timestamp.");
      return;
    }

    const startUs = Math.round(startMs * 1000);
    const endUs = Math.round(endMs * 1000);

    // Update in preview cues if available
    if (previewCues && selectedCueId) {
      const updated = previewCues.map((c) => {
        if (c.id === selectedCueId && c.words) {
          const updatedWords = c.words.map((w) =>
            w.id === selectedWord.id
              ? { ...w, startUs, endUs, provenance: "userAdjusted" as const }
              : w
          );
          return { ...c, words: updatedWords };
        }
        return c;
      });
      setPreviewCues(updated);
    }

    setSelectedWord({
      ...selectedWord,
      startUs,
      endUs,
      provenance: "userAdjusted",
    });
    setEditError(null);
  };

  // Apply segmented cues to project
  const handleApplyToSubtitles = async () => {
    if (!previewCues || previewCues.length === 0) return;
    setIsApplying(true);
    try {
      const lang = alignmentResult?.language || activeTrack?.language || "en";
      const displayName =
        trackNameInput.trim() || `Smart Subtitles (${lang})`;

      const track: SubtitleTrack = {
        id: `track-smart-${Date.now()}`,
        displayName,
        language: lang,
        sourceFormat: "nativeProject",
        cues: previewCues,
        isEnabled: true,
        isVisible: true,
        schemaVersion: 1,
      };

      const baseDoc: SubtitleDocument = document || {
        schemaVersion: 1,
        id: `doc-${Date.now()}`,
        tracks: [],
        createdAtEpochMs: Date.now(),
        updatedAtEpochMs: Date.now(),
      };

      const newDoc: SubtitleDocument = {
        ...baseDoc,
        tracks: [...baseDoc.tracks, track],
        activeTrackId: track.id,
        updatedAtEpochMs: Date.now(),
      };

      onApplyTrack(newDoc);
      setPreviewCues(null);
    } catch (err) {
      setEditError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsApplying(false);
    }
  };

  const getQualityBadge = (quality: TimingQuality) => {
    switch (quality) {
      case "aligned":
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-950 text-emerald-300 border border-emerald-800/80">
            <CheckIcon size={12} />
            <span>Aligned (Measured)</span>
          </span>
        );
      case "estimated":
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] font-semibold bg-indigo-950 text-indigo-300 border border-indigo-800/80">
            <ActivityIcon size={12} />
            <span>Whisper Native</span>
          </span>
        );
      case "partiallyAligned":
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-950 text-amber-300 border border-amber-800/80">
            <AlertTriangleIcon size={12} />
            <span>Partially Aligned</span>
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[10px] font-semibold bg-zinc-800 text-zinc-400 border border-zinc-700">
            <span>Unavailable</span>
          </span>
        );
    }
  };

  return (
    <div
      data-testid="word-alignment-panel"
      className="p-4 space-y-4 text-zinc-200 text-xs overflow-y-auto max-h-full"
    >
      {/* Header & Quality Status */}
      <div className="flex items-center justify-between border-b border-zinc-800 pb-3">
        <div className="flex items-center space-x-2">
          <SparklesIcon size={16} className="text-indigo-400" />
          <h3 className="font-semibold text-sm text-zinc-100">
            Word Alignment & Smart Segmentation
          </h3>
        </div>
        {getQualityBadge(timingQuality)}
      </div>

      {/* Summary Metrics */}
      <div className="grid grid-cols-3 gap-2 bg-zinc-900/60 p-2.5 rounded border border-zinc-800/80 text-center">
        <div>
          <span className="block text-[10px] text-zinc-400">Total Words</span>
          <span className="font-mono text-sm font-bold text-zinc-100">
            {words.length}
          </span>
        </div>
        <div>
          <span className="block text-[10px] text-zinc-400">Timed Words</span>
          <span className="font-mono text-sm font-bold text-emerald-400">
            {alignedCount}
          </span>
        </div>
        <div>
          <span className="block text-[10px] text-zinc-400">Untimed Words</span>
          <span className="font-mono text-sm font-bold text-zinc-500">
            {unalignedCount}
          </span>
        </div>
      </div>

      {/* Interactive Controls: Karaoke & Timeline Markers */}
      <div className="flex items-center justify-between p-2.5 bg-zinc-900/40 border border-zinc-800/80 rounded">
        <div className="space-y-0.5">
          <span className="font-medium text-zinc-200 block">Preview Display Modes</span>
          <span className="text-[10px] text-zinc-400 block">
            Real-time highlighting & waveform timeline boundary marks
          </span>
        </div>
        <div className="flex items-center space-x-2">
          <button
            type="button"
            data-testid="toggle-karaoke-btn"
            onClick={onToggleKaraoke}
            className={`px-2 py-1 rounded text-[11px] font-medium border transition-colors ${
              enableKaraoke
                ? "bg-amber-950/80 border-amber-600/80 text-amber-200"
                : "bg-zinc-800 border-zinc-700 text-zinc-400"
            }`}
          >
            Karaoke: {enableKaraoke ? "ON" : "OFF"}
          </button>
          <button
            type="button"
            data-testid="toggle-word-markers-btn"
            onClick={onToggleWordMarkers}
            className={`px-2 py-1 rounded text-[11px] font-medium border transition-colors ${
              showWordMarkers
                ? "bg-indigo-950/80 border-indigo-600/80 text-indigo-200"
                : "bg-zinc-800 border-zinc-700 text-zinc-400"
            }`}
          >
            Word Markers: {showWordMarkers ? "ON" : "OFF"}
          </button>
        </div>
      </div>

      {/* Segmentation Policy Controls */}
      <div className="space-y-2.5 border border-zinc-800/80 p-3 rounded bg-zinc-900/30">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-1.5 font-medium text-zinc-200">
            <WrenchIcon size={14} className="text-zinc-400" />
            <span>Smart Segmentation Policies</span>
          </div>
          <button
            type="button"
            onClick={() => setPolicy(DEFAULT_SEGMENTATION_POLICY)}
            className="text-[10px] text-indigo-400 hover:text-indigo-300 underline"
          >
            Reset Defaults
          </button>
        </div>

        <div className="grid grid-cols-2 gap-3 text-[11px]">
          <div>
            <label className="block text-zinc-400 mb-0.5">Max Chars / Line</label>
            <input
              type="number"
              min={20}
              max={80}
              value={policy.maxCharsPerLine}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  maxCharsPerLine: parseInt(e.target.value) || 42,
                })
              }
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono"
            />
          </div>
          <div>
            <label className="block text-zinc-400 mb-0.5">Max Lines / Cue</label>
            <input
              type="number"
              min={1}
              max={3}
              value={policy.maxLinesPerCue}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  maxLinesPerCue: parseInt(e.target.value) || 2,
                })
              }
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono"
            />
          </div>
          <div>
            <label className="block text-zinc-400 mb-0.5">Max Duration (s)</label>
            <input
              type="number"
              step={0.5}
              min={2}
              max={15}
              value={policy.maxCueDurationUs / 1_000_000}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  maxCueDurationUs: Math.round(
                    (parseFloat(e.target.value) || 7) * 1_000_000
                  ),
                })
              }
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono"
            />
          </div>
          <div>
            <label className="block text-zinc-400 mb-0.5">Min Duration (s)</label>
            <input
              type="number"
              step={0.2}
              min={0.5}
              max={3}
              value={policy.minCueDurationUs / 1_000_000}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  minCueDurationUs: Math.round(
                    (parseFloat(e.target.value) || 1) * 1_000_000
                  ),
                })
              }
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono"
            />
          </div>
          <div>
            <label className="block text-zinc-400 mb-0.5">Silence Split Gap (ms)</label>
            <input
              type="number"
              step={50}
              min={200}
              max={2000}
              value={policy.splitOnSilenceGapUs / 1000}
              onChange={(e) =>
                setPolicy({
                  ...policy,
                  splitOnSilenceGapUs: Math.round(
                    (parseFloat(e.target.value) || 400) * 1000
                  ),
                })
              }
              className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-zinc-200 font-mono"
            />
          </div>
          <div className="flex items-center pt-4 space-x-2">
            <input
              type="checkbox"
              id="split-punct-toggle"
              checked={policy.splitOnPunctuation}
              onChange={(e) =>
                setPolicy({ ...policy, splitOnPunctuation: e.target.checked })
              }
              className="rounded bg-zinc-950 border-zinc-800 text-indigo-600 focus:ring-0"
            />
            <label htmlFor="split-punct-toggle" className="text-zinc-300">
              Split on punctuation (. ? ! ।)
            </label>
          </div>
        </div>

        <Button
          size="sm"
          variant="primary"
          onClick={handleGenerateCues}
          disabled={isSegmenting || words.length === 0}
          className="w-full justify-center space-x-1.5 mt-2"
        >
          <SparklesIcon size={14} />
          <span>{isSegmenting ? "Segmenting..." : "Generate Readable Cues"}</span>
        </Button>
      </div>

      {/* Generated Cues Preview & Word Inspector */}
      {previewCues && (
        <div className="space-y-3 border border-indigo-900/50 bg-indigo-950/20 p-3 rounded">
          <div className="flex items-center justify-between">
            <span className="font-semibold text-zinc-100 flex items-center space-x-1">
              <FileSearchIcon size={14} className="text-indigo-400" />
              <span>Generated Cues ({previewCues.length})</span>
            </span>
            <span className="text-[10px] text-zinc-400">
              Click any word to inspect & edit
            </span>
          </div>

          <div className="max-h-56 overflow-y-auto space-y-2 border border-zinc-800 rounded p-2 bg-zinc-950">
            {previewCues.map((cue, cIdx) => (
              <div
                key={cue.id}
                className="p-2 rounded bg-zinc-900/70 border border-zinc-800 space-y-1"
              >
                <div className="flex items-center justify-between text-[10px] font-mono text-zinc-400">
                  <span>
                    #{cIdx + 1} ({(cue.startUs / 1_000_000).toFixed(2)}s →{" "}
                    {(cue.endUs / 1_000_000).toFixed(2)}s)
                  </span>
                  {onSeek && (
                    <button
                      type="button"
                      onClick={() => onSeek(cue.startUs / 1_000_000)}
                      className="hover:text-indigo-400 flex items-center space-x-0.5"
                      title="Seek playhead to cue start"
                    >
                      <PlayIcon size={10} />
                      <span>Seek</span>
                    </button>
                  )}
                </div>

                {/* Words pill display */}
                <div className="flex flex-wrap gap-1 pt-1">
                  {cue.words?.map((w) => {
                    const isSelected = selectedWord?.id === w.id;
                    return (
                      <button
                        key={w.id}
                        type="button"
                        onClick={() => handleSelectWord(w, cue.id)}
                        className={`px-1.5 py-0.5 rounded text-[11px] font-medium transition-colors select-none ${
                          isSelected
                            ? "bg-amber-500 text-black font-bold ring-2 ring-amber-300"
                            : "bg-zinc-800 text-zinc-200 hover:bg-zinc-700"
                        }`}
                        title={`[${(w.startUs / 1000).toFixed(0)}ms - ${(w.endUs / 1000).toFixed(0)}ms] Provenance: ${w.provenance}`}
                      >
                        {w.punctuationPrefix}
                        {w.text}
                        {w.punctuationSuffix}
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>

          {/* Word Timing Inspector / Manual Correction */}
          {selectedWord && (
            <div className="p-2.5 bg-zinc-900 border border-amber-600/60 rounded space-y-2">
              <div className="flex items-center justify-between">
                <span className="font-semibold text-amber-300 text-xs">
                  Inspect Word: "{selectedWord.text}"
                </span>
                <span className="text-[10px] text-zinc-400 font-mono">
                  Provenance: {selectedWord.provenance}
                </span>
              </div>

              <div className="grid grid-cols-2 gap-2 text-[11px]">
                <div>
                  <label className="block text-zinc-400 mb-0.5">Start (ms)</label>
                  <input
                    type="number"
                    value={wordEditStartMs}
                    onChange={(e) => setWordEditStartMs(e.target.value)}
                    className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-zinc-200 font-mono"
                  />
                </div>
                <div>
                  <label className="block text-zinc-400 mb-0.5">End (ms)</label>
                  <input
                    type="number"
                    value={wordEditEndMs}
                    onChange={(e) => setWordEditEndMs(e.target.value)}
                    className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-zinc-200 font-mono"
                  />
                </div>
              </div>

              {editError && (
                <div className="text-[11px] text-red-400 flex items-center space-x-1">
                  <ShieldAlertIcon size={12} />
                  <span>{editError}</span>
                </div>
              )}

              <div className="flex items-center space-x-2 pt-1">
                <Button
                  size="sm"
                  variant="primary"
                  onClick={handleSaveWordTiming}
                  className="flex-1"
                >
                  Save Timing Correction
                </Button>
                {onSeek && (
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => onSeek(selectedWord.startUs / 1_000_000)}
                  >
                    Play Word
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="subtle"
                  onClick={() => setSelectedWord(null)}
                >
                  Cancel
                </Button>
              </div>
            </div>
          )}

          {/* Apply to Document Controls */}
          <div className="space-y-2 pt-2 border-t border-zinc-800">
            <div>
              <label className="block text-zinc-400 text-[11px] mb-1">
                New Track Display Name:
              </label>
              <input
                type="text"
                placeholder={`Smart Subtitles (${alignmentResult?.language || "en"})`}
                value={trackNameInput}
                onChange={(e) => setTrackNameInput(e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded px-2 py-1 text-xs text-zinc-200"
              />
            </div>

            <div className="flex items-center space-x-2">
              <Button
                size="sm"
                variant="primary"
                onClick={handleApplyToSubtitles}
                disabled={isApplying}
                className="flex-1"
              >
                {isApplying ? "Applying..." : "Add as New Subtitle Track"}
              </Button>
              <Button
                size="sm"
                variant="subtle"
                onClick={() => setPreviewCues(null)}
              >
                Clear Preview
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
