import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type {
  WaveformSegmentResponse,
  WaveformStatusResponse,
} from "../types/waveform";

/**
 * Queries the current audio waveform analysis status for an authorized asset.
 */
export async function getWaveformStatus(assetId: string): Promise<WaveformStatusResponse> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<WaveformStatusResponse>("get_waveform_status", { assetId });
}

/**
 * Requests background streaming audio waveform extraction and multi-LOD pyramid generation.
 */
export async function requestWaveformAnalysis(assetId: string): Promise<WaveformStatusResponse> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<WaveformStatusResponse>("request_waveform_analysis", { assetId });
}

/**
 * Cancels an in-flight waveform extraction task for an asset.
 */
export async function cancelWaveformAnalysis(assetId: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<boolean>("cancel_waveform_analysis", { assetId });
}

/**
 * Queries a bounded slice of waveform amplitude bins covering a visible source interval.
 */
export async function getWaveformSegment(
  assetId: string,
  startMicros: number,
  endMicros: number,
  pixelWidth: number
): Promise<WaveformSegmentResponse> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<WaveformSegmentResponse>("get_waveform_segment", {
    assetId,
    startMicros: Math.round(startMicros),
    endMicros: Math.round(endMicros),
    pixelWidth: Math.round(pixelWidth),
  });
}
