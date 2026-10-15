import type { AlignedWord } from "./alignment";

export type SubtitleFormat = "srt" | "webVtt" | "nativeProject";

export type FindingSeverity = "info" | "warning" | "error";

export interface WebVttCueSettings {
  vertical?: string;
  line?: string;
  position?: string;
  size?: string;
  align?: string;
  rawSettings?: string;
}

export interface SubtitleCue {
  id: string;
  startUs: number;
  endUs: number;
  text: string;
  speaker?: string;
  settings?: WebVttCueSettings;
  rawIdentifier?: string;
  customMetadata?: Record<string, string>;
  words?: AlignedWord[];
}

export interface SubtitleTrack {
  id: string;
  language: string;
  displayName: string;
  sourceFormat: SubtitleFormat;
  cues: SubtitleCue[];
  isEnabled: boolean;
  isVisible: boolean;
  schemaVersion: number;
}

export interface SubtitleDocument {
  schemaVersion: number;
  id: string;
  associatedMediaAssetId?: string;
  tracks: SubtitleTrack[];
  activeTrackId?: string;
  createdAtEpochMs: number;
  updatedAtEpochMs: number;
  metadata?: Record<string, string>;
}

export interface SubtitleFinding {
  code: string;
  severity: FindingSeverity;
  message: string;
  trackId?: string;
  cueId?: string;
  startUs?: number;
  endUs?: number;
}

export interface SubtitleImportResponse {
  document: SubtitleDocument;
  track: SubtitleTrack;
  findings: SubtitleFinding[];
  warnings: string[];
  fileName: string;
  fileSizeBytes: number;
  filePath?: string;
}

export interface SubtitleExportResponse {
  exportedPath: string;
  bytesWritten: number;
  format: SubtitleFormat;
  warnings: string[];
}

export interface SubtitleProjectSaveResponse {
  projectPath: string;
  bytesWritten: number;
}

export interface SubtitleProjectOpenResponse {
  document: SubtitleDocument;
  findings: SubtitleFinding[];
  fileName: string;
  filePath: string;
}

export type SubtitleOperation =
  | { type: "addCue"; payload: { trackId: string; cue: SubtitleCue } }
  | { type: "deleteCue"; payload: { trackId: string; cueId: string } }
  | { type: "deleteCues"; payload: { trackId: string; cueIds: string[] } }
  | { type: "updateCueText"; payload: { trackId: string; cueId: string; text: string } }
  | { type: "updateCueSpeaker"; payload: { trackId: string; cueId: string; speaker?: string } }
  | { type: "setCueTiming"; payload: { trackId: string; cueId: string; startUs: number; endUs: number } }
  | { type: "moveCue"; payload: { trackId: string; cueId: string; deltaUs: number } }
  | { type: "resizeCueStart"; payload: { trackId: string; cueId: string; newStartUs: number } }
  | { type: "resizeCueEnd"; payload: { trackId: string; cueId: string; newEndUs: number } }
  | { type: "splitCue"; payload: { trackId: string; cueId: string; splitTimeUs: number } }
  | { type: "mergeCues"; payload: { trackId: string; firstCueId: string; secondCueId: string } }
  | { type: "shiftSelectedCues"; payload: { trackId: string; cueIds: string[]; deltaUs: number } }
  | { type: "duplicateCue"; payload: { trackId: string; cueId: string } }
  | { type: "updateTrackMetadata"; payload: { trackId: string; language?: string; displayName?: string } };

export interface SubtitleOperationResponse {
  document: SubtitleDocument;
  findings: SubtitleFinding[];
}

export type SnapTargetType = "playhead" | "cueStart" | "cueEnd" | "grid";

export interface SnapResult {
  snappedTimeUs: number;
  snappedPixelX: number;
  targetType: SnapTargetType;
  targetTimeUs: number;
  label: string;
}

