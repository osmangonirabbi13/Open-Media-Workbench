import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type { PlaybackInfo, ProxyStatusResponse } from "../types/playback";

/**
 * Resolves an authorized media asset ID into an internal streaming URL and playback metadata.
 */
export async function getMediaPlaybackUrl(assetId: string): Promise<PlaybackInfo> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<PlaybackInfo>("get_media_playback_url", { assetId });
}

/**
 * Triggers background preview proxy generation for an authorized media asset.
 */
export async function requestPreviewProxy(assetId: string): Promise<ProxyStatusResponse> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<ProxyStatusResponse>("request_preview_proxy", { assetId });
}

/**
 * Cancels an in-flight preview proxy generation task for an asset.
 */
export async function cancelPreviewProxy(assetId: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<boolean>("cancel_preview_proxy", { assetId });
}

/**
 * Queries current proxy generation status for an asset.
 */
export async function getProxyStatus(assetId: string): Promise<ProxyStatusResponse> {
  if (!isTauriEnvironment()) {
    throw new Error("Tauri IPC unavailable: OpenTake running outside desktop shell.");
  }
  return await invoke<ProxyStatusResponse>("get_proxy_status", { assetId });
}

/**
 * Formats fractional seconds into standard SMPTE-style timecode (HH:MM:SS:FF).
 */
export function formatTimecode(seconds: number, fps: number = 30): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return "00:00:00:00";
  }

  const safeFps = fps > 0 ? fps : 30;
  const totalFrames = Math.floor(seconds * safeFps);
  const frames = Math.floor(totalFrames % safeFps);
  const totalSeconds = Math.floor(seconds);
  const secs = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const mins = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);

  const pad = (n: number, width: number = 2): string => String(n).padStart(width, "0");
  return `${pad(hours)}:${pad(mins)}:${pad(secs)}:${pad(frames)}`;
}

/**
 * Formats fractional seconds into a frame count representation ("00000 f").
 */
export function formatFrames(seconds: number, fps: number = 30): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return "00000 f";
  }
  const safeFps = fps > 0 ? fps : 30;
  const totalFrames = Math.floor(seconds * safeFps);
  return `${String(totalFrames).padStart(5, "0")} f`;
}
