
export interface TranslationLanguagePair {
  sourceCode: string;
  sourceName: string;
  targetCode: string;
  targetName: string;
  compatibleModelIds: string[];
  isVerified: boolean;
}

export interface TranslationRequest {
  documentId?: string;
  trackId: string;
  sourceLanguage: string;
  targetLanguage: string;
  modelId: string;
  customRuntimePath?: string;
  batchSize?: number;
}

export type TranslationProgressStage =
  | "queued"
  | "checkingModel"
  | "preparing"
  | "translating"
  | "validating"
  | "completed"
  | "failed"
  | "cancelled";

export interface TranslationStatus {
  jobId: string;
  stage: TranslationProgressStage;
  progressPercent?: number;
  message?: string;
  error?: string;
  translatedCues: number;
  totalCues: number;
}

export interface TranslatedCue {
  sourceCueId: string;
  translatedCueId: string;
  originalText: string;
  translatedText: string;
  sourceStartUs: number;
  sourceEndUs: number;
  targetStartUs: number;
  targetEndUs: number;
  warnings: string[];
  modelProvenance: string;
}

export interface TranslationResult {
  schemaVersion: number;
  jobId: string;
  originalTrackId: string;
  modelId: string;
  runtimeVersion: string;
  sourceLanguage: string;
  targetLanguage: string;
  translatedCues: TranslatedCue[];
  untranslatedCueIds: string[];
  warnings: string[];
  elapsedMs: number;
}

export interface TranslationRuntimeCapabilities {
  isAvailable: boolean;
  runtimePath?: string;
  runtimeVersion?: string;
  supportedLanguagePairs: TranslationLanguagePair[];
}
