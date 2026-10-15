//! Client bridge, mock fallbacks, and utilities for Offline Whisper Speech-to-Text Transcription.

import { invoke } from "@tauri-apps/api/core";
import type { SubtitleDocument, SubtitleTrack, SubtitleCue } from "../types/subtitles";
import type {
  TranscriptionRequest,
  TranscriptionResult,
  TranscriptionStatus,
  WhisperRuntimeCapabilities,
} from "../types/transcription";

function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// In-memory mock jobs for test and non-Tauri environments
const mockJobs = new Map<string, {
  status: TranscriptionStatus;
  result: TranscriptionResult;
}>();

export async function getWhisperRuntimeCapabilities(): Promise<WhisperRuntimeCapabilities> {
  if (isTauriEnvironment()) {
    return invoke<WhisperRuntimeCapabilities>("get_whisper_runtime_capabilities");
  }

  return {
    isAvailable: true,
    runtimePath: "/usr/bin/whisper-cli",
    runtimeVersion: "whisper.cpp (mock)",
    isCustom: false,
    supportedLanguages: [
      { code: "auto", label: "Auto Detect", isMultilingualOnly: true },
      { code: "en", label: "English", isMultilingualOnly: false },
      { code: "bn", label: "Bengali (বাংলা)", isMultilingualOnly: true },
      { code: "es", label: "Spanish (Español)", isMultilingualOnly: true },
      { code: "fr", label: "French (Français)", isMultilingualOnly: true },
      { code: "de", label: "German (Deutsch)", isMultilingualOnly: true },
      { code: "hi", label: "Hindi (हिन्दी)", isMultilingualOnly: true },
      { code: "ja", label: "Japanese (日本語)", isMultilingualOnly: true },
      { code: "zh", label: "Chinese (中文)", isMultilingualOnly: true },
    ],
    errors: [],
  };
}

export async function selectWhisperRuntime(): Promise<WhisperRuntimeCapabilities> {
  if (isTauriEnvironment()) {
    return invoke<WhisperRuntimeCapabilities>("select_whisper_runtime");
  }

  return {
    isAvailable: true,
    runtimePath: "/custom/bin/whisper-cli",
    runtimeVersion: "whisper.cpp v1.7.0 (custom mock)",
    isCustom: true,
    supportedLanguages: [
      { code: "auto", label: "Auto Detect", isMultilingualOnly: true },
      { code: "en", label: "English", isMultilingualOnly: false },
      { code: "bn", label: "Bengali (বাংলা)", isMultilingualOnly: true },
    ],
    errors: [],
  };
}

export async function startOfflineTranscription(
  request: TranscriptionRequest,
): Promise<string> {
  if (isTauriEnvironment()) {
    return invoke<string>("start_offline_transcription", { request });
  }

  const jobId = `mock-job-${Date.now()}`;
  const isBengali = request.language === "bn";

  const segments = isBengali
    ? [
        {
          id: `seg-${Date.now()}-1`,
          startUs: 0,
          endUs: 2500000,
          text: "ওপেন মিডিয়া ওয়ার্কবেঞ্চে স্বাগতম।",
          detectedLanguage: "bn",
          confidence: 0.98,
        },
        {
          id: `seg-${Date.now()}-2`,
          startUs: 2600000,
          endUs: 5200000,
          text: "এটি একটি নির্ভরযোগ্য অফলাইন ট্রান্সক্রিপশন ইঞ্জিন।",
          detectedLanguage: "bn",
          confidence: 0.95,
        },
      ]
    : [
        {
          id: `seg-${Date.now()}-1`,
          startUs: 0,
          endUs: 2500000,
          text: "Welcome to Open Media Workbench.",
          detectedLanguage: "en",
          confidence: 0.98,
        },
        {
          id: `seg-${Date.now()}-2`,
          startUs: 2600000,
          endUs: 5200000,
          text: "This is a reliable offline speech-to-text transcription engine.",
          detectedLanguage: "en",
          confidence: 0.95,
        },
      ];

  const result: TranscriptionResult = {
    schemaVersion: 1,
    jobId,
    assetId: request.assetId,
    modelId: request.modelId,
    runtimeVersion: "whisper.cpp (mock)",
    requestedLanguage: request.language,
    detectedLanguage: isBengali ? "bn" : "en",
    segments,
    processingDurationMs: 1250,
    audioDurationUs: 5500000,
    warnings: [],
  };

  mockJobs.set(jobId, {
    status: {
      jobId,
      stage: "completed",
      progressPercent: 100,
      message: "Transcription complete.",
    },
    result,
  });

  return jobId;
}

export async function getOfflineTranscriptionStatus(
  jobId: string,
): Promise<TranscriptionStatus | null> {
  if (isTauriEnvironment()) {
    return invoke<TranscriptionStatus | null>(
      "get_offline_transcription_status",
      { jobId },
    );
  }

  const job = mockJobs.get(jobId);
  return job ? job.status : null;
}

export async function cancelOfflineTranscription(jobId: string): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("cancel_offline_transcription", { jobId });
  }

  const job = mockJobs.get(jobId);
  if (job) {
    job.status.stage = "cancelled";
    job.status.message = "Cancelled by user.";
  }
}

export async function getOfflineTranscriptionResult(
  jobId: string,
): Promise<TranscriptionResult> {
  if (isTauriEnvironment()) {
    return invoke<TranscriptionResult>("get_offline_transcription_result", {
      jobId,
    });
  }

  const job = mockJobs.get(jobId);
  if (!job) {
    throw new Error(`Transcription job '${jobId}' not found.`);
  }

  return job.result;
}

export async function applyTranscriptionResultToSubtitles(
  jobId: string,
  customTrackName?: string,
  document?: SubtitleDocument | null,
): Promise<SubtitleDocument> {
  if (isTauriEnvironment()) {
    return invoke<SubtitleDocument>("apply_transcription_result_to_subtitles", {
      jobId,
      customTrackName,
      document,
    });
  }

  const job = mockJobs.get(jobId);
  if (!job) {
    throw new Error(`Transcription job '${jobId}' not found.`);
  }

  const lang = job.result.detectedLanguage || job.result.requestedLanguage;
  const cues: SubtitleCue[] = job.result.segments.map((seg) => ({
    id: seg.id,
    startUs: seg.startUs,
    endUs: seg.endUs,
    text: seg.text,
    customMetadata: seg.confidence !== undefined
      ? { confidence: seg.confidence.toFixed(3) }
      : undefined,
  }));

  const newTrack: SubtitleTrack = {
    id: `track-${Date.now()}`,
    language: lang,
    displayName: customTrackName || `AI Subtitles (${lang})`,
    sourceFormat: "nativeProject",
    cues,
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  if (document) {
    return {
      ...document,
      tracks: [...document.tracks, newTrack],
      updatedAtEpochMs: Date.now(),
    };
  }

  return {
    id: `doc-${Date.now()}`,
    associatedMediaAssetId: job.result.assetId,
    tracks: [newTrack],
    createdAtEpochMs: Date.now(),
    updatedAtEpochMs: Date.now(),
    schemaVersion: 1,
  };
}
