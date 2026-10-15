import { invoke } from "@tauri-apps/api/core";
import type { SubtitleCue, SubtitleDocument, SubtitleTrack } from "../types/subtitles";
import type {
  TranslationLanguagePair,
  TranslationRequest,
  TranslationResult,
  TranslationRuntimeCapabilities,
  TranslationStatus,
} from "../types/translation";

function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const mockJobs = new Map<
  string,
  {
    status: TranslationStatus;
    result: TranslationResult;
  }
>();

export async function getTranslationCapabilities(): Promise<TranslationRuntimeCapabilities> {
  if (isTauriEnvironment()) {
    return invoke<TranslationRuntimeCapabilities>("get_translation_capabilities");
  }

  return {
    isAvailable: true,
    runtimePath: "/usr/local/bin/llama-cli",
    runtimeVersion: "llama.cpp b11658 (mock)",
    supportedLanguagePairs: await listTranslationLanguagePairs(),
  };
}

export async function listTranslationLanguagePairs(): Promise<TranslationLanguagePair[]> {
  if (isTauriEnvironment()) {
    return invoke<TranslationLanguagePair[]>("list_translation_language_pairs");
  }

  return [
    {
      sourceCode: "en",
      sourceName: "English",
      targetCode: "es",
      targetName: "Spanish (Español)",
      compatibleModelIds: ["qwen2.5-0.5b-instruct", "opus-mt-en-es"],
      isVerified: true,
    },
    {
      sourceCode: "en",
      sourceName: "English",
      targetCode: "bn",
      targetName: "Bengali (বাংলা)",
      compatibleModelIds: ["qwen2.5-0.5b-instruct"],
      isVerified: true,
    },
    {
      sourceCode: "es",
      sourceName: "Spanish (Español)",
      targetCode: "en",
      targetName: "English",
      compatibleModelIds: ["qwen2.5-0.5b-instruct"],
      isVerified: true,
    },
    {
      sourceCode: "bn",
      sourceName: "Bengali (বাংলা)",
      targetCode: "en",
      targetName: "English",
      compatibleModelIds: ["qwen2.5-0.5b-instruct"],
      isVerified: true,
    },
    {
      sourceCode: "en",
      sourceName: "English",
      targetCode: "fr",
      targetName: "French (Français)",
      compatibleModelIds: ["qwen2.5-0.5b-instruct"],
      isVerified: false,
    },
    {
      sourceCode: "en",
      sourceName: "English",
      targetCode: "de",
      targetName: "German (Deutsch)",
      compatibleModelIds: ["qwen2.5-0.5b-instruct"],
      isVerified: false,
    },
    {
      sourceCode: "en",
      sourceName: "English",
      targetCode: "ja",
      targetName: "Japanese (日本語)",
      compatibleModelIds: ["qwen2.5-0.5b-instruct"],
      isVerified: false,
    },
  ];
}

export const getTranslationLanguagePairs = listTranslationLanguagePairs;

export async function selectTranslationRuntime(path: string): Promise<string> {
  if (isTauriEnvironment()) {
    return invoke<string>("select_translation_runtime", { path });
  }

  return `Validated runtime at ${path}: llama.cpp vMock (custom)`;
}

export async function startOfflineTranslation(
  request: TranslationRequest,
  cues: SubtitleCue[]
): Promise<string> {
  if (isTauriEnvironment()) {
    return invoke<string>("start_offline_translation", { request, cues });
  }

  const jobId = `mock-job-${Date.now()}`;
  const translatedCues = cues.map((cue) => {
    let mockTrans = `[${request.targetLanguage.toUpperCase()}] ${cue.text}`;
    if (request.targetLanguage === "es") {
      mockTrans = `Traducción: ${cue.text}`;
    } else if (request.targetLanguage === "bn") {
      mockTrans = `অনুবাদ: ${cue.text}`;
    }

    return {
      sourceCueId: cue.id,
      translatedCueId: `${cue.id}-tr-${request.targetLanguage}`,
      originalText: cue.text,
      translatedText: mockTrans,
      sourceStartUs: cue.startUs,
      sourceEndUs: cue.endUs,
      targetStartUs: cue.startUs,
      targetEndUs: cue.endUs,
      warnings: [],
      modelProvenance: request.modelId,
    };
  });

  const result: TranslationResult = {
    schemaVersion: 1,
    jobId,
    originalTrackId: request.trackId,
    modelId: request.modelId,
    runtimeVersion: "llama.cpp (mock)",
    sourceLanguage: request.sourceLanguage,
    targetLanguage: request.targetLanguage,
    translatedCues,
    untranslatedCueIds: [],
    warnings: [],
    elapsedMs: 250,
  };

  const status: TranslationStatus = {
    jobId,
    stage: "completed",
    progressPercent: 100,
    message: "Translation completed successfully (mock).",
    translatedCues: cues.length,
    totalCues: cues.length,
  };

  mockJobs.set(jobId, { status, result });
  return jobId;
}

export async function getOfflineTranslationStatus(
  jobId: string
): Promise<TranslationStatus | null> {
  if (isTauriEnvironment()) {
    return invoke<TranslationStatus | null>("get_offline_translation_status", { jobId });
  }

  const job = mockJobs.get(jobId);
  return job ? job.status : null;
}

export async function cancelOfflineTranslation(jobId: string): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("cancel_offline_translation", { jobId });
  }

  const job = mockJobs.get(jobId);
  if (job) {
    job.status.stage = "cancelled";
    job.status.message = "Job cancelled by user.";
  }
}

export async function getOfflineTranslationResult(
  jobId: string
): Promise<TranslationResult | null> {
  if (isTauriEnvironment()) {
    return invoke<TranslationResult | null>("get_offline_translation_result", { jobId });
  }

  const job = mockJobs.get(jobId);
  return job ? job.result : null;
}

export async function applyTranslationResultToSubtitles(
  result: TranslationResult,
  document: SubtitleDocument,
  customTrackName?: string
): Promise<SubtitleDocument> {
  if (isTauriEnvironment()) {
    return invoke<SubtitleDocument>("apply_translation_result_to_subtitles", {
      result,
      document,
      customTrackName,
    });
  }

  const fallbackName = `Translated (${result.targetLanguage.toUpperCase()})`;
  const displayName = customTrackName || fallbackName;
  const trackId = `track-tr-${Date.now()}`;

  const newTrack: SubtitleTrack = {
    id: trackId,
    language: result.targetLanguage,
    displayName,
    sourceFormat: "nativeProject",
    cues: result.translatedCues.map((c) => ({
      id: c.translatedCueId,
      startUs: c.targetStartUs,
      endUs: c.targetEndUs,
      text: c.translatedText,
      speaker: undefined,
      words: undefined, // Word timing from source is NOT valid for translated text
    })),
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  return {
    ...document,
    tracks: [...document.tracks, newTrack],
    activeTrackId: trackId,
    updatedAtEpochMs: Date.now(),
  };
}
