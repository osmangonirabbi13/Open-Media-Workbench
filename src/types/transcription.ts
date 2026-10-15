export type TranscriptionProgressStage =
  | "queued"
  | "preparingAudio"
  | "loadingModel"
  | "transcribing"
  | "finalizing"
  | "completed"
  | "failed"
  | "cancelled";

export interface TranscriptionRequest {
  assetId: string;
  modelId: string;
  language: string;
  outputTrackName?: string;
  customRuntimePath?: string;
  threads?: number;
  startTimeUs?: number;
  endTimeUs?: number;
}

export interface TranscriptionStatus {
  jobId: string;
  stage: TranscriptionProgressStage;
  progressPercent?: number;
  message?: string;
  error?: string;
}

export interface TranscriptionSegment {
  id: string;
  startUs: number;
  endUs: number;
  text: string;
  detectedLanguage?: string;
  confidence?: number;
}

export interface TranscriptionResult {
  schemaVersion: number;
  jobId: string;
  assetId: string;
  modelId: string;
  runtimeVersion: string;
  requestedLanguage: string;
  detectedLanguage?: string;
  segments: TranscriptionSegment[];
  processingDurationMs: number;
  audioDurationUs?: number;
  warnings: string[];
}

export interface TranscriptionLanguage {
  code: string;
  label: string;
  isMultilingualOnly: boolean;
}

export interface WhisperRuntimeCapabilities {
  isAvailable: boolean;
  runtimePath?: string;
  runtimeVersion?: string;
  isCustom: boolean;
  supportedLanguages: TranscriptionLanguage[];
  errors: string[];
}
