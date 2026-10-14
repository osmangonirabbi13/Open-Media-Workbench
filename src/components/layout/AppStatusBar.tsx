import React from "react";
import { ShieldCheckIcon } from "../ui/Icons";
import type { AppInfo, IpcState } from "../../types/ipc";

export interface AppStatusBarProps {
  ipcState: IpcState<AppInfo>;
  timecodeFormat: "timecode" | "frames";
}

export function AppStatusBar({
  ipcState,
  timecodeFormat,
}: AppStatusBarProps): React.ReactElement {
  return (
    <footer className="h-6 shrink-0 flex items-center justify-between px-3 border-t border-zinc-800/80 bg-zinc-950/95 text-[11px] text-zinc-400 select-none">
      {/* Left: Backend & System Runtime details */}
      <div className="flex items-center space-x-3">
        <div className="flex items-center space-x-1.5">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              ipcState.status === "success"
                ? "bg-emerald-400"
                : ipcState.status === "loading"
                  ? "bg-amber-400 animate-pulse"
                  : "bg-rose-400"
            }`}
            aria-hidden="true"
          />
          <span className="font-mono text-zinc-300">
            {ipcState.status === "success"
              ? `OpenTake v${ipcState.data.appVersion} (${ipcState.data.osName})`
              : "Backend Initializing"}
          </span>
        </div>

        <span className="text-zinc-600">|</span>

        <span>Timebase: 1 µs integer (CFR 30fps)</span>

        <span className="text-zinc-600">|</span>

        <span className="capitalize">Display: {timecodeFormat}</span>
      </div>

      {/* Center: Privacy & Offline-First identity */}
      <div className="hidden md:flex items-center space-x-1.5 text-zinc-400">
        <ShieldCheckIcon size={12} className="text-emerald-400" />
        <span>100% Offline-First • Zero Cloud Telemetry</span>
      </div>

      {/* Right: Keyboard shortcut hint */}
      <div className="flex items-center space-x-2 text-zinc-400">
        <span>Settings</span>
        <kbd className="px-1 py-0.5 rounded bg-zinc-800 border border-zinc-700/60 font-mono text-[10px] text-zinc-300">
          Ctrl+,
        </kbd>
      </div>
    </footer>
  );
}
