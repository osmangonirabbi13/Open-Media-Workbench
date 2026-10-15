export type WordTimingProvenance =
  | "whisperNativeEstimate"
  | "forcedAlignment"
  | "userAdjusted"
  | "unaligned";

export type TimingQuality =
  | "aligned"
  | "estimated"
  | "partiallyAligned"
  | "unavailable";

export interface AlignedWord {
  id: string;
  text: string;
  startUs: number;
  endUs: number;
  sourceSegmentId?: string;
  sourceCueId?: string;
  confidence?: number;
  provenance: WordTimingProvenance;
  punctuationPrefix?: string;
  punctuationSuffix?: string;
}

export interface SegmentationPolicy {
  maxCharsPerLine: number;
  maxLinesPerCue: number;
  maxCueDurationUs: number;
  minCueDurationUs: number;
  maxCharsPerSecond: number;
  splitOnSilenceGapUs: number;
  splitOnPunctuation: boolean;
}

export const DEFAULT_SEGMENTATION_POLICY: SegmentationPolicy = {
  maxCharsPerLine: 42,
  maxLinesPerCue: 2,
  maxCueDurationUs: 7_000_000,
  minCueDurationUs: 1_000_000,
  maxCharsPerSecond: 21.0,
  splitOnSilenceGapUs: 400_000,
  splitOnPunctuation: true,
};

export interface AlignmentResult {
  schemaVersion: number;
  sourceAssetId: string;
  jobId: string;
  modelId: string;
  language: string;
  timingQuality: TimingQuality;
  words: AlignedWord[];
  warnings: string[];
}
