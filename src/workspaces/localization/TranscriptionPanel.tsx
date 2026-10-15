import React, { useState, useEffect, useCallback, useRef } from "react";
import type { MediaAsset } from "../../types/media";
import type { SubtitleDocument } from "../../types/subtitles";
import type {
  TranscriptionProgressStage,
  TranscriptionResult,
  WhisperRuntimeCapabilities,
} from "../../types/transcription";
import type { AiModelSummary } from "../../types/ai";
import {
  getWhisperRuntimeCapabilities,
  selectWhisperRuntime,
  startOfflineTranscription,
  getOfflineTranscriptionStatus,
  cancelOfflineTranscription,
  getOfflineTranscriptionResult,
  applyTranscriptionResultToSubtitles,
} from "../../lib/transcription";
import { listAiModels } from "../../lib/aiManager";
import { formatMicrosToTimecode } from "../../lib/subtitles";
import {
  MicIcon,
  SparklesIcon,
  CheckIcon,
  ShieldAlertIcon,
  ShieldCheckIcon,
  CpuIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";

export interface TranscriptionPanelProps {
  asset: MediaAsset | null;
  document: SubtitleDocument | null;
  onDocumentChange: (doc: SubtitleDocument) => void;
  onOpenModelManager?: () => void;
  onClose?: () => void;
}

export function TranscriptionPanel({
  asset,
  document,
  onDocumentChange,
  onOpenModelManager,
  onClose,
}: TranscriptionPanelProps): React.ReactElement {
  // Runtime and Models
  const [runtimeCaps, setRuntimeCaps] = useState<WhisperRuntimeCapabilities | null>(null);
  const [models, setModels] = useState<AiModelSummary[]>([]);
  const [selectedModelId, setSelectedModelId] = useState<string>("");
  const [selectedLanguage, setSelectedLanguage] = useState<string>("auto");
  const [customTrackName, setCustomTrackName] = useState<string>("");

  // Job Execution State
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [jobStage, setJobStage] = useState<TranscriptionProgressStage | null>(null);
  const [jobProgressPercent, setJobProgressPercent] = useState<number>(0);
  const [jobMessage, setJobMessage] = useState<string | null>(null);
  const [jobError, setJobError] = useState<string | null>(null);

  // Result Preview
  const [result, setResult] = useState<TranscriptionResult | null>(null);
  const [isApplying, setIsApplying] = useState(false);
  const [isSelectingRuntime, setIsSelectingRuntime] = useState(false);

  const pollIntervalRef = useRef<NodeJS.Timeout | null>(null);

  // Load models and runtime capabilities
  const loadPrerequisites = useCallback(async () => {
    try {
      const [caps, allModels] = await Promise.all([
        getWhisperRuntimeCapabilities(),
        listAiModels(),
      ]);
      setRuntimeCaps(caps);

      // Filter for speech-to-text whisper models
      const whisperModels = allModels.filter(
        (m) => m.manifest.capability === "speechToText" && m.manifest.family === "whisper",
      );
      setModels(whisperModels);

      // Default to first installed model if available
      const installed = whisperModels.find((m) => m.status === "installed");
      if (installed) {
        setSelectedModelId(installed.manifest.id);
      } else if (whisperModels.length > 0) {
        setSelectedModelId(whisperModels[0].manifest.id);
      }
    } catch (err) {
      console.error("Failed to load transcription prerequisites:", err);
    }
  }, []);

  useEffect(() => {
    loadPrerequisites();
  }, [loadPrerequisites]);

  // Clean up polling interval
  useEffect(() => {
    return () => {
      if (pollIntervalRef.current) {
        clearInterval(pollIntervalRef.current);
      }
    };
  }, []);

  const selectedModel = models.find((m) => m.manifest.id === selectedModelId);
  const isSelectedModelInstalled = selectedModel?.status === "installed";
  const isRuntimeReady = runtimeCaps?.isAvailable ?? false;

  // Multi-lingual vs English-only check
  const isEnglishOnlyModel = selectedModel?.manifest.id.endsWith(".en") ?? false;

  // Handle language switch
  const handleLanguageChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setSelectedLanguage(e.target.value);
  };

  // Select custom runtime executable
  const handleSelectCustomRuntime = async () => {
    setIsSelectingRuntime(true);
    try {
      const caps = await selectWhisperRuntime();
      setRuntimeCaps(caps);
    } catch (err) {
      setJobError(`Failed to select custom runtime: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsSelectingRuntime(false);
    }
  };

  // Start transcription job
  const handleStartTranscription = async () => {
    if (!asset || !selectedModelId || !isSelectedModelInstalled || !isRuntimeReady) {
      return;
    }

    setJobError(null);
    setResult(null);
    setJobStage("queued");
    setJobProgressPercent(0);
    setJobMessage("Initiating transcription job...");

    try {
      const jobId = await startOfflineTranscription({
        assetId: asset.id,
        modelId: selectedModelId,
        language: selectedLanguage,
        outputTrackName: customTrackName.trim() || undefined,
      });

      setActiveJobId(jobId);

      // Start polling status
      pollIntervalRef.current = setInterval(async () => {
        try {
          const status = await getOfflineTranscriptionStatus(jobId);
          if (!status) return;

          setJobStage(status.stage);
          if (status.progressPercent !== undefined) {
            setJobProgressPercent(status.progressPercent);
          }
          if (status.message) {
            setJobMessage(status.message);
          }

          if (status.stage === "completed") {
            if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
            const res = await getOfflineTranscriptionResult(jobId);
            setResult(res);
            setActiveJobId(null);
          } else if (status.stage === "failed" || status.stage === "cancelled") {
            if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
            setActiveJobId(null);
            if (status.error) {
              setJobError(status.error);
            }
          }
        } catch (err) {
          if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
          setActiveJobId(null);
          setJobError(`Status poll error: ${err instanceof Error ? err.message : String(err)}`);
        }
      }, 300);
    } catch (err) {
      setJobStage("failed");
      setJobError(err instanceof Error ? err.message : String(err));
    }
  };

  // Cancel in-flight transcription
  const handleCancelTranscription = async () => {
    if (!activeJobId) return;
    try {
      await cancelOfflineTranscription(activeJobId);
      setJobMessage("Cancelling transcription job...");
    } catch (err) {
      setJobError(`Cancellation error: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  // Apply result to SubtitleDocument
  const handleApplyResult = async () => {
    if (!result) return;
    setIsApplying(true);
    try {
      const updatedDoc = await applyTranscriptionResultToSubtitles(
        result.jobId,
        customTrackName.trim() || undefined,
        document,
      );
      onDocumentChange(updatedDoc);
      setResult(null);
      if (onClose) onClose();
    } catch (err) {
      setJobError(`Failed to apply subtitles: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsApplying(false);
    }
  };

  return (
    <div
      role="region"
      aria-label="Offline Speech-to-Text Transcription"
      className="flex flex-col h-full bg-zinc-950 text-zinc-100 overflow-y-auto p-4 space-y-4"
    >
      {/* Title & Privacy Guarantee Header */}
      <div className="border-b border-zinc-800 pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <MicIcon size={18} className="text-indigo-400" />
            <h2 className="text-sm font-semibold tracking-wide uppercase text-zinc-100">
              Offline Speech-to-Text
            </h2>
          </div>
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-emerald-950 text-emerald-400 border border-emerald-800/60">
            <ShieldCheckIcon size={11} />
            <span>100% On-Device</span>
          </span>
        </div>
        <p className="text-xs text-zinc-400 mt-1">
          Zero remote APIs, zero cloud uploads. Operates strictly offline on authorized local media.
        </p>
      </div>

      {/* Asset Notice */}
      {!asset ? (
        <div className="p-3 bg-amber-950/40 border border-amber-800/60 rounded-md text-amber-300 text-xs flex items-center space-x-2">
          <ShieldAlertIcon size={16} className="shrink-0" />
          <span>Please import or select a media asset in the Media Library to begin transcription.</span>
        </div>
      ) : (
        <div className="p-2.5 bg-zinc-900 border border-zinc-800 rounded-md text-xs flex items-center justify-between">
          <span className="text-zinc-400">Target Asset:</span>
          <span className="font-medium text-zinc-200 truncate max-w-[200px]" title={asset.fileName}>
            {asset.fileName}
          </span>
        </div>
      )}

      {/* Runtime Status Banner */}
      <div
        className={`p-3 rounded-md border text-xs flex flex-col space-y-2 ${
          isRuntimeReady
            ? "bg-zinc-900/60 border-zinc-800 text-zinc-300"
            : "bg-red-950/30 border-red-900/60 text-red-300"
        }`}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <CpuIcon size={14} className={isRuntimeReady ? "text-emerald-400" : "text-red-400"} />
            <span className="font-medium">
              Whisper Runtime: {isRuntimeReady ? "Operational" : "Not Found"}
            </span>
          </div>
          {isRuntimeReady && runtimeCaps?.runtimeVersion && (
            <span className="text-[10px] text-zinc-400 font-mono">
              {runtimeCaps.runtimeVersion}
            </span>
          )}
        </div>

        {!isRuntimeReady ? (
          <div className="space-y-2">
            <p className="text-[11px] text-zinc-400 leading-relaxed">
              No local <code className="text-zinc-300 bg-zinc-900 px-1 py-0.5 rounded">whisper-cli</code> binary was discovered. Install whisper.cpp on your system or select an authorized local executable.
            </p>
            <Button
              size="sm"
              variant="secondary"
              onClick={handleSelectCustomRuntime}
              disabled={isSelectingRuntime}
              className="text-xs"
            >
              {isSelectingRuntime ? "Selecting..." : "Select Whisper Executable..."}
            </Button>
          </div>
        ) : (
          runtimeCaps?.runtimePath && (
            <span className="text-[10px] text-zinc-500 font-mono truncate" title={runtimeCaps.runtimePath}>
              Path: {runtimeCaps.runtimePath}
            </span>
          )
        )}
      </div>

      {/* Model Selection */}
      <div className="space-y-1.5">
        <div className="flex items-center justify-between">
          <label htmlFor="transcription-model-select" className="text-xs font-medium text-zinc-300">
            Whisper Model:
          </label>
          {onOpenModelManager && (
            <button
              type="button"
              onClick={onOpenModelManager}
              className="text-[11px] text-indigo-400 hover:text-indigo-300 underline underline-offset-2"
            >
              Manage AI Models
            </button>
          )}
        </div>

        {models.length === 0 ? (
          <div className="p-2.5 bg-zinc-900 border border-zinc-800 rounded-md text-xs text-zinc-400">
            No compatible Whisper models found in catalog.
          </div>
        ) : (
          <select
            id="transcription-model-select"
            value={selectedModelId}
            onChange={(e) => setSelectedModelId(e.target.value)}
            disabled={Boolean(activeJobId)}
            className="w-full bg-zinc-900 border border-zinc-800 rounded-md px-2.5 py-1.5 text-xs text-zinc-100 outline-none focus:border-indigo-500"
          >
            {models.map((m) => (
              <option key={m.manifest.id} value={m.manifest.id}>
                {m.manifest.name} ({m.status === "installed" ? "Installed" : "Not Installed"})
              </option>
            ))}
          </select>
        )}

        {/* Model Installation Warning */}
        {selectedModel && !isSelectedModelInstalled && (
          <div className="p-2.5 bg-amber-950/40 border border-amber-800/60 rounded-md text-amber-300 text-xs flex items-center justify-between">
            <span>Model '{selectedModel.manifest.name}' is not yet downloaded.</span>
            {onOpenModelManager && (
              <Button size="sm" variant="secondary" onClick={onOpenModelManager} className="text-xs">
                Install Model
              </Button>
            )}
          </div>
        )}
      </div>

      {/* Language Selection */}
      <div className="space-y-1.5">
        <label htmlFor="transcription-lang-select" className="text-xs font-medium text-zinc-300">
          Spoken Language:
        </label>
        <select
          id="transcription-lang-select"
          value={selectedLanguage}
          onChange={handleLanguageChange}
          disabled={Boolean(activeJobId) || isEnglishOnlyModel}
          className="w-full bg-zinc-900 border border-zinc-800 rounded-md px-2.5 py-1.5 text-xs text-zinc-100 outline-none focus:border-indigo-500 disabled:opacity-60"
        >
          {runtimeCaps?.supportedLanguages.map((l) => (
            <option key={l.code} value={l.code}>
              {l.label}
            </option>
          ))}
        </select>
        {isEnglishOnlyModel && (
          <p className="text-[11px] text-amber-400/90">
            Selected model is English-only. Multilingual transcription and Auto Detect are disabled.
          </p>
        )}
      </div>

      {/* Track Name Customization */}
      <div className="space-y-1.5">
        <label htmlFor="transcription-track-name" className="text-xs font-medium text-zinc-300">
          Generated Track Name (Optional):
        </label>
        <input
          id="transcription-track-name"
          type="text"
          value={customTrackName}
          onChange={(e) => setCustomTrackName(e.target.value)}
          placeholder={`AI Subtitles (${selectedLanguage})`}
          disabled={Boolean(activeJobId)}
          className="w-full bg-zinc-900 border border-zinc-800 rounded-md px-2.5 py-1.5 text-xs text-zinc-100 outline-none focus:border-indigo-500 disabled:opacity-60"
        />
      </div>

      {/* Error Message Display */}
      {jobError && (
        <div className="p-3 bg-red-950/50 border border-red-800 rounded-md text-red-200 text-xs space-y-1">
          <div className="flex items-center space-x-1.5 font-medium text-red-300">
            <ShieldAlertIcon size={14} />
            <span>Transcription Notice</span>
          </div>
          <p className="leading-relaxed">{jobError}</p>
        </div>
      )}

      {/* Active Job Progress View */}
      {activeJobId && (
        <div className="p-3 bg-zinc-900 border border-zinc-800 rounded-md space-y-2.5">
          <div className="flex items-center justify-between text-xs">
            <span className="font-medium text-zinc-200">
              Stage: {jobStage ? jobStage.toUpperCase() : "TRANSCRIBING"}
            </span>
            <span className="text-zinc-400 font-mono">{jobProgressPercent.toFixed(0)}%</span>
          </div>

          <div className="w-full bg-zinc-800 rounded-full h-1.5 overflow-hidden">
            <div
              className="bg-indigo-500 h-1.5 transition-all duration-300 ease-out"
              style={{ width: `${Math.max(jobProgressPercent, 8)}%` }}
            />
          </div>

          {jobMessage && <p className="text-[11px] text-zinc-400">{jobMessage}</p>}

          <Button
            size="sm"
            variant="danger"
            onClick={handleCancelTranscription}
            className="w-full text-xs"
          >
            Cancel Transcription
          </Button>
        </div>
      )}

      {/* Start Action Button */}
      {!activeJobId && !result && (
        <Button
          size="md"
          variant="primary"
          onClick={handleStartTranscription}
          disabled={!asset || !isSelectedModelInstalled || !isRuntimeReady}
          className="w-full justify-center space-x-2 text-xs font-semibold py-2"
        >
          <SparklesIcon size={14} />
          <span>Start Speech-to-Text</span>
        </Button>
      )}

      {/* Completed Result Preview & Subtitle Track Addition */}
      {result && (
        <div className="border border-zinc-800 bg-zinc-900/50 rounded-md p-3 space-y-3">
          <div className="flex items-center justify-between border-b border-zinc-800 pb-2">
            <div className="flex items-center space-x-1.5 text-xs font-semibold text-emerald-400">
              <CheckIcon size={15} />
              <span>Transcription Completed</span>
            </div>
            <span className="text-[10px] text-zinc-400">
              {result.segments.length} cues in {(result.processingDurationMs / 1000).toFixed(1)}s
            </span>
          </div>

          <div className="text-[11px] text-zinc-300 space-y-1">
            <div>
              <span className="text-zinc-500">Detected Language: </span>
              <span className="font-semibold uppercase text-zinc-200">
                {result.detectedLanguage || result.requestedLanguage}
              </span>
            </div>
          </div>

          {/* Cues Preview Table */}
          <div className="max-h-48 overflow-y-auto border border-zinc-800 rounded bg-zinc-950 divide-y divide-zinc-900 text-[11px]">
            {result.segments.map((seg) => (
              <div key={seg.id} className="p-2 space-y-0.5">
                <div className="text-[10px] font-mono text-zinc-500">
                  {formatMicrosToTimecode(seg.startUs)} → {formatMicrosToTimecode(seg.endUs)}
                </div>
                <div className="text-zinc-200 font-sans leading-relaxed select-text">
                  {seg.text}
                </div>
              </div>
            ))}
          </div>

          <div className="flex items-center space-x-2 pt-1">
            <Button
              size="sm"
              variant="primary"
              onClick={handleApplyResult}
              disabled={isApplying}
              className="flex-1 text-xs"
            >
              {isApplying ? "Adding Track..." : "Add as New Track"}
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setResult(null)}
              className="text-xs"
            >
              Discard
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
