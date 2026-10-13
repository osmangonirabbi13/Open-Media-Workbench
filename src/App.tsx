import React, { useEffect, useState } from "react";
import { getAppInfo, isTauriEnvironment } from "./lib/tauri";
import type { AppInfo, IpcState } from "./types/ipc";

export function App(): React.ReactElement {
  const [ipcState, setIpcState] = useState<IpcState<AppInfo>>({ status: "idle" });

  const fetchBackendInfo = async (): Promise<void> => {
    setIpcState({ status: "loading" });
    try {
      const info = await getAppInfo();
      setIpcState({ status: "success", data: info });
    } catch (err: unknown) {
      const errorMessage =
        err instanceof Error ? err.message : "Unknown backend communication failure.";
      setIpcState({ status: "error", error: errorMessage });
    }
  };

  useEffect(() => {
    void fetchBackendInfo();
  }, []);

  return (
    <div className="flex flex-col h-screen w-screen bg-zinc-950 text-zinc-100 select-none">
      {/* Titlebar / Header */}
      <header className="flex items-center justify-between px-6 py-3.5 border-b border-zinc-800/80 bg-zinc-900/60 backdrop-blur-md">
        <div className="flex items-center space-x-3">
          <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-indigo-600 text-white font-bold text-sm shadow-md shadow-indigo-600/30">
            OT
          </div>
          <div>
            <h1 className="text-base font-semibold tracking-tight text-zinc-100">OpenTake</h1>
            <p className="text-xs text-zinc-400">Free AI Video Editing Studio</p>
          </div>
        </div>

        {/* Backend Connection Indicator */}
        <div className="flex items-center space-x-2 px-3 py-1.5 rounded-full text-xs font-medium border border-zinc-800 bg-zinc-900/80">
          {ipcState.status === "loading" && (
            <>
              <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
              <span className="text-zinc-300">Connecting to Core Engine...</span>
            </>
          )}
          {ipcState.status === "success" && (
            <>
              <span className="w-2 h-2 rounded-full bg-emerald-400 shadow-sm shadow-emerald-400/50" />
              <span className="text-emerald-300">Rust Core Online</span>
            </>
          )}
          {ipcState.status === "error" && (
            <>
              <span className="w-2 h-2 rounded-full bg-rose-400 shadow-sm shadow-rose-400/50" />
              <span className="text-rose-300">Core Disconnected</span>
            </>
          )}
          {ipcState.status === "idle" && (
            <>
              <span className="w-2 h-2 rounded-full bg-zinc-500" />
              <span className="text-zinc-400">Initializing</span>
            </>
          )}
        </div>
      </header>

      {/* Main Workspace Body */}
      <main className="flex-1 flex flex-col items-center justify-center p-8 text-center max-w-3xl mx-auto w-full">
        <div className="w-full rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-8 shadow-2xl backdrop-blur-sm space-y-6">
          <div className="space-y-2">
            <div className="inline-flex items-center px-3 py-1 rounded-full text-xs font-medium bg-indigo-950/80 text-indigo-300 border border-indigo-800/60 mb-2">
              Milestone 1 — Foundation | STEP 3
            </div>
            <h2 className="text-2xl font-bold tracking-tight text-zinc-100">
              Desktop Application Bootstrap
            </h2>
            <p className="text-sm text-zinc-400 max-w-lg mx-auto">
              OpenTake Tauri 2 desktop shell is running natively on your workstation with secure,
              typed IPC communication to the headless Rust core.
            </p>
          </div>

          {/* System & Runtime Diagnostic Card */}
          <div className="rounded-xl border border-zinc-800 bg-zinc-950/70 p-5 text-left space-y-4">
            <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
              Runtime Diagnostic State
            </h3>

            {ipcState.status === "success" && (
              <div className="grid grid-cols-2 gap-3 text-sm">
                <div className="p-3 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
                  <span className="text-xs text-zinc-400 block">Desktop App Version</span>
                  <span className="font-mono text-zinc-200 font-medium">
                    v{ipcState.data.appVersion}
                  </span>
                </div>
                <div className="p-3 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
                  <span className="text-xs text-zinc-400 block">Rust Core Engine</span>
                  <span className="font-mono text-zinc-200 font-medium">
                    v{ipcState.data.coreVersion}
                  </span>
                </div>
                <div className="p-3 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
                  <span className="text-xs text-zinc-400 block">Host Operating System</span>
                  <span className="font-mono text-zinc-200 font-medium capitalize">
                    {ipcState.data.osName}
                  </span>
                </div>
                <div className="p-3 rounded-lg bg-zinc-900/60 border border-zinc-800/60">
                  <span className="text-xs text-zinc-400 block">IPC Bridge Status</span>
                  <span className="font-mono text-emerald-400 font-medium">Verified Active</span>
                </div>
              </div>
            )}

            {ipcState.status === "loading" && (
              <div className="flex items-center justify-center p-6 text-sm text-zinc-400 space-x-2">
                <svg className="animate-spin h-5 w-5 text-indigo-400" viewBox="0 0 24 24" fill="none">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8H4z" />
                </svg>
                <span>Querying Rust backend over Tauri IPC bridge...</span>
              </div>
            )}

            {ipcState.status === "error" && (
              <div className="p-4 rounded-lg bg-rose-950/40 border border-rose-800/60 text-sm text-rose-300 space-y-3">
                <p className="font-medium">IPC Error: {ipcState.error}</p>
                {!isTauriEnvironment() && (
                  <p className="text-xs text-zinc-400">
                    To run the desktop application, execute{" "}
                    <code className="text-indigo-300 font-mono">npm run tauri dev</code> in your terminal.
                  </p>
                )}
                <button
                  type="button"
                  onClick={() => void fetchBackendInfo()}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white transition-colors cursor-pointer"
                >
                  Retry Connection
                </button>
              </div>
            )}
          </div>

          {/* Roadmap Handoff Notice */}
          <div className="rounded-xl border border-dashed border-zinc-800 p-4 text-xs text-zinc-400 text-left space-y-1">
            <span className="font-semibold text-zinc-300 block">
              Planned Capabilities (Steps 4–21):
            </span>
            <p>
              Media import, FFmpeg inspection, video preview, audio waveform rendering, silence DSP
              detection, timeline editing, and MP4 rendering will be built in subsequent roadmap steps.
            </p>
          </div>
        </div>
      </main>

      {/* Footer / Status bar */}
      <footer className="flex items-center justify-between px-6 py-2 border-t border-zinc-800/60 bg-zinc-900/40 text-[11px] text-zinc-400">
        <div>OpenTake Desktop Beta Foundation</div>
        <div>Offline-First &bull; Privacy-First &bull; Free & Open Source</div>
      </footer>
    </div>
  );
}

export default App;
