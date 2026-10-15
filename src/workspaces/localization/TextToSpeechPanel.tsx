import React, { useState, useEffect, useRef } from "react";
import type { SubtitleDocument, SubtitleTrack } from "../../types/subtitles";
import type {
  AiVoiceManifest,
  TtsAudioSegment,
  TtsResult,
  TtsRuntimeCapabilities,
  TtsVoiceSettings,
} from "../../types/tts";
import {
  getTtsRuntimeCapabilities,
  listAvailableTtsVoices,
  startOfflineTts,
  getOfflineTtsStatus,
  getOfflineTtsResult,
  cancelOfflineTts,
  exportTtsAudio,
} from "../../lib/tts";
import { Button } from "../../components/ui/Button";
import {
  MicIcon,
  PlayIcon,
  PauseIcon,
  DownloadIcon,
  AlertTriangleIcon,
  CheckIcon,
  SparklesIcon,
} from "../../components/ui/Icons";

export interface TextToSpeechPanelProps {
  document: SubtitleDocument | null;
  activeTrack: SubtitleTrack | null;
  onOpenModelManager?: () => void;
  onSeek?: (seconds: number) => void;
}

export function TextToSpeechPanel({
  document,
  activeTrack,
  onOpenModelManager,
  onSeek,
}: TextToSpeechPanelProps): React.ReactElement {
  // Mode: script or subtitle track
  const [sourceMode, setSourceMode] = useState<"script" | "subtitle">("script");
  const [scriptText, setScriptText] = useState<string>("Welcome to Open Media Workbench.");
  const [selectedTrackId, setSelectedTrackId] = useState<string>(activeTrack?.id || "");
  const [selectedCueIds] = useState<string[]>([]);

  // Voices & capabilities
  const [voices, setVoices] = useState<AiVoiceManifest[]>([]);
  const [selectedVoiceId, setSelectedVoiceId] = useState<string>("piper-en-lessac-medium");
  const [runtimeCaps, setRuntimeCaps] = useState<TtsRuntimeCapabilities | null>(null);

  // Voice settings
  const [speedScale, setSpeedScale] = useState<number>(1.0);
  const [noiseScale, setNoiseScale] = useState<number>(0.667);
  const [noiseW, setNoiseW] = useState<number>(0.8);
  const [showSettings, setShowSettings] = useState<boolean>(false);

  // Job state
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [isSynthesizing, setIsSynthesizing] = useState<boolean>(false);
  const [progressPct, setProgressPct] = useState<number>(0);
  const [statusMessage, setStatusMessage] = useState<string>("");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [ttsResult, setTtsResult] = useState<TtsResult | null>(null);

  // Audio preview state
  const [activeSegmentIndex, setActiveSegmentIndex] = useState<number>(0);
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  // Export state
  const [exportFeedback, setExportFeedback] = useState<string | null>(null);

  useEffect(() => {
    if (activeTrack && !selectedTrackId) {
      setSelectedTrackId(activeTrack.id);
    }
  }, [activeTrack, selectedTrackId]);

  useEffect(() => {
    async function loadData() {
      try {
        const [caps, vList] = await Promise.all([
          getTtsRuntimeCapabilities(),
          listAvailableTtsVoices(),
        ]);
        setRuntimeCaps(caps);
        setVoices(vList);
        if (vList.length > 0 && !selectedVoiceId) {
          setSelectedVoiceId(vList[0].id);
        }
      } catch (err) {
        console.error("Failed to load TTS data:", err);
      }
    }
    loadData();
  }, []);

  // Poll active synthesis job
  useEffect(() => {
    if (!activeJobId || !isSynthesizing) return;

    const interval = setInterval(async () => {
      try {
        const status = await getOfflineTtsStatus(activeJobId);
        setProgressPct(status.progressPct);
        setStatusMessage(status.currentMessage);

        if (status.stage === "completed") {
          setIsSynthesizing(false);
          const res = await getOfflineTtsResult(activeJobId);
          setTtsResult(res);
          setActiveSegmentIndex(0);
        } else if (status.stage === "failed") {
          setIsSynthesizing(false);
          setErrorMsg(status.error || "Synthesis failed");
        } else if (status.stage === "cancelled") {
          setIsSynthesizing(false);
          setStatusMessage("Synthesis cancelled");
        }
      } catch (err) {
        setIsSynthesizing(false);
        setErrorMsg(err instanceof Error ? err.message : String(err));
      }
    }, 200);

    return () => clearInterval(interval);
  }, [activeJobId, isSynthesizing]);

  const targetTrack = document?.tracks.find((t) => t.id === selectedTrackId) || activeTrack;

  const handleStartSynthesis = async () => {
    setErrorMsg(null);
    setExportFeedback(null);
    setTtsResult(null);

    const voiceSettings: TtsVoiceSettings = {
      speedScale,
      noiseScale,
      noiseW,
    };

    try {
      setIsSynthesizing(true);
      setProgressPct(0);
      setStatusMessage("Starting offline synthesis...");

      let jobId: string;
      if (sourceMode === "script") {
        if (!scriptText.trim()) {
          throw new Error("Script text cannot be empty");
        }
        jobId = await startOfflineTts({
          source: {
            type: "script",
            text: scriptText,
          },
          voiceId: selectedVoiceId,
          language: "en-US",
          settings: voiceSettings,
        });
      } else {
        if (!targetTrack || targetTrack.cues.length === 0) {
          throw new Error("Selected subtitle track has no cues to synthesize");
        }
        jobId = await startOfflineTts(
          {
            source: {
              type: "subtitleTrack",
              trackId: targetTrack.id,
              cueIds: selectedCueIds.length > 0 ? selectedCueIds : undefined,
            },
            voiceId: selectedVoiceId,
            language: targetTrack.language || "en",
            settings: voiceSettings,
          },
          targetTrack
        );
      }

      setActiveJobId(jobId);
    } catch (err) {
      setIsSynthesizing(false);
      setErrorMsg(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCancelSynthesis = async () => {
    if (!activeJobId) return;
    try {
      await cancelOfflineTts(activeJobId);
      setIsSynthesizing(false);
      setStatusMessage("Job cancelled");
    } catch (err) {
      console.error("Cancel failed:", err);
    }
  };

  const selectedVoice = voices.find((v) => v.id === selectedVoiceId);

  const activeSegment: TtsAudioSegment | undefined =
    ttsResult?.segments[activeSegmentIndex];

  const handlePlayToggle = () => {
    if (!audioRef.current) return;
    if (isPlaying) {
      audioRef.current.pause();
      setIsPlaying(false);
    } else {
      audioRef.current.play();
      setIsPlaying(true);
    }
  };

  const handleSegmentSelect = (index: number) => {
    setActiveSegmentIndex(index);
    setIsPlaying(false);
    const seg = ttsResult?.segments[index];
    if (seg?.cueStartUs && onSeek) {
      onSeek(seg.cueStartUs / 1_000_000);
    }
  };

  const handleExportSegment = async (segmentId: string) => {
    if (!activeJobId) return;
    try {
      const defaultName = `speech_${segmentId}.wav`;
      const exported = await exportTtsAudio(activeJobId, defaultName, segmentId);
      setExportFeedback(`Exported segment to ${exported}`);
    } catch (err) {
      setErrorMsg(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const handleExportAll = async () => {
    if (!activeJobId) return;
    try {
      const defaultName = `speech_full_${activeJobId}.wav`;
      const exported = await exportTtsAudio(activeJobId, defaultName);
      setExportFeedback(`Exported complete speech to ${exported}`);
    } catch (err) {
      setErrorMsg(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  return (
    <div className="p-4 space-y-4 overflow-y-auto h-full text-zinc-100">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h4 className="text-xs font-semibold text-zinc-200 flex items-center space-x-1.5">
          <MicIcon size={14} className="text-indigo-400" />
          <span>Offline Text-to-Speech</span>
        </h4>
        <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-950 text-emerald-300 border border-emerald-800">
          STEP 24 Active
        </span>
      </div>

      {/* Runtime readiness info */}
      <div className="p-2.5 rounded border border-zinc-800 bg-zinc-900/60 flex items-center justify-between text-xs">
        <div className="flex items-center space-x-2">
          <span className="text-zinc-400">Runtime:</span>
          <span className="font-mono text-zinc-200">
            {runtimeCaps?.version || (runtimeCaps?.discovered ? "Piper (Active)" : "Unavailable")}
          </span>
        </div>
        {onOpenModelManager && (
          <button
            onClick={onOpenModelManager}
            className="text-[11px] text-indigo-400 hover:text-indigo-300 underline"
          >
            Model Manager
          </button>
        )}
      </div>

      {/* Source mode selector tabs */}
      <div className="flex border-b border-zinc-800 space-x-2 text-xs">
        <button
          onClick={() => setSourceMode("script")}
          className={`pb-1.5 font-medium transition-colors ${
            sourceMode === "script"
              ? "border-b-2 border-indigo-500 text-indigo-400"
              : "text-zinc-400 hover:text-zinc-200"
          }`}
        >
          Script Text
        </button>
        <button
          onClick={() => setSourceMode("subtitle")}
          className={`pb-1.5 font-medium transition-colors ${
            sourceMode === "subtitle"
              ? "border-b-2 border-indigo-500 text-indigo-400"
              : "text-zinc-400 hover:text-zinc-200"
          }`}
        >
          Subtitle Track
        </button>
      </div>

      {/* Source inputs */}
      {sourceMode === "script" ? (
        <div className="space-y-1.5">
          <label className="text-[11px] text-zinc-400 block font-medium">Script Input</label>
          <textarea
            value={scriptText}
            onChange={(e) => setScriptText(e.target.value)}
            rows={4}
            placeholder="Type or paste narration script..."
            className="w-full text-xs p-2.5 bg-zinc-900 border border-zinc-800 rounded-md text-zinc-100 focus:outline-hidden focus:border-indigo-500 font-sans"
          />
        </div>
      ) : (
        <div className="space-y-2">
          <label className="text-[11px] text-zinc-400 block font-medium">Source Subtitle Track</label>
          <select
            value={selectedTrackId}
            onChange={(e) => setSelectedTrackId(e.target.value)}
            className="w-full text-xs p-2 bg-zinc-900 border border-zinc-800 rounded-md text-zinc-200 focus:outline-hidden focus:border-indigo-500"
          >
            {document?.tracks.map((t) => (
              <option key={t.id} value={t.id}>
                {t.displayName} ({t.language}) — {t.cues.length} cues
              </option>
            ))}
          </select>
          {targetTrack && (
            <div className="text-[11px] text-zinc-400">
              Total cues to synthesize: {targetTrack.cues.length}
            </div>
          )}
        </div>
      )}

      {/* Voice selection */}
      <div className="space-y-1.5">
        <label className="text-[11px] text-zinc-400 block font-medium">Neural Voice</label>
        <select
          value={selectedVoiceId}
          onChange={(e) => setSelectedVoiceId(e.target.value)}
          className="w-full text-xs p-2 bg-zinc-900 border border-zinc-800 rounded-md text-zinc-200 focus:outline-hidden focus:border-indigo-500"
        >
          {voices.map((v) => (
            <option key={v.id} value={v.id}>
              {v.displayName} [{v.language}] ({v.license})
            </option>
          ))}
        </select>
        {selectedVoice && (
          <div className="text-[10px] text-zinc-400 flex items-center justify-between">
            <span>Sample Rate: {selectedVoice.sampleRate} Hz</span>
            <span className={selectedVoice.isInstalled ? "text-emerald-400" : "text-amber-400"}>
              {selectedVoice.isInstalled ? "Installed" : "Needs Installation"}
            </span>
          </div>
        )}
      </div>

      {/* Advanced Voice Settings Collapsible */}
      <div className="border border-zinc-800/80 rounded-md bg-zinc-900/30 p-2 text-xs space-y-2">
        <button
          onClick={() => setShowSettings(!showSettings)}
          className="w-full flex items-center justify-between text-zinc-400 hover:text-zinc-200 font-medium"
        >
          <span>Voice Synthesis Settings</span>
          <span>{showSettings ? "▲" : "▼"}</span>
        </button>

        {showSettings && (
          <div className="pt-2 space-y-3 border-t border-zinc-800/60">
            <div>
              <div className="flex justify-between text-[11px] text-zinc-400">
                <span>Speed Scale: {speedScale.toFixed(2)}x</span>
                <button
                  onClick={() => setSpeedScale(1.0)}
                  className="text-[10px] text-indigo-400 hover:underline"
                >
                  Reset
                </button>
              </div>
              <input
                type="range"
                min="0.5"
                max="2.0"
                step="0.05"
                value={speedScale}
                onChange={(e) => setSpeedScale(parseFloat(e.target.value))}
                className="w-full accent-indigo-500"
              />
            </div>

            <div>
              <div className="flex justify-between text-[11px] text-zinc-400">
                <span>Noise Scale (Phoneme variation): {noiseScale.toFixed(3)}</span>
              </div>
              <input
                type="range"
                min="0.0"
                max="1.5"
                step="0.05"
                value={noiseScale}
                onChange={(e) => setNoiseScale(parseFloat(e.target.value))}
                className="w-full accent-indigo-500"
              />
            </div>

            <div>
              <div className="flex justify-between text-[11px] text-zinc-400">
                <span>Noise Width: {noiseW.toFixed(2)}</span>
              </div>
              <input
                type="range"
                min="0.0"
                max="1.5"
                step="0.05"
                value={noiseW}
                onChange={(e) => setNoiseW(parseFloat(e.target.value))}
                className="w-full accent-indigo-500"
              />
            </div>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="space-y-2">
        {isSynthesizing ? (
          <div className="space-y-2">
            <div className="flex justify-between text-xs text-zinc-300">
              <span>{statusMessage}</span>
              <span>{Math.round(progressPct)}%</span>
            </div>
            <div className="w-full bg-zinc-800 rounded-full h-1.5 overflow-hidden">
              <div
                className="bg-indigo-500 h-full transition-all duration-200"
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <Button
              size="xs"
              variant="secondary"
              onClick={handleCancelSynthesis}
              className="w-full text-rose-300 hover:bg-rose-950/40"
            >
              Cancel Synthesis
            </Button>
          </div>
        ) : (
          <Button
            size="xs"
            variant="primary"
            onClick={handleStartSynthesis}
            disabled={!selectedVoice?.isInstalled}
            className="w-full bg-indigo-600 hover:bg-indigo-500 text-white font-medium"
          >
            <SparklesIcon size={13} className="mr-1.5" />
            <span>Generate Speech</span>
          </Button>
        )}
      </div>

      {/* Errors & Feedback */}
      {errorMsg && (
        <div className="p-2.5 rounded bg-rose-950/40 border border-rose-900/60 text-rose-300 text-xs flex items-start space-x-2">
          <AlertTriangleIcon size={14} className="shrink-0 mt-0.5 text-rose-400" />
          <span>{errorMsg}</span>
        </div>
      )}

      {exportFeedback && (
        <div className="p-2.5 rounded bg-emerald-950/40 border border-emerald-900/60 text-emerald-300 text-xs flex items-center space-x-2">
          <CheckIcon size={14} className="shrink-0 text-emerald-400" />
          <span className="truncate">{exportFeedback}</span>
        </div>
      )}

      {/* Synthesis Result & Audio Player Preview */}
      {ttsResult && (
        <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/50 space-y-3">
          <div className="flex items-center justify-between text-xs">
            <span className="font-semibold text-zinc-200">Generated Audio</span>
            <span className="text-[11px] text-zinc-400 font-mono">
              {(ttsResult.totalDurationUs / 1_000_000).toFixed(2)}s • {ttsResult.segments.length} segment(s)
            </span>
          </div>

          {activeSegment && (
            <div className="p-2.5 rounded bg-zinc-950 border border-zinc-800/80 space-y-2">
              <div className="text-xs text-zinc-300 italic">
                &ldquo;{activeSegment.originalText}&rdquo;
              </div>

              {/* HTML5 audio element with media source */}
              <audio
                ref={audioRef}
                src={`opentake-media://${encodeURIComponent(activeSegment.stagedAudioPath)}`}
                onEnded={() => setIsPlaying(false)}
                className="hidden"
              />

              <div className="flex items-center justify-between">
                <Button
                  size="xs"
                  variant="secondary"
                  onClick={handlePlayToggle}
                  className="text-xs flex items-center space-x-1"
                >
                  {isPlaying ? <PauseIcon size={12} /> : <PlayIcon size={12} />}
                  <span>{isPlaying ? "Pause" : "Play Segment"}</span>
                </Button>
                <span className="text-[10px] text-zinc-400 font-mono">
                  {(activeSegment.durationUs / 1_000_000).toFixed(2)}s
                </span>
              </div>

              {/* Timing difference notice if applicable */}
              {activeSegment.timingDifferenceUs !== undefined &&
                Math.abs(activeSegment.timingDifferenceUs) > 500_000 && (
                  <div className="text-[10px] text-amber-400 flex items-center space-x-1">
                    <AlertTriangleIcon size={10} />
                    <span>
                      Audio differs from cue display by{" "}
                      {(activeSegment.timingDifferenceUs / 1_000_000).toFixed(2)}s
                    </span>
                  </div>
                )}
            </div>
          )}

          {/* Segment List */}
          {ttsResult.segments.length > 1 && (
            <div className="space-y-1">
              <label className="text-[10px] text-zinc-400 uppercase font-semibold">Segments</label>
              <div className="max-h-36 overflow-y-auto space-y-1 pr-1">
                {ttsResult.segments.map((seg, idx) => (
                  <div
                    key={seg.id}
                    onClick={() => handleSegmentSelect(idx)}
                    className={`p-1.5 rounded text-[11px] cursor-pointer flex items-center justify-between border transition-colors ${
                      idx === activeSegmentIndex
                        ? "bg-indigo-950/40 border-indigo-800 text-indigo-200"
                        : "bg-zinc-900 border-zinc-800/60 text-zinc-400 hover:text-zinc-200"
                    }`}
                  >
                    <span className="truncate max-w-[180px]">{seg.originalText}</span>
                    <span className="font-mono text-[10px] shrink-0">
                      {(seg.durationUs / 1_000_000).toFixed(2)}s
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Export Actions */}
          <div className="flex space-x-2 pt-1">
            <Button
              size="xs"
              variant="secondary"
              onClick={() => activeSegment && handleExportSegment(activeSegment.id)}
              disabled={!activeSegment}
              className="flex-1 text-xs"
            >
              <DownloadIcon size={12} className="mr-1" />
              <span>Export Segment WAV</span>
            </Button>
            <Button
              size="xs"
              variant="primary"
              onClick={handleExportAll}
              className="flex-1 text-xs bg-indigo-600 hover:bg-indigo-500 text-white"
            >
              <DownloadIcon size={12} className="mr-1" />
              <span>Export Full WAV</span>
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
