export type AnalysisMode = "quick" | "deep";
export type AnalysisStatus = "completed" | "partial" | "cancelled" | "failed";
export type Severity = "info" | "warning" | "error" | "critical";
export type FindingCategory =
  | "containerSignature"
  | "boxStructure"
  | "streamIntegrity"
  | "timestampContinuity"
  | "decodeSample";

export interface ByteRange {
  start: number;
  length: number;
}

export interface DiagnosticFinding {
  code: string;
  severity: Severity;
  category: FindingCategory;
  summary: string;
  evidence: string;
  affectedStream?: string | null;
  mediaTimeRangeMs?: [number, number] | null;
  byteRange?: ByteRange | null;
  confidence: string;
  recommendedAction?: string | null;
}

export interface RepairRecommendation {
  strategyId: string;
  title: string;
  description: string;
  confidence: string;
  requiredEvidence: string[];
  targetStep: number;
  availableNow: boolean;
}

export interface InspectionCoverage {
  headerScannedBytes: number;
  totalFileBytes: number;
  boxCountFound: number;
  probeExecuted: boolean;
  decodeSampled: boolean;
  decodeSampleFrames?: number | null;
}

export interface DetectedContainerInfo {
  formatType: string;
  extension: string;
  signatureMatched: boolean;
  signatureDetails: string;
  mismatchWarning?: string | null;
}

export interface DiagnosticReport {
  schemaVersion: number;
  analysisId: string;
  assetId: string;
  fileName: string;
  detectedContainer: DetectedContainerInfo;
  analysisMode: AnalysisMode;
  startedAtEpochMs: number;
  completedAtEpochMs: number;
  analysisStatus: AnalysisStatus;
  findings: DiagnosticFinding[];
  recommendations: RepairRecommendation[];
  inspectionCoverage: InspectionCoverage;
  toolVersions?: string | null;
}

// ==========================================
// STEP 13 — Recovery Pipeline Types
// ==========================================

export type RecoveryMethod =
  | "losslessRemux"
  | "streamSalvage"
  | "timestampRebuild";

export type RecoveryStatus =
  | "queued"
  | "running"
  | "validating"
  | "completed"
  | "partial"
  | "failed"
  | "cancelled";

export type SalvageStreamType =
  | "videoOnly"
  | "audioOnly"
  | "allReadable";

export type ValidationLevel =
  | "metadataOnly"
  | "sampledDecode"
  | "fullDecode";

export interface StreamReportInfo {
  index: number;
  streamType: string;
  codecName: string;
  durationSeconds?: number | null;
  resolution?: string | null;
  sampleRate?: number | null;
  channels?: number | null;
}

export interface RecoveryValidationResult {
  passed: boolean;
  isPlayable: boolean;
  validationLevel: ValidationLevel;
  streamsPresent: string[];
  streamsExpected: string[];
  streamLossDetected: boolean;
  outputDurationSeconds?: number | null;
  details: string;
  warnings: string[];
}

export interface RecoveryRequest {
  assetId: string;
  method: RecoveryMethod;
  outputContainerPreference?: string | null;
  salvageStreamType?: SalvageStreamType | null;
  customDestination?: string | null;
  diagnosticReportId?: string | null;
}

export interface RecoveryReport {
  schemaVersion: number;
  jobId: string;
  sourceAssetId: string;
  sourceFileName: string;
  recoveryMethod: RecoveryMethod;
  originalContainer: string;
  intendedOutputContainer: string;
  actualOutputContainer: string;
  inputStreams: StreamReportInfo[];
  outputStreams: StreamReportInfo[];
  processingStatus: RecoveryStatus;
  validation: RecoveryValidationResult;
  warnings: string[];
  observedFfmpegErrors: string[];
  outputFileSizeBytes?: number | null;
  outputFilePath?: string | null;
  sourceDurationSeconds?: number | null;
  outputDurationSeconds?: number | null;
  durationDifferenceSeconds?: number | null;
  startedAtEpochMs: number;
  completedAtEpochMs: number;
  limitations: string[];
}

export interface RecoveryJobStatus {
  jobId: string;
  assetId: string;
  status: RecoveryStatus;
  stageMessage: string;
  report?: RecoveryReport | null;
  errorMessage?: string | null;
}
