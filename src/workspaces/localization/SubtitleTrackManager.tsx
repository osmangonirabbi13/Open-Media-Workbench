import React, { useState, useEffect } from "react";
import type { MediaAsset } from "../../types/media";
import type {
  SubtitleCue,
  SubtitleDocument,
  SubtitleFinding,
  SubtitleFormat,
  SubtitleOperation,
  SubtitleTrack,
} from "../../types/subtitles";
import {
  SubtitlesIcon,
  UploadIcon,
  DownloadIcon,
  ShieldAlertIcon,
  CheckIcon,
  InfoIcon,
  UndoIcon,
  RedoIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";
import {
  exportSubtitleTrack,
  formatMicrosToTimecode,
  openSubtitleProject,
  pickAndImportSubtitle,
  saveSubtitleProject,
} from "../../lib/subtitles";
import { SubtitleCueInspector } from "./SubtitleCueInspector";
import {
  loadAutosaveSnapshot,
  clearAutosaveSnapshot,
} from "../../lib/subtitleAutosave";

export interface SubtitleTrackManagerProps {
  asset: MediaAsset | null;
  document: SubtitleDocument | null;
  activeTrack: SubtitleTrack | null;
  currentTime: number;
  selectedCueIds?: string[];
  onSelectCues?: (cueIds: string[]) => void;
  onApplyOperation?: (op: SubtitleOperation) => void;
  canUndo?: boolean;
  canRedo?: boolean;
  onUndo?: () => void;
  onRedo?: () => void;
  onDocumentChange: (doc: SubtitleDocument | null) => void;
  onActiveTrackChange: (trackId: string) => void;
  onToggleTrackVisibility: (trackId: string) => void;
  onSeek: (seconds: number) => void;
}

export function SubtitleTrackManager({
  asset,
  document,
  activeTrack,
  currentTime,
  selectedCueIds = [],
  onSelectCues = () => {},
  onApplyOperation = () => {},
  canUndo = false,
  canRedo = false,
  onUndo = () => {},
  onRedo = () => {},
  onDocumentChange,
  onActiveTrackChange,
  onToggleTrackVisibility,
  onSeek,
}: SubtitleTrackManagerProps): React.ReactElement {
  const [isLoading, setIsLoading] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showFindings, setShowFindings] = useState(false);
  const [activeFindings, setActiveFindings] = useState<SubtitleFinding[]>([]);
  const [autosavePrompt, setAutosavePrompt] = useState<SubtitleDocument | null>(null);

  const currentTimeUs = Math.round(currentTime * 1_000_000);

  // Check for crash recovery / autosave snapshot on mount or asset change
  useEffect(() => {
    const snapshot = loadAutosaveSnapshot(asset?.id);
    if (snapshot && snapshot.document) {
      if (!document || snapshot.savedAtEpochMs > (document.updatedAtEpochMs || 0)) {
        setAutosavePrompt(snapshot.document);
      }
    }
  }, [asset?.id]);

  // Restore autosaved session
  const handleRestoreAutosave = () => {
    if (autosavePrompt) {
      onDocumentChange(autosavePrompt);
      setFeedback("Restored unsaved subtitle session from crash recovery snapshot.");
      setAutosavePrompt(null);
    }
  };

  // Dismiss autosave recovery
  const handleDismissAutosave = () => {
    clearAutosaveSnapshot(asset?.id);
    setAutosavePrompt(null);
  };

  // Import Subtitle file via native dialog
  const handleImport = async () => {
    setError(null);
    setFeedback(null);
    setIsLoading(true);

    try {
      const res = await pickAndImportSubtitle(asset?.id);
      if (!res) {
        setIsLoading(false);
        return; // User cancelled
      }

      if (document) {
        // Add to existing document
        const updatedTracks = [...document.tracks, res.track];
        const updatedDoc: SubtitleDocument = {
          ...document,
          tracks: updatedTracks,
          activeTrackId: res.track.id,
          updatedAtEpochMs: Date.now(),
        };
        onDocumentChange(updatedDoc);
      } else {
        onDocumentChange(res.document);
      }

      setActiveFindings(res.findings);
      if (res.warnings.length > 0) {
        setFeedback(`Imported ${res.fileName} with ${res.warnings.length} format notices.`);
      } else {
        setFeedback(`Successfully imported ${res.track.cues.length} cues from ${res.fileName}`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  // Open Subtitle project (.omwsub)
  const handleOpenProject = async () => {
    setError(null);
    setFeedback(null);
    setIsLoading(true);

    try {
      const res = await openSubtitleProject();
      if (!res) {
        setIsLoading(false);
        return;
      }
      onDocumentChange(res.document);
      setActiveFindings(res.findings);
      setFeedback(`Opened project ${res.fileName} (${res.document.tracks.length} track(s))`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  // Save Subtitle project
  const handleSaveProject = async () => {
    if (!document) return;
    setError(null);
    setFeedback(null);
    setIsLoading(true);

    try {
      const res = await saveSubtitleProject(document);
      if (res) {
        clearAutosaveSnapshot(asset?.id);
        setFeedback(`Project saved successfully to ${res.projectPath}`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  // Export active track to SRT or WebVTT
  const handleExportTrack = async (format: SubtitleFormat) => {
    if (!activeTrack) return;
    setError(null);
    setFeedback(null);
    setIsLoading(true);

    try {
      const res = await exportSubtitleTrack(activeTrack, format);
      if (res) {
        const warningsMsg = res.warnings.length > 0 ? ` (${res.warnings.join("; ")})` : "";
        setFeedback(`Exported track to ${res.exportedPath} [${res.bytesWritten} bytes]${warningsMsg}`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  // Add a new cue at the current playhead
  const handleAddCueAtPlayhead = () => {
    if (!activeTrack) return;
    const defaultDurationUs = 2_000_000; // 2 seconds
    const newCue: SubtitleCue = {
      id: crypto.randomUUID(),
      startUs: currentTimeUs,
      endUs: currentTimeUs + defaultDurationUs,
      text: "New Subtitle",
    };

    onApplyOperation({
      type: "addCue",
      payload: {
        trackId: activeTrack.id,
        cue: newCue,
      },
    });

    onSelectCues([newCue.id]);
  };

  const cues = activeTrack?.cues || [];

  // Selected cue instances
  const selectedCues = cues.filter((c) => selectedCueIds.includes(c.id));
  const primarySelectedCue = selectedCues.length > 0 ? selectedCues[0] : null;

  // Next cue id for merging
  const primaryIdx = primarySelectedCue ? cues.findIndex((c) => c.id === primarySelectedCue.id) : -1;
  const nextCueId = primaryIdx >= 0 && primaryIdx < cues.length - 1 ? cues[primaryIdx + 1].id : undefined;

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-100 select-none overflow-hidden">
      {/* 1. Header Toolbar */}
      <div className="p-3 border-b border-zinc-800 bg-zinc-900/40 space-y-2.5">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <SubtitlesIcon size={16} className="text-indigo-400" />
            <h3 className="text-xs font-semibold text-zinc-200">Subtitle Studio</h3>
            <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-zinc-800 text-zinc-400 border border-zinc-700/50">
              {document ? `${document.tracks.length} track(s)` : "No Document"}
            </span>
          </div>

          <div className="flex items-center space-x-1.5">
            {/* Undo / Redo buttons */}
            <button
              type="button"
              disabled={!canUndo}
              onClick={onUndo}
              title="Undo edit (Ctrl+Z)"
              className="p-1 rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-40 disabled:hover:bg-zinc-800 text-zinc-300 transition-colors"
            >
              <UndoIcon size={13} />
            </button>
            <button
              type="button"
              disabled={!canRedo}
              onClick={onRedo}
              title="Redo edit (Ctrl+Shift+Z / Ctrl+Y)"
              className="p-1 rounded bg-zinc-800 hover:bg-zinc-700 disabled:opacity-40 disabled:hover:bg-zinc-800 text-zinc-300 transition-colors"
            >
              <RedoIcon size={13} />
            </button>

            <div className="h-3.5 w-[1px] bg-zinc-800 mx-0.5" />

            <Button
              size="xs"
              variant="secondary"
              onClick={handleOpenProject}
              disabled={isLoading}
              title="Open Subtitle Project (.omwsub)"
            >
              Open
            </Button>
            <Button
              size="xs"
              variant="secondary"
              onClick={handleSaveProject}
              disabled={!document || isLoading}
              title="Save Subtitle Project (.omwsub)"
            >
              Save
            </Button>
            <Button
              size="xs"
              variant="primary"
              onClick={handleImport}
              disabled={isLoading}
              className="bg-indigo-600 hover:bg-indigo-500 text-white font-medium shadow-xs"
            >
              <UploadIcon size={12} className="mr-1" />
              Import
            </Button>
          </div>
        </div>

        {/* Track Selector Bar & Add Cue Button */}
        {document && document.tracks.length > 0 && (
          <div className="flex items-center justify-between pt-1">
            <div className="flex items-center space-x-2 overflow-x-auto py-1">
              {document.tracks.map((track) => {
                const isActive = activeTrack?.id === track.id;
                return (
                  <div
                    key={track.id}
                    onClick={() => onActiveTrackChange(track.id)}
                    className={`cursor-pointer px-2.5 py-1 rounded text-xs flex items-center space-x-2 transition-colors border ${
                      isActive
                        ? "bg-indigo-950/70 text-indigo-200 border-indigo-700/80 shadow-xs font-medium"
                        : "bg-zinc-900/60 text-zinc-400 border-zinc-800 hover:text-zinc-200 hover:bg-zinc-800/40"
                    }`}
                  >
                    <span className="uppercase font-mono text-[10px] font-bold px-1 rounded bg-zinc-800/80 text-zinc-300">
                      {track.language}
                    </span>
                    <span className="truncate max-w-[100px]">{track.displayName}</span>
                    <span className="text-[10px] opacity-70 font-mono">({track.cues.length})</span>
                    <button
                      type="button"
                      title={track.isVisible ? "Hide on Preview" : "Show on Preview"}
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleTrackVisibility(track.id);
                      }}
                      className={`ml-1 text-[11px] hover:text-white transition-opacity ${
                        track.isVisible ? "text-indigo-400" : "text-zinc-600"
                      }`}
                    >
                      {track.isVisible ? "👁️" : "✕"}
                    </button>
                  </div>
                );
              })}
            </div>

            <Button
              size="xs"
              variant="secondary"
              onClick={handleAddCueAtPlayhead}
              disabled={!activeTrack}
              title="Add a new subtitle cue at current playhead"
              className="shrink-0 ml-2 bg-zinc-800 hover:bg-zinc-700 border-zinc-700 text-zinc-200"
            >
              + Add Cue
            </Button>
          </div>
        )}

        {/* Export & Validation Quick Actions for Active Track */}
        {activeTrack && (
          <div className="flex items-center justify-between text-xs pt-1 border-t border-zinc-800/60">
            <div className="flex items-center space-x-2">
              <span className="text-zinc-400 text-[11px]">Export:</span>
              <button
                type="button"
                onClick={() => handleExportTrack("srt")}
                className="px-2 py-0.5 rounded text-[11px] bg-zinc-800 text-zinc-300 hover:bg-zinc-700 border border-zinc-700 flex items-center space-x-1"
              >
                <DownloadIcon size={10} />
                <span>SRT</span>
              </button>
              <button
                type="button"
                onClick={() => handleExportTrack("webVtt")}
                className="px-2 py-0.5 rounded text-[11px] bg-zinc-800 text-zinc-300 hover:bg-zinc-700 border border-zinc-700 flex items-center space-x-1"
              >
                <DownloadIcon size={10} />
                <span>WebVTT</span>
              </button>
            </div>

            {activeFindings.length > 0 && (
              <button
                type="button"
                onClick={() => setShowFindings(!showFindings)}
                className="flex items-center space-x-1 text-[11px] text-amber-400 hover:text-amber-300 transition-colors"
              >
                <ShieldAlertIcon size={12} />
                <span>{activeFindings.length} Notice(s)</span>
              </button>
            )}
          </div>
        )}
      </div>

      {/* Crash Recovery / Autosave Banner */}
      {autosavePrompt && (
        <div className="p-2.5 bg-amber-950/60 border-b border-amber-800/60 text-xs text-amber-200 flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <ShieldAlertIcon size={14} className="text-amber-400 shrink-0" />
            <span>Unsaved subtitle edits detected from previous session.</span>
          </div>
          <div className="flex items-center space-x-1.5 ml-2">
            <Button
              size="xs"
              variant="primary"
              onClick={handleRestoreAutosave}
              className="bg-amber-600 hover:bg-amber-500 text-white font-medium"
            >
              Restore
            </Button>
            <Button
              size="xs"
              variant="subtle"
              onClick={handleDismissAutosave}
              className="text-zinc-400 hover:text-zinc-200"
            >

              Dismiss
            </Button>
          </div>
        </div>
      )}

      {/* Feedback & Error Banners */}
      {feedback && (
        <div className="px-3 py-2 bg-indigo-950/40 border-b border-indigo-900/60 text-xs text-indigo-200 flex items-center justify-between">
          <div className="flex items-center space-x-1.5">
            <CheckIcon size={13} className="text-indigo-400 shrink-0" />
            <span className="truncate">{feedback}</span>
          </div>
          <button
            type="button"
            onClick={() => setFeedback(null)}
            className="text-zinc-400 hover:text-zinc-200 text-xs ml-2"
          >
            ✕
          </button>
        </div>
      )}

      {error && (
        <div className="px-3 py-2 bg-rose-950/40 border-b border-rose-900/60 text-xs text-rose-200 flex items-center justify-between">
          <div className="flex items-center space-x-1.5">
            <ShieldAlertIcon size={13} className="text-rose-400 shrink-0" />
            <span className="truncate">{error}</span>
          </div>
          <button
            type="button"
            onClick={() => setError(null)}
            className="text-zinc-400 hover:text-zinc-200 text-xs ml-2"
          >
            ✕
          </button>
        </div>
      )}

      {/* Validation Findings Drawer */}
      {showFindings && activeFindings.length > 0 && (
        <div className="p-3 bg-zinc-900/90 border-b border-amber-900/50 text-xs space-y-1.5 max-h-40 overflow-y-auto">
          <div className="flex items-center justify-between text-[11px] font-semibold text-amber-400">
            <span>Validation Findings ({activeFindings.length})</span>
            <button
              type="button"
              onClick={() => setShowFindings(false)}
              className="text-zinc-400 hover:text-zinc-200"
            >
              ✕
            </button>
          </div>
          {activeFindings.map((finding, idx) => (
            <div
              key={idx}
              className="text-[11px] p-1.5 rounded bg-zinc-950 border border-zinc-800 flex items-start space-x-2"
            >
              <span
                className={`uppercase font-mono text-[9px] px-1 py-0.2 rounded font-bold ${
                  finding.severity === "error"
                    ? "bg-rose-950 text-rose-300 border border-rose-800"
                    : finding.severity === "warning"
                      ? "bg-amber-950 text-amber-300 border border-amber-800"
                      : "bg-blue-950 text-blue-300 border border-blue-800"
                }`}
              >
                {finding.code}
              </span>
              <span className="text-zinc-300 flex-1 leading-snug">{finding.message}</span>
            </div>
          ))}
        </div>
      )}

      {/* Selected Cue Inspector */}
      {(primarySelectedCue || selectedCues.length > 1) && activeTrack && (
        <div className="p-2 border-b border-zinc-800 bg-zinc-950/90 shrink-0">
          <SubtitleCueInspector
            cue={primarySelectedCue}
            selectedCues={selectedCues}
            currentTimeMicros={currentTimeUs}
            nextCueId={nextCueId}
            onUpdateText={(cueId, text) => {
              onApplyOperation({
                type: "updateCueText",
                payload: { trackId: activeTrack.id, cueId, text },
              });
            }}
            onUpdateTiming={(cueId, startUs, endUs) => {
              onApplyOperation({
                type: "setCueTiming",
                payload: { trackId: activeTrack.id, cueId, startUs, endUs },
              });
            }}
            onNudge={(cueIds, deltaUs) => {
              if (cueIds.length === 1) {
                onApplyOperation({
                  type: "moveCue",
                  payload: { trackId: activeTrack.id, cueId: cueIds[0], deltaUs },
                });
              } else {
                onApplyOperation({
                  type: "shiftSelectedCues",
                  payload: { trackId: activeTrack.id, cueIds, deltaUs },
                });
              }
            }}
            onSplit={(cueId, splitTimeUs) => {
              onApplyOperation({
                type: "splitCue",
                payload: { trackId: activeTrack.id, cueId, splitTimeUs },
              });
            }}
            onMergeWithNext={(firstCueId, secondCueId) => {
              onApplyOperation({
                type: "mergeCues",
                payload: { trackId: activeTrack.id, firstCueId, secondCueId },
              });
            }}
            onDuplicate={(cueId) => {
              onApplyOperation({
                type: "duplicateCue",
                payload: { trackId: activeTrack.id, cueId },
              });
            }}
            onDeleteCues={(cueIds) => {
              if (cueIds.length === 1) {
                onApplyOperation({
                  type: "deleteCue",
                  payload: { trackId: activeTrack.id, cueId: cueIds[0] },
                });
              } else {
                onApplyOperation({
                  type: "deleteCues",
                  payload: { trackId: activeTrack.id, cueIds },
                });
              }
              onSelectCues([]);
            }}
            onSeek={onSeek}
          />
        </div>
      )}

      {/* Subtitle Cue List */}
      <div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-1.5 select-none">
        {!asset && (
          <div className="p-4 rounded-lg border border-amber-900/30 bg-amber-950/10 text-amber-300 text-xs flex items-center space-x-2 mb-2">
            <InfoIcon size={14} className="shrink-0 text-amber-400" />
            <span>Select a media asset from the Media Library on the left to synchronize subtitle timing.</span>
          </div>
        )}

        {cues.length === 0 ? (
          <div className="p-8 rounded-lg border border-dashed border-zinc-800 text-center space-y-2.5 my-4">
            <SubtitlesIcon size={28} className="mx-auto text-zinc-600" />
            <h4 className="text-xs font-medium text-zinc-300">No Subtitle Track Active</h4>
            <p className="text-[11px] text-zinc-500 max-w-sm mx-auto leading-relaxed">
              Import a standard SubRip (.srt) or WebVTT (.vtt) file or click "+ Add Cue" to start editing.
            </p>
            <div className="flex items-center justify-center space-x-2 pt-2">
              <Button
                size="xs"
                variant="secondary"
                onClick={handleImport}
                disabled={isLoading}
              >
                <UploadIcon size={12} className="mr-1" />
                Import Subtitles
              </Button>
              {activeTrack && (
                <Button
                  size="xs"
                  variant="primary"
                  onClick={handleAddCueAtPlayhead}
                >
                  + Add First Cue
                </Button>
              )}
            </div>
          </div>
        ) : (
          cues.map((cue, idx) => {
            const isSelected = selectedCueIds.includes(cue.id);
            const isActive = cue.startUs <= currentTimeUs && currentTimeUs < cue.endUs;
            const startTimeStr = formatMicrosToTimecode(cue.startUs);
            const endTimeStr = formatMicrosToTimecode(cue.endUs);
            const durationSecs = ((cue.endUs - cue.startUs) / 1_000_000).toFixed(2);

            return (
              <div
                key={cue.id}
                data-testid={`subtitle-cue-row-${idx}`}
                onClick={(e) => {
                  if (e.metaKey || e.ctrlKey) {
                    if (isSelected) {
                      onSelectCues(selectedCueIds.filter((id) => id !== cue.id));
                    } else {
                      onSelectCues([...selectedCueIds, cue.id]);
                    }
                  } else if (e.shiftKey && selectedCueIds.length > 0) {
                    const lastId = selectedCueIds[selectedCueIds.length - 1];
                    const lastIdx = cues.findIndex((c) => c.id === lastId);
                    if (lastIdx !== -1) {
                      const start = Math.min(lastIdx, idx);
                      const end = Math.max(lastIdx, idx);
                      const range = cues.slice(start, end + 1).map((c) => c.id);
                      onSelectCues(Array.from(new Set([...selectedCueIds, ...range])));
                    }
                  } else {
                    onSelectCues([cue.id]);
                  }
                  onSeek(cue.startUs / 1_000_000);
                }}
                className={`p-2.5 rounded-md border text-xs cursor-pointer transition-all ${
                  isSelected
                    ? "bg-indigo-950/90 border-indigo-500 shadow-sm text-indigo-100 ring-1 ring-indigo-400/80"
                    : isActive
                      ? "bg-indigo-950/50 border-indigo-600/70 text-indigo-200"
                      : "bg-zinc-900/50 border-zinc-800/80 text-zinc-300 hover:bg-zinc-900 hover:border-zinc-700"
                }`}
              >
                <div className="flex items-center justify-between font-mono text-[11px] mb-1 text-zinc-400">
                  <div className="flex items-center space-x-1.5">
                    <span className="font-semibold text-zinc-500">#{idx + 1}</span>
                    <span className={isActive ? "text-indigo-300 font-bold" : "text-zinc-300"}>
                      {startTimeStr} → {endTimeStr}
                    </span>
                  </div>
                  <span className="text-[10px] text-zinc-500">{durationSecs}s</span>
                </div>

                {cue.speaker && (
                  <div className="text-[10px] text-amber-400 font-semibold mb-0.5 tracking-wide">
                    [{cue.speaker}]
                  </div>
                )}

                <div className="text-xs leading-relaxed whitespace-pre-line font-sans break-words select-text">
                  {cue.text}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
