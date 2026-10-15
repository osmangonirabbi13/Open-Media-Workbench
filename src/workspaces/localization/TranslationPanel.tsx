import React, { useState, useEffect } from "react";
import type { SubtitleDocument } from "../../types/subtitles";
import type {
  TranslationRequest,
  TranslationResult,
  TranslationRuntimeCapabilities,
  TranslationStatus,
} from "../../types/translation";
import {
  getTranslationCapabilities,
  startOfflineTranslation,
  getOfflineTranslationStatus,
  cancelOfflineTranslation,
  getOfflineTranslationResult,
  applyTranslationResultToSubtitles,
} from "../../lib/translation";
import { formatMicrosToTimecode } from "../../lib/subtitles";

interface TranslationPanelProps {
  document: SubtitleDocument | null;
  onUpdateDocument: (doc: SubtitleDocument) => void;
  onOpenModelManager?: () => void;
}

export const TranslationPanel: React.FC<TranslationPanelProps> = ({
  document,
  onUpdateDocument,
  onOpenModelManager,
}) => {
  const [capabilities, setCapabilities] = useState<TranslationRuntimeCapabilities | null>(null);
  const [selectedTrackId, setSelectedTrackId] = useState<string>("");
  const [sourceLang, setSourceLang] = useState<string>("en");
  const [targetLang, setTargetLang] = useState<string>("es");
  const [selectedModelId, setSelectedModelId] = useState<string>("qwen2.5-0.5b-instruct");
  const [customTrackName, setCustomTrackName] = useState<string>("");

  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [jobStatus, setJobStatus] = useState<TranslationStatus | null>(null);
  const [translationResult, setTranslationResult] = useState<TranslationResult | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [isApplying, setIsApplying] = useState(false);

  // Load capabilities on mount
  useEffect(() => {
    getTranslationCapabilities()
      .then((caps) => {
        setCapabilities(caps);
      })
      .catch((err) => {
        setErrorMsg(`Failed to probe translation runtime: ${err}`);
      });
  }, []);

  // Update selectedTrackId when document changes
  useEffect(() => {
    if (document && document.tracks.length > 0) {
      if (!selectedTrackId || !document.tracks.some((t) => t.id === selectedTrackId)) {
        setSelectedTrackId(document.activeTrackId || document.tracks[0].id);
      }
    }
  }, [document, selectedTrackId]);

  // Polling active job
  useEffect(() => {
    if (!activeJobId) return;

    let isMounted = true;
    const interval = setInterval(async () => {
      try {
        const status = await getOfflineTranslationStatus(activeJobId);
        if (!isMounted || !status) return;

        setJobStatus(status);

        if (status.stage === "completed") {
          clearInterval(interval);
          const result = await getOfflineTranslationResult(activeJobId);
          if (isMounted && result) {
            setTranslationResult(result);
            setActiveJobId(null);
          }
        } else if (status.stage === "failed" || status.stage === "cancelled") {
          clearInterval(interval);
          setActiveJobId(null);
          if (status.error) {
            setErrorMsg(status.error);
          }
        }
      } catch (err) {
        if (isMounted) {
          setErrorMsg(`Error checking job status: ${err}`);
          clearInterval(interval);
          setActiveJobId(null);
        }
      }
    }, 500);

    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, [activeJobId]);

  const activeTrack = document?.tracks.find((t) => t.id === selectedTrackId);
  const cueCount = activeTrack?.cues.length ?? 0;

  // Filter available target languages for chosen source language
  const availablePairs = (capabilities?.supportedLanguagePairs || []).filter(
    (p) => p.sourceCode.toLowerCase() === sourceLang.toLowerCase()
  );

  const handleStartTranslation = async () => {
    if (!document || !activeTrack || activeTrack.cues.length === 0) {
      setErrorMsg("Please select a subtitle track with at least one cue.");
      return;
    }

    setErrorMsg(null);
    setTranslationResult(null);

    const request: TranslationRequest = {
      documentId: document.id,
      trackId: activeTrack.id,
      sourceLanguage: sourceLang,
      targetLanguage: targetLang,
      modelId: selectedModelId,
      batchSize: 1,
    };

    try {
      const jobId = await startOfflineTranslation(request, activeTrack.cues);
      setActiveJobId(jobId);
      setJobStatus({
        jobId,
        stage: "queued",
        progressPercent: 0,
        message: "Starting translation...",
        translatedCues: 0,
        totalCues: activeTrack.cues.length,
      });
    } catch (err) {
      setErrorMsg(`Failed to start translation: ${err}`);
    }
  };

  const handleCancelTranslation = async () => {
    if (!activeJobId) return;
    try {
      await cancelOfflineTranslation(activeJobId);
    } catch (err) {
      setErrorMsg(`Failed to cancel: ${err}`);
    }
  };

  const handleApplyResult = async () => {
    if (!document || !translationResult) return;
    setIsApplying(true);
    try {
      const updatedDoc = await applyTranslationResultToSubtitles(
        translationResult,
        document,
        customTrackName.trim() || undefined
      );
      onUpdateDocument(updatedDoc);
      setTranslationResult(null);
      setCustomTrackName("");
    } catch (err) {
      setErrorMsg(`Failed to apply translated track: ${err}`);
    } finally {
      setIsApplying(false);
    }
  };

  const isBusy = activeJobId !== null;

  return (
    <div className="flex flex-col h-full bg-neutral-900 text-neutral-200 overflow-y-auto p-4 space-y-5 text-sm">
      {/* Header */}
      <div>
        <div className="flex items-center justify-between">
          <h2 className="text-base font-semibold text-neutral-100 flex items-center gap-2">
            <svg
              className="w-5 h-5 text-indigo-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
              />
            </svg>
            Offline Subtitle Translation
          </h2>
          <span className="text-xs bg-indigo-950 text-indigo-300 border border-indigo-800 rounded px-2 py-0.5 font-medium">
            STEP 22
          </span>
        </div>
        <p className="text-xs text-neutral-400 mt-1">
          Translate subtitle tracks locally on CPU using verified offline AI models. Zero cloud APIs,
          strict timing preservation, and non-destructive track creation.
        </p>
      </div>

      {/* Runtime Readiness Banner */}
      <div
        className={`rounded-lg p-3 border text-xs flex items-start gap-2.5 ${
          capabilities?.isAvailable
            ? "bg-emerald-950/40 border-emerald-800/70 text-emerald-200"
            : "bg-amber-950/40 border-amber-800/70 text-amber-200"
        }`}
      >
        <span className="mt-0.5">
          {capabilities?.isAvailable ? "✓" : "⚠"}
        </span>
        <div className="flex-1">
          <div className="font-medium">
            {capabilities?.isAvailable
              ? `Local Translation Engine Ready: ${capabilities.runtimeVersion || "llama.cpp"}`
              : "Translation Runtime Missing"}
          </div>
          <div className="text-neutral-400 mt-0.5">
            {capabilities?.isAvailable
              ? `Discovered executable at: ${capabilities.runtimePath || "system PATH"}`
              : "Please install or select a valid llama-cli executable to run local inference."}
          </div>
        </div>
        {onOpenModelManager && (
          <button
            onClick={onOpenModelManager}
            className="text-xs text-indigo-400 hover:text-indigo-300 underline font-medium whitespace-nowrap self-center"
          >
            AI Models ↗
          </button>
        )}
      </div>

      {/* Error Alert */}
      {errorMsg && (
        <div className="rounded-lg p-3 bg-red-950/50 border border-red-800 text-red-200 text-xs flex items-start justify-between">
          <span>{errorMsg}</span>
          <button
            onClick={() => setErrorMsg(null)}
            className="text-red-400 hover:text-red-100 ml-2 font-bold"
          >
            ×
          </button>
        </div>
      )}

      {/* Track Selection & Language Options */}
      <div className="bg-neutral-800/60 rounded-lg p-3 border border-neutral-700/60 space-y-3">
        <h3 className="text-xs font-semibold text-neutral-300 uppercase tracking-wider">
          1. Source Track & Languages
        </h3>

        <div className="grid grid-cols-1 gap-3">
          {/* Track selection */}
          <div>
            <label className="block text-xs font-medium text-neutral-400 mb-1">
              Subtitle Track to Translate
            </label>
            <select
              value={selectedTrackId}
              onChange={(e) => setSelectedTrackId(e.target.value)}
              disabled={isBusy || !document?.tracks.length}
              className="w-full bg-neutral-900 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-indigo-500"
            >
              {document?.tracks.map((track) => (
                <option key={track.id} value={track.id}>
                  {track.displayName} ({track.language.toUpperCase()}) — {track.cues.length} cues
                </option>
              ))}
            </select>
          </div>

          {/* Languages */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-xs font-medium text-neutral-400 mb-1">
                Source Language
              </label>
              <select
                value={sourceLang}
                onChange={(e) => setSourceLang(e.target.value)}
                disabled={isBusy}
                className="w-full bg-neutral-900 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-indigo-500"
              >
                <option value="en">English (en)</option>
                <option value="es">Spanish (es)</option>
                <option value="bn">Bengali (বাংলা)</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-medium text-neutral-400 mb-1">
                Target Language
              </label>
              <select
                value={targetLang}
                onChange={(e) => setTargetLang(e.target.value)}
                disabled={isBusy}
                className="w-full bg-neutral-900 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-indigo-500"
              >
                {availablePairs.map((pair) => (
                  <option key={pair.targetCode} value={pair.targetCode}>
                    {pair.targetName} {pair.isVerified ? "★" : ""}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>

        {/* Selected Pair Badge */}
        <div className="text-xs text-neutral-400 flex items-center justify-between pt-1 border-t border-neutral-700/40">
          <span>
            Selected: <strong className="text-neutral-200">{sourceLang.toUpperCase()}</strong> →{" "}
            <strong className="text-neutral-200">{targetLang.toUpperCase()}</strong>
          </span>
          <span className="text-[11px] text-emerald-400 bg-emerald-950/60 px-2 py-0.5 rounded border border-emerald-800/60 font-medium">
            Timing Preservation Guaranteed
          </span>
        </div>
      </div>

      {/* Model Selection */}
      <div className="bg-neutral-800/60 rounded-lg p-3 border border-neutral-700/60 space-y-3">
        <h3 className="text-xs font-semibold text-neutral-300 uppercase tracking-wider">
          2. Verified Offline Model
        </h3>

        <div>
          <label className="block text-xs font-medium text-neutral-400 mb-1">
            Translation Model
          </label>
          <select
            value={selectedModelId}
            onChange={(e) => setSelectedModelId(e.target.value)}
            disabled={isBusy}
            className="w-full bg-neutral-900 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-indigo-500"
          >
            <option value="qwen2.5-0.5b-instruct">
              Qwen 2.5 0.5B Instruct (GGUF Q4_K_M) — 468 MB [Verified]
            </option>
          </select>
          <p className="text-[11px] text-neutral-400 mt-1.5">
            Runs offline on CPU via llama.cpp without sending text over the internet.
          </p>
        </div>
      </div>

      {/* Action and Live Progress */}
      <div className="space-y-3">
        {!isBusy && !translationResult && (
          <button
            onClick={handleStartTranslation}
            disabled={!capabilities?.isAvailable || cueCount === 0}
            className={`w-full py-2.5 px-4 rounded-lg font-medium text-xs flex items-center justify-center gap-2 transition-colors ${
              capabilities?.isAvailable && cueCount > 0
                ? "bg-indigo-600 hover:bg-indigo-500 text-white cursor-pointer shadow-md"
                : "bg-neutral-800 text-neutral-500 cursor-not-allowed"
            }`}
          >
            <svg
              className="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M13 10V3L4 14h7v7l9-11h-7z"
              />
            </svg>
            Start Translation ({cueCount} Cues)
          </button>
        )}

        {/* In-Flight Progress */}
        {isBusy && (
          <div className="bg-neutral-800/80 rounded-lg p-4 border border-indigo-800/50 space-y-3">
            <div className="flex items-center justify-between text-xs">
              <span className="font-medium text-indigo-300 flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-indigo-400 animate-ping" />
                {jobStatus?.message || "Translating subtitles..."}
              </span>
              <span className="text-neutral-400 font-mono">
                {jobStatus?.translatedCues} / {jobStatus?.totalCues} cues
              </span>
            </div>

            <div className="w-full bg-neutral-900 rounded-full h-2 overflow-hidden border border-neutral-700">
              <div
                className="bg-indigo-500 h-full transition-all duration-300"
                style={{ width: `${jobStatus?.progressPercent || 5}%` }}
              />
            </div>

            <div className="flex justify-end">
              <button
                onClick={handleCancelTranslation}
                className="text-xs text-red-400 hover:text-red-300 bg-red-950/40 border border-red-800/60 px-3 py-1 rounded"
              >
                Cancel Translation
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Translation Result Review & Apply */}
      {translationResult && (
        <div className="bg-neutral-800/60 rounded-lg p-3 border border-emerald-800/60 space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-xs font-semibold text-emerald-300 flex items-center gap-1.5">
                <span>✓</span> Translation Completed ({translationResult.translatedCues.length} Cues)
              </h3>
              <p className="text-[11px] text-neutral-400 mt-0.5">
                Elapsed: {translationResult.elapsedMs} ms • All timestamps preserved exactly
              </p>
            </div>
            <button
              onClick={() => setTranslationResult(null)}
              className="text-xs text-neutral-500 hover:text-neutral-300"
            >
              Dismiss
            </button>
          </div>

          {/* Cue preview table */}
          <div className="max-h-60 overflow-y-auto border border-neutral-700/60 rounded divide-y divide-neutral-800 text-xs">
            {translationResult.translatedCues.slice(0, 15).map((cue, idx) => (
              <div key={cue.translatedCueId} className="p-2 space-y-1 hover:bg-neutral-800/40">
                <div className="flex items-center justify-between text-[11px] text-neutral-500 font-mono">
                  <span>#{idx + 1}</span>
                  <span>
                    {formatMicrosToTimecode(cue.targetStartUs)} → {formatMicrosToTimecode(cue.targetEndUs)}
                  </span>
                </div>
                <div className="text-neutral-400 line-clamp-1 italic">
                  "{cue.originalText}"
                </div>
                <div className="text-neutral-100 font-medium">
                  {cue.translatedText}
                </div>
                {cue.warnings.length > 0 && (
                  <div className="text-[10px] text-amber-400">
                    ⚠ {cue.warnings.join(", ")}
                  </div>
                )}
              </div>
            ))}
            {translationResult.translatedCues.length > 15 && (
              <div className="p-2 text-center text-xs text-neutral-500 italic">
                + {translationResult.translatedCues.length - 15} more cues...
              </div>
            )}
          </div>

          {/* Track name & Apply button */}
          <div className="space-y-2 pt-2 border-t border-neutral-700/60">
            <div>
              <label className="block text-[11px] font-medium text-neutral-400 mb-1">
                New Subtitle Track Name
              </label>
              <input
                type="text"
                value={customTrackName}
                onChange={(e) => setCustomTrackName(e.target.value)}
                placeholder={`Translated (${targetLang.toUpperCase()})`}
                className="w-full bg-neutral-900 border border-neutral-700 rounded px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-emerald-500"
              />
            </div>

            <button
              onClick={handleApplyResult}
              disabled={isApplying}
              className="w-full py-2 px-3 bg-emerald-600 hover:bg-emerald-500 text-white rounded font-medium text-xs flex items-center justify-center gap-1.5 shadow transition-colors cursor-pointer"
            >
              {isApplying ? "Adding Track..." : "Add as New Subtitle Track"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
