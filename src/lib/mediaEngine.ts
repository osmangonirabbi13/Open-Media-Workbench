import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type { MediaEngineStatus } from "../types/mediaEngine";

/**
 * Invokes the Rust backend `get_media_engine_status` command.
 *
 * @param forceRefresh When true, bypasses the in-memory cache and re-probes binaries.
 */
export async function getMediaEngineStatus(
  forceRefresh: boolean = false
): Promise<MediaEngineStatus> {
  if (!isTauriEnvironment()) {
    throw new Error(
      "Media Engine IPC unavailable: OpenTake is running in a browser window rather than the Tauri desktop container."
    );
  }

  try {
    return await invoke<MediaEngineStatus>("get_media_engine_status", {
      forceRefresh,
    });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to query media engine status: ${message}`);
  }
}
