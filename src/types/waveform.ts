export type WaveformStatus =
  | { state: "none" }
  | { state: "extracting" }
  | { state: "ready"; durationMicros: number; totalSamples: number }
  | { state: "noAudio" }
  | { state: "failed"; error: string }
  | { state: "cancelled" };

export interface WaveformBin {
  min: number;
  max: number;
  rms: number;
  sampleCount: number;
}

export interface WaveformSegment {
  assetId: string;
  lodLevel: number;
  binsPerSecond: number;
  startMicros: number;
  endMicros: number;
  bins: WaveformBin[];
}

export interface WaveformStatusResponse {
  assetId: string;
  status: WaveformStatus;
}

export interface WaveformSegmentResponse {
  assetId: string;
  segment: WaveformSegment | null;
  status: WaveformStatus;
}
