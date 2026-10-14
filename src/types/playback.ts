export type ProxyState =
  | { state: "none" }
  | { state: "generating" }
  | { state: "ready"; proxyPath: string }
  | { state: "failed"; error: string }
  | { state: "cancelled" };

export interface PlaybackInfo {
  assetId: string;
  playbackUrl: string;
  isProxy: boolean;
  mimeType: string;
  width?: number;
  height?: number;
  durationSeconds?: number;
  fps?: number;
  hasAudio: boolean;
  proxyState: ProxyState;
}

export interface ProxyStatusResponse {
  assetId: string;
  status: ProxyState;
}
