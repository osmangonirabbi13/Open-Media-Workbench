import { useCallback, useEffect, useState } from "react";
import {
  clamp,
  loadWorkspacePreferences,
  resetWorkspacePreferences,
  saveWorkspacePreferences,
} from "../lib/preferences";
import {
  LAYOUT_BOUNDS,
  type WorkspacePreferences,
} from "../types/preferences";

export interface UseWorkspacePreferencesReturn {
  preferences: WorkspacePreferences;
  setMediaPanelWidth: (width: number) => void;
  setTimelineDeckHeight: (height: number) => void;
  setTimecodeFormat: (format: "timecode" | "frames") => void;
  setSnapEnabled: (enabled: boolean) => void;
  setActiveWorkspace: (workspace: "repair" | "localization") => void;
  resetLayout: () => void;
}

export function useWorkspacePreferences(): UseWorkspacePreferencesReturn {
  const [preferences, setPreferences] = useState<WorkspacePreferences>(() =>
    loadWorkspacePreferences()
  );

  // Sync to localStorage on update
  useEffect(() => {
    saveWorkspacePreferences(preferences);
  }, [preferences]);

  const setMediaPanelWidth = useCallback((width: number) => {
    setPreferences((prev) => ({
      ...prev,
      mediaPanelWidth: clamp(width, LAYOUT_BOUNDS.MEDIA_MIN_WIDTH, LAYOUT_BOUNDS.MEDIA_MAX_WIDTH),
    }));
  }, []);

  const setTimelineDeckHeight = useCallback((height: number) => {
    setPreferences((prev) => ({
      ...prev,
      timelineDeckHeight: clamp(
        height,
        LAYOUT_BOUNDS.TIMELINE_MIN_HEIGHT,
        LAYOUT_BOUNDS.TIMELINE_MAX_HEIGHT
      ),
    }));
  }, []);

  const setTimecodeFormat = useCallback((timecodeFormat: "timecode" | "frames") => {
    setPreferences((prev) => ({
      ...prev,
      timecodeFormat,
    }));
  }, []);

  const setSnapEnabled = useCallback((snapEnabled: boolean) => {
    setPreferences((prev) => ({
      ...prev,
      snapEnabled,
    }));
  }, []);

  const setActiveWorkspace = useCallback((activeWorkspace: "repair" | "localization") => {
    setPreferences((prev) => ({
      ...prev,
      activeWorkspace,
    }));
  }, []);

  const resetLayout = useCallback(() => {
    const defaults = resetWorkspacePreferences();
    setPreferences(defaults);
  }, []);

  return {
    preferences,
    setMediaPanelWidth,
    setTimelineDeckHeight,
    setTimecodeFormat,
    setSnapEnabled,
    setActiveWorkspace,
    resetLayout,
  };
}
