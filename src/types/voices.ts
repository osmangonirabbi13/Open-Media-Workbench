//! TypeScript data contracts for the Offline AI Voice Library.

import type { TtsVoiceSettings } from './tts';

export type VoiceAvailabilityStatus =
  | 'availableForInstall'
  | 'installedRuntimeMissing'
  | 'installedVerificationRequired'
  | 'ready'
  | 'incompatible'
  | 'unsupportedLanguage'
  | 'unavailable';

export interface VoiceCatalogEntry {
  id: string;
  displayName: string;
  modelId: string;
  runtimeBackend: string;
  language: string;
  locale: string;
  version: string;
  qualityTier: string;
  artifactSizeBytes: number;
  sourceUrl: string;
  license: string;
  attribution: string;
  speakerCount: number;
  supportedSpeakerIds: number[];
  sampleRate: number;
  description: string;
  accent?: string;
}

export interface VoiceCompatibility {
  voiceId: string;
  status: VoiceAvailabilityStatus;
  isReady: boolean;
  isInstalled: boolean;
  runtimeReady: boolean;
  supportedLanguages: string[];
  speakerCount: number;
  warnings: string[];
}

export interface VoicePreferences {
  schemaVersion: number;
  favoriteVoiceIds: string[];
  preferredVoiceId?: string;
  recentVoiceIds: string[];
  lastSpeakerIds: Record<string, number>;
  updatedAtEpochMs: number;
}

export interface VoiceAssignment {
  trackId: string;
  voiceId: string;
  language: string;
  speakerId?: number;
  assignedAtEpochMs: number;
}

export interface ProjectVoiceAssignments {
  defaultVoiceId?: string;
  trackAssignments: VoiceAssignment[];
}

export interface VoicePreviewRequest {
  voiceId: string;
  customText?: string;
  speakerId?: number;
  settings?: TtsVoiceSettings;
}

export interface VoicePreviewResult {
  previewId: string;
  voiceId: string;
  text: string;
  stagedAudioPath: string;
  sampleRate: number;
  channelCount: number;
  durationUs: number;
  cached: boolean;
  createdAtEpochMs: number;
}

export interface VoiceComparisonRequest {
  voiceIdA: string;
  voiceIdB: string;
  text?: string;
}

export interface VoiceComparisonResult {
  voiceA: VoicePreviewResult;
  voiceB: VoicePreviewResult;
  durationDiffUs: number;
}
