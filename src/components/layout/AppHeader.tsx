import React from "react";
import { InfoIcon, OpenMediaWorkbenchLogo, ResetIcon, SettingsIcon } from "../ui/Icons";
import { IconButton } from "../ui/IconButton";
import { StatusBadge } from "../ui/StatusBadge";
import { WorkspaceSwitcher } from "./WorkspaceSwitcher";
import type { AppInfo, IpcState } from "../../types/ipc";
import type { WorkspaceId } from "../../types/workspace";

export interface AppHeaderProps {
  ipcState: IpcState<AppInfo>;
  activeWorkspace?: WorkspaceId;
  onSelectWorkspace?: (workspace: WorkspaceId) => void;
  onRetryIpc: () => void;
  onOpenSettings: () => void;
  onOpenAbout: () => void;
  onResetLayout: () => void;
}

export function AppHeader({
  ipcState,
  activeWorkspace,
  onSelectWorkspace,
  onRetryIpc,
  onOpenSettings,
  onOpenAbout,
  onResetLayout,
}: AppHeaderProps): React.ReactElement {
  const connectionStatus =
    ipcState.status === "success"
      ? "online"
      : ipcState.status === "loading"
        ? "connecting"
        : ipcState.status === "error"
          ? "offline"
          : "idle";

  return (
    <header className="h-11 shrink-0 flex items-center justify-between px-3.5 border-b border-zinc-800/80 bg-zinc-950/80 backdrop-blur-sm select-none">
      {/* Left: Branding & Project Workspace State */}
      <div className="flex items-center space-x-3">
        <div className="flex items-center space-x-2">
          <OpenMediaWorkbenchLogo size={22} />
          <span className="font-semibold text-sm tracking-tight text-zinc-100 hidden sm:inline">
            Open Media Workbench
          </span>
        </div>

        <div className="h-4 w-[1px] bg-zinc-800" aria-hidden="true" />

        <div className="flex items-center space-x-2">
          <span className="text-xs text-zinc-300 font-medium">Untitled Project</span>
          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-normal bg-zinc-800/60 text-zinc-400 border border-zinc-700/40">
            Unsaved
          </span>
        </div>
      </div>

      {/* Center: Global Workspace Switcher */}
      {activeWorkspace && onSelectWorkspace && (
        <div className="flex items-center justify-center">
          <WorkspaceSwitcher
            activeWorkspace={activeWorkspace}
            onSelectWorkspace={onSelectWorkspace}
          />
        </div>
      )}

      {/* Right: Actions, Layout Controls, Backend Indicator & Settings */}
      <div className="flex items-center space-x-2">
        {/* Backend status badge */}
        <div className="cursor-pointer" onClick={ipcState.status === "error" ? onRetryIpc : undefined}>
          <StatusBadge
            status={connectionStatus}
            label={
              ipcState.status === "success"
                ? `Core v${ipcState.data.coreVersion}`
                : ipcState.status === "error"
                  ? "Core Offline (Click to Retry)"
                  : undefined
            }
          />
        </div>

        <div className="h-4 w-[1px] bg-zinc-800" aria-hidden="true" />

        {/* Reset Layout */}
        <IconButton
          icon={<ResetIcon size={14} />}
          label="Reset Workspace Layout"
          shortcut="Ctrl+R"
          size="sm"
          variant="subtle"
          onClick={onResetLayout}
        />

        {/* About App */}
        <IconButton
          icon={<InfoIcon size={14} />}
          label="About Open Media Workbench"
          size="sm"
          variant="subtle"
          onClick={onOpenAbout}
        />

        {/* Settings Dialog Trigger */}
        <IconButton
          icon={<SettingsIcon size={14} />}
          label="Application Settings"
          shortcut="Ctrl+,"
          size="sm"
          variant="subtle"
          onClick={onOpenSettings}
        />
      </div>
    </header>
  );
}
