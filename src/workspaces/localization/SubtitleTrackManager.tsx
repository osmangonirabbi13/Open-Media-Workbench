import React, { useState } from "react";
import type { MediaAsset } from "../../types/media";
import type {
  SubtitleDocument,
  SubtitleFinding,
  SubtitleFormat,
  SubtitleTrack,
} from "../../types/subtitles";
import {
  SubtitlesIcon,
  UploadIcon,
  DownloadIcon,
  ShieldAlertIcon,
  CheckIcon,
  InfoIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";
import {
  exportSubtitleTrack,
  formatMicrosToTimecode,
  openSubtitleProject,
  pickAndImportSubtitle,
  saveSubtitleProject,
} from "../../lib/subtitles";

export interface SubtitleTrackManagerProps {
  asset: MediaAsset | null;
  document: SubtitleDocument | null;
  activeTrack: SubtitleTrack | null;
  currentTime: number;
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

  const currentTimeUs = Math.round(currentTime * 1_000_000);

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

  const cues = activeTrack?.cues || [];

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-100 select-none overflow-hidden">
      {/* 1. Header Toolbar */}
      <div className="p-3 border-b border-zinc-800 bg-zinc-900/40 space-y-2.5">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <SubtitlesIcon size={16} className="text-indigo-400" />
            <h3 className="text-xs font-semibold text-zinc-200">Subtitle Tracks</h3>
            <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-zinc-800 text-zinc-400 border border-zinc-700/50">
              {document ? `${document.tracks.length} track(s)` : "No Document"}
            </span>
          </div>

          <div className="flex items-center space-x-1.5">
            <Button
              size="xs"
              variant="secondary"
              onClick={handleOpenProject}
              disabled={isLoading}
              title="Open Subtitle Project (.omwsub)"
            >
              Open Project
            </Button>
            <Button
              size="xs"
              variant="secondary"
              onClick={handleSaveProject}
              disabled={!document || isLoading}
              title="Save Subtitle Project (.omwsub)"
            >
              Save Project
            </Button>
            <Button
              size="xs"
              variant="primary"
              onClick={handleImport}
              disabled={isLoading}
              className="bg-indigo-600 hover:bg-indigo-500 text-white font-medium shadow-xs"
            >
              <UploadIcon size={12} className="mr-1" />
              Import Subtitle
            </Button>
          </div>
        </div>

        {/* Track Selector Bar */}
        {document && document.tracks.length > 0 && (
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
                  <span className="truncate max-w-[120px]">{track.displayName}</span>
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
        )}

        {/* Export & Validation Quick Actions for Active Track */}
        {activeTrack && (
          <div className="flex items-center justify-between text-xs pt-1 border-t border-zinc-800/60">
            <div className="flex items-center space-x-2">
              <span className="text-zinc-400 text-[11px]">Export Active:</span>
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

      {/* 2. Notification Feedback Banners */}
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

      {/* 3. Validation Findings Drawer */}
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

      {/* 4. Subtitle Cue List / Empty State */}
      <div className="flex-1 min-h-0 overflow-y-auto p-3 space-y-1.5 select-text">
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
              Import a standard SubRip (.srt) or WebVTT (.vtt) file using the button above to inspect cues and synchronize preview.
            </p>
            <Button
              size="xs"
              variant="secondary"
              onClick={handleImport}
              disabled={isLoading}
              className="mt-2"
            >
              <UploadIcon size={12} className="mr-1" />
              Import Subtitles Now
            </Button>
          </div>
        ) : (
          cues.map((cue, idx) => {
            const isActive = cue.startUs <= currentTimeUs && currentTimeUs < cue.endUs;
            const startTimeStr = formatMicrosToTimecode(cue.startUs);
            const endTimeStr = formatMicrosToTimecode(cue.endUs);
            const durationSecs = ((cue.endUs - cue.startUs) / 1_000_000).toFixed(2);

            return (
              <div
                key={cue.id}
                data-testid={`subtitle-cue-row-${idx}`}
                onClick={() => onSeek(cue.startUs / 1_000_000)}
                className={`p-2.5 rounded-md border text-xs cursor-pointer transition-all ${
                  isActive
                    ? "bg-indigo-950/60 border-indigo-600/80 shadow-sm text-indigo-100 ring-1 ring-indigo-500/40"
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

                <div className="text-xs leading-relaxed whitespace-pre-line font-sans break-words">
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
