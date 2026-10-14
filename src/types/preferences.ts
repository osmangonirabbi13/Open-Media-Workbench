/**
 * Workspace Preferences Schema Definition
 * Versioned schema for UI layout and display preferences.
 * Stored safely in localStorage with validation and recovery.
 */

export interface WorkspacePreferences {
  schemaVersion: number;
  /** Width of the media library panel in pixels */
  mediaPanelWidth: number;
  /** Height of the bottom timeline deck in pixels */
  timelineDeckHeight: number;
  /** UI theme - currently locked to 'dark' for beta */
  theme: "dark";
  /** Timecode display format */
  timecodeFormat: "timecode" | "frames";
  /** Whether snap-to-grid is visually enabled in the timeline */
  snapEnabled: boolean;
  /** Active workspace tab (repair or localization) */
  activeWorkspace?: "repair" | "localization";
}

export const PREFERENCES_SCHEMA_VERSION = 1;
export const PREFERENCES_STORAGE_KEY = "opentake_workspace_preferences_v1";

/**
 * Hard constraints for workspace layout dimensions (in pixels)
 */
export const LAYOUT_BOUNDS = {
  MEDIA_MIN_WIDTH: 220,
  MEDIA_MAX_WIDTH: 520,
  MEDIA_DEFAULT_WIDTH: 280,

  TIMELINE_MIN_HEIGHT: 180,
  TIMELINE_MAX_HEIGHT: 550,
  TIMELINE_DEFAULT_HEIGHT: 260,
} as const;

export const DEFAULT_WORKSPACE_PREFERENCES: WorkspacePreferences = {
  schemaVersion: PREFERENCES_SCHEMA_VERSION,
  mediaPanelWidth: LAYOUT_BOUNDS.MEDIA_DEFAULT_WIDTH,
  timelineDeckHeight: LAYOUT_BOUNDS.TIMELINE_DEFAULT_HEIGHT,
  theme: "dark",
  timecodeFormat: "timecode",
  snapEnabled: true,
};
