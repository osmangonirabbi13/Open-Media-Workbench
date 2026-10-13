import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "../types/ipc";

/**
 * Checks if the current execution context is inside the Tauri native desktop window.
 */
export function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Invokes the Rust backend `get_app_info` command via typed Tauri IPC.
 *
 * Catches any IPC errors and formats them into an explicit error message.
 */
export async function getAppInfo(): Promise<AppInfo> {
  if (!isTauriEnvironment()) {
    // If opened in a regular web browser instead of the Tauri desktop shell:
    throw new Error(
      "Desktop IPC unavailable: OpenTake is running in a standalone browser window rather than the Tauri desktop container."
    );
  }

  try {
    return await invoke<AppInfo>("get_app_info");
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to communicate with OpenTake Rust backend: ${message}`);
  }
}
