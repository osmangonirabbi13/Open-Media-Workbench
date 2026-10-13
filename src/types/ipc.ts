/**
 * Strongly typed IPC contract matching the Rust `AppInfo` structure
 * returned by the `get_app_info` command.
 */
export interface AppInfo {
  appName: string;
  appVersion: string;
  coreVersion: string;
  osName: string;
  backendReady: boolean;
}

export type IpcState<T> =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "success"; data: T }
  | { status: "error"; error: string };
