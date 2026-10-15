/**
 * Tauri IPC Client for Offline Text-to-Speech (TTS).
 */

import { invoke } from "@tauri-apps/api/core";
import type { SubtitleTrack } from "../types/subtitles";
import type {
  AiVoiceManifest,
  TtsRequest,
  TtsResult,
  TtsRuntimeCapabilities,
  TtsStatus,
} from "../types/tts";

function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

// In-memory mock state for testing outside Tauri
let mockJobCounter = 1;
const mockJobs = new Map<string, { status: TtsStatus; result?: TtsResult }>();

/**
 * Queries local TTS runtime availability and discovered binary path.
 */
export async function getTtsRuntimeCapabilities(): Promise<TtsRuntimeCapabilities> {
  if (!isTauriEnvironment()) {
    return {
      discovered: true,
      binaryPath: "/mock/bin/piper",
      version: "Mock Piper 1.0.0",
      supportedFormats: ["wav"],
    };
  }
  return invoke<TtsRuntimeCapabilities>("get_tts_runtime_capabilities");
}

/**
 * Opens native file dialog to choose a custom Piper binary.
 */
export async function selectTtsRuntime(): Promise<TtsRuntimeCapabilities> {
  if (!isTauriEnvironment()) {
    return {
      discovered: true,
      binaryPath: "/custom/mock/piper",
      version: "Custom Mock Piper 1.0.0",
      supportedFormats: ["wav"],
    };
  }
  return invoke<TtsRuntimeCapabilities>("select_tts_runtime");
}

/**
 * Lists all supported voices in the local catalog.
 */
export async function listAvailableTtsVoices(): Promise<AiVoiceManifest[]> {
  if (!isTauriEnvironment()) {
    return [
      {
        id: "piper-en-lessac-medium",
        displayName: "English (US) - Lessac (Medium)",
        language: "en-US",
        locale: "en_US",
        voiceFamily: "piper",
        modelId: "piper-en-lessac-medium",
        sampleRate: 22050,
        speakerCount: 1,
        isInstalled: true,
        runtimeReady: true,
        license: "Public Domain",
        description: "High quality neural English voice suitable for narration and dialogue.",
      },
    ];
  }
  return invoke<AiVoiceManifest[]>("list_available_tts_voices");
}

/**
 * Retrieves the status and manifest for a specific voice.
 */
export async function getTtsVoiceStatus(voiceId: string): Promise<AiVoiceManifest> {
  if (!isTauriEnvironment()) {
    const voices = await listAvailableTtsVoices();
    const found = voices.find((v) => v.id === voiceId);
    if (!found) throw new Error(`Voice '${voiceId}' not found`);
    return found;
  }
  return invoke<AiVoiceManifest>("get_tts_voice_status", { voiceId });
}

/**
 * Initiates an asynchronous speech synthesis job.
 */
export async function startOfflineTts(
  request: TtsRequest,
  track?: SubtitleTrack
): Promise<string> {
  if (!isTauriEnvironment()) {
    const jobId = `mock_tts_${mockJobCounter++}`;

    const textSegments: Array<{ text: string; cueId?: string; startUs?: number; endUs?: number }> = [];

    if (request.source.type === "script") {
      const parts = request.source.text.split(/[.!?।\n]+/).map((s) => s.trim()).filter(Boolean);
      for (const p of parts) {
        textSegments.push({ text: p });
      }
      if (textSegments.length === 0 && request.source.text.trim()) {
        textSegments.push({ text: request.source.text.trim() });
      }
    } else if (track) {
      const cues = track.cues.filter((c) => {
        if (request.source.type === "subtitleTrack" && request.source.cueIds?.length) {
          return request.source.cueIds.includes(c.id);
        }
        return true;
      });
      for (const c of cues) {
        textSegments.push({
          text: c.text,
          cueId: c.id,
          startUs: c.startUs,
          endUs: c.endUs,
        });
      }
    }

    const total = textSegments.length || 1;

    const status: TtsStatus = {
      jobId,
      stage: "completed",
      completedSegments: total,
      totalSegments: total,
      progressPct: 100,
      currentMessage: "Mock synthesis completed",
    };

    const segments = textSegments.map((item, idx) => {
      const durUs = Math.max(500000, item.text.length * 60000);
      const cueDurUs = item.endUs && item.startUs ? item.endUs - item.startUs : undefined;
      const timingDiff = cueDurUs !== undefined ? durUs - cueDurUs : undefined;

      return {
        id: `seg_${idx}_${jobId}`,
        sourceCueId: item.cueId,
        originalText: item.text,
        voiceId: request.voiceId,
        sampleRate: 22050,
        channelCount: 1,
        sampleCount: Math.round((durUs / 1000000) * 22050),
        durationUs: durUs,
        cueStartUs: item.startUs,
        cueEndUs: item.endUs,
        timingDifferenceUs: timingDiff,
        stagedAudioPath: `/tmp/mock_${jobId}_seg_${idx}.wav`,
        warnings:
          timingDiff && Math.abs(timingDiff) > 500000
            ? [`Audio duration differs from cue by ${(timingDiff / 1000000).toFixed(2)}s`]
            : [],
      };
    });

    const result: TtsResult = {
      schemaVersion: 1,
      jobId,
      sourceType: request.source.type,
      voiceId: request.voiceId,
      runtime: "Mock Piper",
      segments,
      totalDurationUs: segments.reduce((acc, s) => acc + s.durationUs, 0),
      createdAtEpochMs: Date.now(),
      warnings: [],
    };

    mockJobs.set(jobId, { status, result });
    return jobId;
  }

  return invoke<string>("start_offline_tts", { request, track });
}

/**
 * Queries current job progress and stage.
 */
export async function getOfflineTtsStatus(jobId: string): Promise<TtsStatus> {
  if (!isTauriEnvironment()) {
    const job = mockJobs.get(jobId);
    if (!job) throw new Error(`Job '${jobId}' not found`);
    return job.status;
  }
  return invoke<TtsStatus>("get_offline_tts_status", { jobId });
}

/**
 * Cancels a running synthesis job.
 */
export async function cancelOfflineTts(jobId: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    const job = mockJobs.get(jobId);
    if (job) {
      job.status.stage = "cancelled";
      job.status.currentMessage = "Job cancelled by user";
      return true;
    }
    return false;
  }
  return invoke<boolean>("cancel_offline_tts", { jobId });
}

/**
 * Retrieves the finished result for a synthesis job.
 */
export async function getOfflineTtsResult(jobId: string): Promise<TtsResult> {
  if (!isTauriEnvironment()) {
    const job = mockJobs.get(jobId);
    if (!job || !job.result) throw new Error(`Result not ready for job '${jobId}'`);
    return job.result;
  }
  return invoke<TtsResult>("get_offline_tts_result", { jobId });
}

/**
 * Exports single segment or entire synthesized speech to a local WAV destination.
 */
export async function exportTtsAudio(
  jobId: string,
  destinationPath: string,
  segmentId?: string
): Promise<string> {
  if (!isTauriEnvironment()) {
    return destinationPath;
  }
  return invoke<string>("export_tts_audio", {
    jobId,
    segmentId,
    destinationPath,
  });
}
