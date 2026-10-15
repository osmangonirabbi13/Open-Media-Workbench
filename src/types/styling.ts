/**
 * Subtitle Styling and Video Burn-in Export Types.
 */

export interface ColorRgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

export type TextAlignment = "left" | "center" | "right";

export type SubtitlePosition =
  | { type: "bottomCenter" }
  | { type: "bottomLeft" }
  | { type: "bottomRight" }
  | { type: "center" }
  | { type: "topCenter" }
  | { type: "custom"; xPct: number; yPct: number };

export type SubtitleAnimation = "none" | "fade" | "pop" | "karaokeHighlight";

export interface SubtitleStyle {
  id: string;
  displayName: string;
  fontFamily: string;
  fontSize: number;
  fontWeight: number;
  italic: boolean;
  textColor: ColorRgba;
  outlineColor: ColorRgba;
  outlineWidth: number;
  shadowColor: ColorRgba;
  shadowOffset: number;
  backgroundColor: ColorRgba;
  backgroundOpacity: number;
  backgroundPadding: number;
  textAlignment: TextAlignment;
  position: SubtitlePosition;
  marginVertical: number;
  marginHorizontal: number;
  lineSpacing: number;
  wordHighlightColor: ColorRgba;
  animation: SubtitleAnimation;
  safeAreaEnabled: boolean;
  schemaVersion: number;
}

export interface SubtitleStylePreset {
  id: string;
  displayName: string;
  description: string;
  isBuiltin: boolean;
  style: SubtitleStyle;
}

export interface FontFamilyInfo {
  family: string;
  isInstalled: boolean;
  supportsBengali: boolean;
  isGenericFallback: boolean;
}

export type ExportResolution =
  | { type: "original" }
  | { type: "res1080p" }
  | { type: "res720p" }
  | { type: "custom"; width: number; height: number };

export type AudioExportPolicy = "copy" | "aac";

export interface SubtitleExportProfile {
  container: string;
  resolution: ExportResolution;
  videoCodec: string;
  crf: number;
  preset: string;
  audioPolicy: AudioExportPolicy;
  selectedTrackId: string;
  style: SubtitleStyle;
}

export interface SubtitleBurnInRequest {
  jobId: string;
  sourceMediaPath: string;
  track: import("./subtitles").SubtitleTrack;
  style: SubtitleStyle;
  profile: SubtitleExportProfile;
  stagingDir: string;
}

export type SubtitleBurnInStatus =
  | { stage: "queued" }
  | { stage: "preparing" }
  | { stage: "generatingAss" }
  | { stage: "encoding"; progressPct: number; message: string }
  | { stage: "validating" }
  | {
      stage: "readyToSave";
      stagedOutputPath: string;
      sizeBytes: number;
      durationSecs: number;
      width: number;
      height: number;
    }
  | { stage: "completed"; finalDestinationPath: string }
  | { stage: "failed"; error: string }
  | { stage: "cancelled" };

export interface SubtitleBurnInResult {
  jobId: string;
  stagedOutputPath: string;
  fileSizeBytes: number;
  durationSecs: number;
  width: number;
  height: number;
  videoCodec: string;
  audioCodec?: string;
  validationWarnings: string[];
}
