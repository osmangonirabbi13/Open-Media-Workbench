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
