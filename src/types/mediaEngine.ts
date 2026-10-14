/**
 * Typed contracts matching the Rust `opentake_core::media::capabilities` models.
 */

export type MediaEngineCompatibility =
  | "ready"
  | "degraded"
  | "unsupported"
  | "missing"
  | "error";

export interface ToolStatus {
  available: boolean;
  executable_path?: string | null;
  canonical_path?: string | null;
  version?: string | null;
  major_version?: number | null;
  is_gpl: boolean;
  has_libx264: boolean;
  has_libopus: boolean;
  error?: string | null;
}

export interface MediaEngineStatus {
  overall_status: MediaEngineCompatibility;
  ffmpeg: ToolStatus;
  ffprobe: ToolStatus;
  warnings: string[];
  evaluated_at_epoch_ms: number;
}
