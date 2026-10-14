import type { SubtitleDocument } from "../types/subtitles";

const AUTOSAVE_KEY_PREFIX = "omw_subtitle_autosave_";

export interface AutosaveSnapshot {
  document: SubtitleDocument;
  savedAtEpochMs: number;
  assetId?: string;
  isDirty: boolean;
}

function getStorage(): Storage | null {
  if (typeof window !== "undefined" && window.localStorage) {
    return window.localStorage;
  }
  if (typeof globalThis !== "undefined" && (globalThis as unknown as { localStorage?: Storage }).localStorage) {
    return (globalThis as unknown as { localStorage: Storage }).localStorage;
  }
  return null;
}

/**
 * Saves a recovery snapshot for a subtitle document in local storage.
 */
export function saveAutosaveSnapshot(
  document: SubtitleDocument,
  assetId?: string
): void {
  const storage = getStorage();
  if (!storage) return;

  const key = `${AUTOSAVE_KEY_PREFIX}${assetId || "default"}`;
  const snapshot: AutosaveSnapshot = {
    document,
    savedAtEpochMs: Date.now(),
    assetId,
    isDirty: true,
  };

  try {
    storage.setItem(key, JSON.stringify(snapshot));
  } catch (err) {
    console.warn("Failed to write subtitle autosave snapshot:", err);
  }
}

/**
 * Checks if a valid autosave recovery snapshot exists for the given asset.
 */
export function loadAutosaveSnapshot(
  assetId?: string
): AutosaveSnapshot | null {
  const storage = getStorage();
  if (!storage) return null;

  const key = `${AUTOSAVE_KEY_PREFIX}${assetId || "default"}`;
  try {
    const raw = storage.getItem(key);
    if (!raw) return null;

    const parsed = JSON.parse(raw) as AutosaveSnapshot;
    if (
      parsed &&
      parsed.document &&
      Array.isArray(parsed.document.tracks) &&
      typeof parsed.savedAtEpochMs === "number"
    ) {
      return parsed;
    }
  } catch (err) {
    console.warn("Corrupted autosave snapshot discarded:", err);
    clearAutosaveSnapshot(assetId);
  }

  return null;
}

/**
 * Clears an autosave snapshot after explicit user save or rejection.
 */
export function clearAutosaveSnapshot(assetId?: string): void {
  const storage = getStorage();
  if (!storage) return;
  const key = `${AUTOSAVE_KEY_PREFIX}${assetId || "default"}`;
  try {
    storage.removeItem(key);
  } catch {
    // Ignored
  }
}

