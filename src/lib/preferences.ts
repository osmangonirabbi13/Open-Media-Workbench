import {
  DEFAULT_WORKSPACE_PREFERENCES,
  LAYOUT_BOUNDS,
  PREFERENCES_SCHEMA_VERSION,
  PREFERENCES_STORAGE_KEY,
  type WorkspacePreferences,
} from "../types/preferences";

/**
 * Clamps a numeric value within given min and max boundaries.
 */
export function clamp(value: number, min: number, max: number): number {
  if (Number.isNaN(value)) return min;
  return Math.min(Math.max(value, min), max);
}

/**
 * Validates and sanitizes a raw preference object.
 * Returns valid preferences or falls back safely to defaults.
 */
export function sanitizePreferences(raw: unknown): WorkspacePreferences {
  if (!raw || typeof raw !== "object") {
    return { ...DEFAULT_WORKSPACE_PREFERENCES };
  }

  const candidate = raw as Record<string, unknown>;

  // Check schema version - if version mismatch, fallback or migrate
  if (typeof candidate.schemaVersion !== "number" || candidate.schemaVersion !== PREFERENCES_SCHEMA_VERSION) {
    return { ...DEFAULT_WORKSPACE_PREFERENCES };
  }

  const mediaWidth =
    typeof candidate.mediaPanelWidth === "number"
      ? clamp(candidate.mediaPanelWidth, LAYOUT_BOUNDS.MEDIA_MIN_WIDTH, LAYOUT_BOUNDS.MEDIA_MAX_WIDTH)
      : LAYOUT_BOUNDS.MEDIA_DEFAULT_WIDTH;

  const timelineHeight =
    typeof candidate.timelineDeckHeight === "number"
      ? clamp(candidate.timelineDeckHeight, LAYOUT_BOUNDS.TIMELINE_MIN_HEIGHT, LAYOUT_BOUNDS.TIMELINE_MAX_HEIGHT)
      : LAYOUT_BOUNDS.TIMELINE_DEFAULT_HEIGHT;

  const timecodeFormat =
    candidate.timecodeFormat === "frames" ? "frames" : "timecode";

  const snapEnabled =
    typeof candidate.snapEnabled === "boolean" ? candidate.snapEnabled : true;

  const activeWorkspace =
    candidate.activeWorkspace === "localization"
      ? ("localization" as const)
      : candidate.activeWorkspace === "repair"
        ? ("repair" as const)
        : undefined;

  return {
    schemaVersion: PREFERENCES_SCHEMA_VERSION,
    mediaPanelWidth: mediaWidth,
    timelineDeckHeight: timelineHeight,
    theme: "dark",
    timecodeFormat,
    snapEnabled,
    ...(activeWorkspace ? { activeWorkspace } : {}),
  };
}

/**
 * Loads preferences from localStorage with fallback on error or corruption.
 */
export function loadWorkspacePreferences(): WorkspacePreferences {
  if (typeof window === "undefined" || !window.localStorage) {
    return { ...DEFAULT_WORKSPACE_PREFERENCES };
  }

  try {
    const serialized = window.localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (!serialized) {
      return { ...DEFAULT_WORKSPACE_PREFERENCES };
    }

    const parsed = JSON.parse(serialized);
    return sanitizePreferences(parsed);
  } catch (error) {
    console.warn("Failed to load workspace preferences; restoring defaults.", error);
    return { ...DEFAULT_WORKSPACE_PREFERENCES };
  }
}

/**
 * Saves preferences to localStorage safely.
 */
export function saveWorkspacePreferences(preferences: WorkspacePreferences): boolean {
  if (typeof window === "undefined" || !window.localStorage) {
    return false;
  }

  try {
    const sanitized = sanitizePreferences(preferences);
    window.localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(sanitized));
    return true;
  } catch (error) {
    console.error("Failed to persist workspace preferences to localStorage.", error);
    return false;
  }
}

/**
 * Clears preferences from localStorage and returns fresh defaults.
 */
export function resetWorkspacePreferences(): WorkspacePreferences {
  if (typeof window !== "undefined" && window.localStorage) {
    try {
      window.localStorage.removeItem(PREFERENCES_STORAGE_KEY);
    } catch {
      // Ignore removal errors
    }
  }
  return { ...DEFAULT_WORKSPACE_PREFERENCES };
}
