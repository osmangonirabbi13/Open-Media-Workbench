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
