/**
 * Domain media types synchronized with opentake-core Rust models.
 */

export interface Rational {
  num: number;
  den: number;
}

export interface ContainerMetadata {
  formatName: string;
  formatLongName?: string;
  durationMicros?: number;
  startTimeMicros?: number;
  bitrateBps?: number;
  sizeBytes: number;
  streamCount: number;
}

export interface VideoStreamMetadata {
  streamIndex: number;
  codecName: string;
  codecLongName?: string;
  width: number;
  height: number;
  pixelFormat?: string;
  sampleAspectRatio?: string;
  displayAspectRatio?: string;
  rotationDegrees?: number;
  averageFrameRate?: Rational;
  nominalFrameRate?: Rational;
  timebase?: Rational;
  durationMicros?: number;
  profile?: string;
  isVfrSuspect: boolean;
}

export interface AudioStreamMetadata {
  streamIndex: number;
  codecName: string;
  codecLongName?: string;
  sampleRate: number;
  channels: number;
  channelLayout?: string;
  bitrateBps?: number;
  durationMicros?: number;
  timebase?: Rational;
}

export interface OtherStreamMetadata {
  streamIndex: number;
  codecType: string;
  codecName?: string;
}

export interface MediaMetadata {
  container: ContainerMetadata;
  videoStreams: VideoStreamMetadata[];
  audioStreams: AudioStreamMetadata[];
  otherStreams: OtherStreamMetadata[];
}

export type CompatibilityStatus =
  | 'supported'
  | 'supportedWithLimitations'
  | 'unsupported'
  | 'invalid';

export interface MediaCompatibility {
  overall: CompatibilityStatus;
  containerStatus: CompatibilityStatus;
  videoStatus: CompatibilityStatus;
  audioStatus: CompatibilityStatus;
  canInspect: boolean;
  canEditFuture: boolean;
  warnings: string[];
}

export interface SourceIdentity {
  canonicalPath: string;
  fileSize: number;
  modifiedEpochMs?: number;
  identityHint: string;
}

export interface MediaAsset {
  id: string;
  fileName: string;
  source: SourceIdentity;
  metadata: MediaMetadata;
  compatibility: MediaCompatibility;
  importedAtEpochMs: number;
}

export interface ImportFailure {
  fileName: string;
  errorCode: string;
  errorMessage: string;
}

export interface ImportBatchResult {
  succeeded: MediaAsset[];
  duplicates: string[];
  failed: ImportFailure[];
}

/** Formats integer microseconds into MM:SS or HH:MM:SS format */
export function formatDurationMicros(micros?: number): string {
  if (micros === undefined || micros === null) return '--:--';
  const totalSeconds = Math.floor(micros / 1_000_000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  const mm = String(minutes).padStart(2, '0');
  const ss = String(seconds).padStart(2, '0');

  if (hours > 0) {
    const hh = String(hours).padStart(2, '0');
    return `${hh}:${mm}:${ss}`;
  }
  return `${mm}:${ss}`;
}

/** Formats byte size into human readable string */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

/** Computes display fps string from Rational frame rate */
export function formatFps(rate?: Rational): string {
  if (!rate || rate.den === 0) return 'Unknown fps';
  const fps = rate.num / rate.den;
  if (Number.isInteger(fps)) {
    return `${fps} fps`;
  }
  return `${fps.toFixed(2)} fps`;
}
