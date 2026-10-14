import React, { useEffect, useState } from "react";
import { AppHeader } from "../components/layout/AppHeader";
import { AppStatusBar } from "../components/layout/AppStatusBar";
import { ResizableWorkspace } from "../components/layout/ResizableWorkspace";
import { SettingsDialog } from "../components/dialogs/SettingsDialog";
import { AboutDialog } from "../components/dialogs/AboutDialog";
import { useWorkspacePreferences } from "../hooks/useWorkspacePreferences";
import type { AppInfo, IpcState } from "../types/ipc";

export interface EditorShellProps {
  ipcState: IpcState<AppInfo>;
  onRetryIpc: () => void;
}

export function EditorShell({
  ipcState,
  onRetryIpc,
}: EditorShellProps): React.ReactElement {
  const {
    preferences,
    setMediaPanelWidth,
    setTimelineDeckHeight,
    setTimecodeFormat,
    setSnapEnabled,
    resetLayout,
  } = useWorkspacePreferences();

  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isAboutOpen, setIsAboutOpen] = useState(false);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent): void => {
      // Ctrl+, or Cmd+, opens settings
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setIsSettingsOpen((prev) => !prev);
      }
      // Ctrl+R or Cmd+R with shift resets layout (without reloading page)
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "r") {
        e.preventDefault();
        resetLayout();
      }
      // 'S' toggles snapping when not typing inside an input/textarea
      if (
        e.key.toLowerCase() === "s" &&
        !e.ctrlKey &&
        !e.metaKey &&
        !e.altKey &&
        !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)
      ) {
        // Toggle snapping
        setSnapEnabled(!preferences.snapEnabled);
      }
    };

    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => window.removeEventListener("keydown", handleGlobalKeyDown);
  }, [preferences.snapEnabled, resetLayout, setSnapEnabled]);

  const appInfoData = ipcState.status === "success" ? ipcState.data : undefined;

  return (
    <div className="flex flex-col h-screen w-screen bg-zinc-950 text-zinc-100 overflow-hidden select-none">
      {/* Global Application Header */}
      <AppHeader
        ipcState={ipcState}
        onRetryIpc={onRetryIpc}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onOpenAbout={() => setIsAboutOpen(true)}
        onResetLayout={resetLayout}
      />

      {/* IPC Warning Banner if connection failed */}
      {ipcState.status === "error" && (
        <div
          role="alert"
          className="bg-rose-950/80 border-b border-rose-800/80 px-4 py-1.5 text-xs text-rose-200 flex items-center justify-between"
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

      {/* Main Resizable Workspace (Media Library, Preview Monitor, Timeline Deck) */}
      <main className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <ResizableWorkspace
          mediaPanelWidth={preferences.mediaPanelWidth}
          timelineDeckHeight={preferences.timelineDeckHeight}
          timecodeFormat={preferences.timecodeFormat}
          snapEnabled={preferences.snapEnabled}
          onMediaPanelWidthChange={setMediaPanelWidth}
          onTimelineDeckHeightChange={setTimelineDeckHeight}
          onToggleSnap={() => setSnapEnabled(!preferences.snapEnabled)}
        />
      </main>

      {/* Status Bar */}
      <AppStatusBar
        ipcState={ipcState}
        timecodeFormat={preferences.timecodeFormat}
      />

      {/* Settings Modal */}
      <SettingsDialog
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        appInfo={appInfoData}
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
