import React, { useCallback, useEffect, useState } from "react";
import { AppHeader } from "../components/layout/AppHeader";
import { AppStatusBar } from "../components/layout/AppStatusBar";
import { RepairWorkspace } from "../workspaces/repair/RepairWorkspace";
import { LocalizationWorkspace } from "../workspaces/localization/LocalizationWorkspace";
import { SettingsDialog } from "../components/dialogs/SettingsDialog";
import { AboutDialog } from "../components/dialogs/AboutDialog";
import { useWorkspacePreferences } from "../hooks/useWorkspacePreferences";
import { getMediaEngineStatus } from "../lib/mediaEngine";
import type { AppInfo, IpcState } from "../types/ipc";
import type { MediaAsset } from "../types/media";
import type { MediaEngineStatus } from "../types/mediaEngine";
import type { WorkspaceId } from "../types/workspace";

export interface WorkbenchShellProps {
  ipcState: IpcState<AppInfo>;
  onRetryIpc: () => void;
}

export function WorkbenchShell({
  ipcState,
  onRetryIpc,
}: WorkbenchShellProps): React.ReactElement {
  const {
    preferences,
    setMediaPanelWidth,
    setTimelineDeckHeight,
    setTimecodeFormat,
    setSnapEnabled,
    setActiveWorkspace: setPersistedWorkspace,
    resetLayout,
  } = useWorkspacePreferences();

  // Active workspace state, initialized from stored preferences or defaulted to 'repair'
  const [activeWorkspace, setActiveWorkspace] = useState<WorkspaceId>(
    preferences.activeWorkspace || "repair"
  );

  // Shared active media asset across both workspaces
  const [selectedAsset, setSelectedAsset] = useState<MediaAsset | null>(null);

  // Dialog states
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isAboutOpen, setIsAboutOpen] = useState(false);

  // Media Engine Diagnostic State
  const [mediaStatus, setMediaStatus] = useState<MediaEngineStatus | null>(null);
  const [mediaLoading, setMediaLoading] = useState(false);
  const [mediaError, setMediaError] = useState<string | null>(null);

  const handleSelectWorkspace = useCallback(
    (ws: WorkspaceId) => {
      setActiveWorkspace(ws);
      setPersistedWorkspace(ws);
    },
    [setPersistedWorkspace]
  );

  const fetchMediaEngineDiagnostics = useCallback(
    async (forceRefresh: boolean = false): Promise<void> => {
      if (ipcState.status !== "success") return;
      setMediaLoading(true);
      setMediaError(null);
      try {
        const status = await getMediaEngineStatus(forceRefresh);
        setMediaStatus(status);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setMediaError(msg);
      } finally {
        setMediaLoading(false);
      }
    },
    [ipcState.status]
  );

  // Load diagnostics when backend comes online
  useEffect(() => {
    if (ipcState.status === "success") {
      void fetchMediaEngineDiagnostics(false);
    }
  }, [ipcState.status, fetchMediaEngineDiagnostics]);

  // Global keyboard shortcuts (including Alt+1 / Alt+2 for workspace switching)
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent): void => {
      const isInputFocused =
        e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;

      // Workspace switching: Alt+1 for Repair, Alt+2 for Localization
      if (e.altKey && !e.ctrlKey && !e.metaKey && !isInputFocused) {
        if (e.key === "1") {
          e.preventDefault();
          handleSelectWorkspace("repair");
          return;
        }
        if (e.key === "2") {
          e.preventDefault();
          handleSelectWorkspace("localization");
          return;
        }
      }

      // Ctrl+, or Cmd+, opens settings
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setIsSettingsOpen((prev) => !prev);
        return;
      }

      // Ctrl+R or Cmd+R with shift resets layout (without reloading page)
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "r") {
        e.preventDefault();
        resetLayout();
        return;
      }

      // 'S' toggles snapping when not typing inside an input/textarea
      if (
        e.key.toLowerCase() === "s" &&
        !e.ctrlKey &&
        !e.metaKey &&
        !e.altKey &&
        !isInputFocused
      ) {
        setSnapEnabled(!preferences.snapEnabled);
      }
    };

    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => window.removeEventListener("keydown", handleGlobalKeyDown);
  }, [preferences.snapEnabled, resetLayout, setSnapEnabled, handleSelectWorkspace]);

  const appInfoData = ipcState.status === "success" ? ipcState.data : undefined;

  return (
    <div className="flex flex-col h-screen w-screen bg-zinc-950 text-zinc-100 overflow-hidden select-none">
      {/* Global Application Header with Workspace Switcher */}
      <AppHeader
        ipcState={ipcState}
        activeWorkspace={activeWorkspace}
        onSelectWorkspace={handleSelectWorkspace}
        onRetryIpc={onRetryIpc}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onOpenAbout={() => setIsAboutOpen(true)}
        onResetLayout={resetLayout}
      />

      {/* IPC Warning Banner if connection failed */}
      {ipcState.status === "error" && (
        <div
          role="alert"
          className="bg-rose-950/80 border-b border-rose-800/80 px-4 py-1.5 text-xs text-rose-200 flex items-center justify-between shrink-0"
        >
          <span>
            <strong>Backend Offline:</strong> {ipcState.error}
          </span>
          <button
            type="button"
            onClick={onRetryIpc}
            className="px-2 py-0.5 rounded bg-rose-800 hover:bg-rose-700 text-white font-medium text-[11px] cursor-pointer"
          >
            Retry Connection
          </button>
        </div>
      )}

      {/* Active Studio Workspace */}
      <main className="flex-1 min-h-0 flex flex-col overflow-hidden">
        {activeWorkspace === "repair" ? (
          <RepairWorkspace
            mediaPanelWidth={preferences.mediaPanelWidth}
            timecodeFormat={preferences.timecodeFormat}
            selectedAsset={selectedAsset}
            onSelectAsset={setSelectedAsset}
            onMediaPanelWidthChange={setMediaPanelWidth}
          />
        ) : (
          <LocalizationWorkspace
            mediaPanelWidth={preferences.mediaPanelWidth}
            timelineDeckHeight={preferences.timelineDeckHeight}
            timecodeFormat={preferences.timecodeFormat}
            snapEnabled={preferences.snapEnabled}
            selectedAsset={selectedAsset}
            onSelectAsset={setSelectedAsset}
            onMediaPanelWidthChange={setMediaPanelWidth}
            onTimelineDeckHeightChange={setTimelineDeckHeight}
            onToggleSnap={() => setSnapEnabled(!preferences.snapEnabled)}
          />
        )}
      </main>

      {/* Shared Status Bar */}
      <AppStatusBar
        ipcState={ipcState}
        timecodeFormat={preferences.timecodeFormat}
      />

      {/* Settings Modal */}
      <SettingsDialog
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        appInfo={appInfoData}
        mediaStatus={mediaStatus}
        mediaLoading={mediaLoading}
        mediaError={mediaError}
        onRefreshMediaStatus={() => void fetchMediaEngineDiagnostics(true)}
        preferences={preferences}
        onResetLayout={resetLayout}
        onSetTimecodeFormat={setTimecodeFormat}
        onToggleSnap={() => setSnapEnabled(!preferences.snapEnabled)}
      />

      {/* About Modal */}
      <AboutDialog
        isOpen={isAboutOpen}
        onClose={() => setIsAboutOpen(false)}
        appInfo={appInfoData}
      />
    </div>
  );
}

// Backwards-compatible export alias for any legacy imports
export const EditorShell = WorkbenchShell;
export type EditorShellProps = WorkbenchShellProps;
