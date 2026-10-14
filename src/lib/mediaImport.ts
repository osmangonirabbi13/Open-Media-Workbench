import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type { ImportBatchResult, MediaAsset } from "../types/media";

/**
 * Invokes the native file picker in Rust and ingests chosen media files.
 */
export async function pickAndImportFiles(): Promise<ImportBatchResult> {
  if (!isTauriEnvironment()) {
    throw new Error(
      "Native file picker is unavailable outside the OpenTake Tauri desktop environment."
    );
  }

  try {
    return await invoke<ImportBatchResult>("pick_and_import_files");
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Media import failed: ${message}`);
  }
}

/**
 * Ingests a list of local file paths (e.g. from native window drag-and-drop).
 */
export async function importFilesByPath(paths: string[]): Promise<ImportBatchResult> {
  if (!isTauriEnvironment()) {
    throw new Error(
      "File drop import is unavailable outside the OpenTake Tauri desktop environment."
    );
  }

  if (paths.length === 0) {
    return { succeeded: [], duplicates: [], failed: [] };
  }

  try {
    return await invoke<ImportBatchResult>("import_files_by_path", { paths });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Media import failed: ${message}`);
  }
}

/**
 * Retrieves all imported assets in the active session.
 */
export async function getMediaLibraryAssets(): Promise<MediaAsset[]> {
  if (!isTauriEnvironment()) {
    return [];
  }

  try {
    return await invoke<MediaAsset[]>("get_media_library_assets");
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to load media library: ${message}`);
  }
}

/**
 * Removes an asset from the active session media library.
 * Guarantees original source file remains untouched on disk.
 */
export async function removeMediaLibraryAsset(id: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    return false;
  }

  try {
    return await invoke<boolean>("remove_media_library_asset", { id });
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to remove media asset: ${message}`);
  }
}

/**
 * Clears the active media library session.
 */
export async function clearMediaLibrary(): Promise<void> {
  if (!isTauriEnvironment()) {
    return;
  }

  try {
    await invoke<void>("clear_media_library");
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
          ? err
          : JSON.stringify(err);
    throw new Error(`Failed to clear media library: ${message}`);
  }
}
