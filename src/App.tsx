import React, { useEffect, useState } from "react";
import { EditorShell } from "./app/EditorShell";
import { getAppInfo } from "./lib/tauri";
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

  return <EditorShell ipcState={ipcState} onRetryIpc={() => void fetchBackendInfo()} />;
}

export default App;
