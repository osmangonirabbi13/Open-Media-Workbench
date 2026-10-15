/**
 * TypeScript Data Contracts for Offline Text-to-Speech (TTS).
 */

export interface TtsVoiceSettings {
  speakerId?: number;
  speedScale: number;
  noiseScale: number;
  noiseW: number;
}

export type TtsSource =
  | {
      type: "script";
      text: string;
    }
  | {
      type: "subtitleTrack";
      trackId: string;
      cueIds?: string[];
    };

export interface TtsRequest {
  source: TtsSource;
  voiceId: string;
  language: string;
  settings: TtsVoiceSettings;
}

export type TtsProgressStage =
  | "queued"
  | "checkingVoice"
  | "preparingText"
  | "loadingModel"
  | "synthesizing"
  | "validating"
  | "completed"
  | "failed"
  | "cancelled";

export interface TtsStatus {
  jobId: string;
  stage: TtsProgressStage;
  completedSegments: number;
  totalSegments: number;
  progressPct: number;
  currentMessage: string;
  error?: string;
}

export interface TtsAudioSegment {
  id: string;
  sourceCueId?: string;
  originalText: string;
  voiceId: string;
  sampleRate: number;
  channelCount: number;
  sampleCount: number;
  durationUs: number;
  cueStartUs?: number;
  cueEndUs?: number;
  timingDifferenceUs?: number;
  stagedAudioPath: string;
  warnings: string[];
}

export interface TtsResult {
  schemaVersion: number;
  jobId: string;
  sourceType: string;
  voiceId: string;
  runtime: string;
  segments: TtsAudioSegment[];
  totalDurationUs: number;
  createdAtEpochMs: number;
  warnings: string[];
}

export interface TtsRuntimeCapabilities {
  discovered: boolean;
  binaryPath?: string;
  version?: string;
  supportedFormats: string[];
  error?: string;
}

export interface AiVoiceManifest {
  id: string;
  displayName: string;
  language: string;
  locale: string;
  voiceFamily: string;
  modelId: string;
  sampleRate: number;
  speakerCount: number;
  isInstalled: boolean;
  runtimeReady: boolean;
  license: string;
  description: string;
}
