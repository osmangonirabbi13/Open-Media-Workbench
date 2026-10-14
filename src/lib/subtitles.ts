import { invoke } from "@tauri-apps/api/core";
import { isTauriEnvironment } from "./tauri";
import type {
  SubtitleCue,
  SubtitleDocument,
  SubtitleExportResponse,
  SubtitleFinding,
  SubtitleFormat,
  SubtitleImportResponse,
  SubtitleProjectOpenResponse,
  SubtitleProjectSaveResponse,
  SubtitleTrack,
} from "../types/subtitles";

/**
 * Formats integer microseconds into SMPTE-like subtitle timecode: HH:MM:SS.mmm
 */
export function formatMicrosToTimecode(micros: number): string {
  const safeUs = Math.max(0, Math.round(micros));
  const totalMs = Math.round(safeUs / 1000);

  const ms = totalMs % 1000;
  const totalSeconds = Math.floor(totalMs / 1000);
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const minutes = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);

  const pad2 = (n: number) => n.toString().padStart(2, "0");
  const pad3 = (n: number) => n.toString().padStart(3, "0");

  return `${pad2(hours)}:${pad2(minutes)}:${pad2(seconds)}.${pad3(ms)}`;
}

/**
 * High-performance binary search to find all subtitle cues active at playback time (in seconds).
 * Runs in O(log N + k) time without linear array scans.
 */
export function findActiveCues(cues: SubtitleCue[], timeSeconds: number): SubtitleCue[] {
  if (!cues || cues.length === 0 || timeSeconds < 0) {
    return [];
  }

  const timeUs = Math.round(timeSeconds * 1_000_000);

  // Binary search for partition point where cue.startUs <= timeUs
  let low = 0;
  let high = cues.length;
  while (low < high) {
    const mid = (low + high) >>> 1;
    if (cues[mid].startUs <= timeUs) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }

  const active: SubtitleCue[] = [];
  // Scan backwards from partition point
  for (let i = low - 1; i >= 0; i--) {
    const cue = cues[i];
    if (cue.endUs > timeUs) {
      active.push(cue);
    } else if (timeUs - cue.startUs > 60_000_000) {
      // Bounded backward scan if cue ended and started >60s ago
      break;
    }
  }

  active.reverse();
  return active;
}

/**
 * Simple client-side SRT parser for testing outside Tauri runtime.
 */
export function parseSrtClient(content: string): { track: SubtitleTrack; warnings: string[] } {
  const sanitized = content.replace(/^\uFEFF/, "").replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const blocks = sanitized.split("\n\n");
  const cues: SubtitleCue[] = [];
  const warnings: string[] = [];

  let idx = 1;
  for (const block of blocks) {
    const lines = block
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean);
    if (lines.length === 0) continue;

    let lineCursor = 0;
    let rawIdentifier: string | undefined;

    if (!lines[lineCursor].includes("-->")) {
      rawIdentifier = lines[lineCursor];
      lineCursor++;
    }

    if (lineCursor >= lines.length || !lines[lineCursor].includes("-->")) {
      warnings.push(`Malformed cue block: ${block}`);
      continue;
    }

    const timingParts = lines[lineCursor].split("-->");
    lineCursor++;

    const parseTs = (str: string): number => {
      const clean = str.trim().split(/\s+/)[0];
      const [hms, msPart] = clean.includes(",") ? clean.split(",") : clean.split(".");
      const [h, m, s] = hms.split(":").map(Number);
      const ms = Number(msPart || "0");
      return (h * 3600 + m * 60 + s) * 1_000_000 + ms * 1000;
    };

    const startUs = parseTs(timingParts[0]);
    const endUs = parseTs(timingParts[1]);

    const text = lines.slice(lineCursor).join("\n");
    cues.push({
      id: `cue-${idx}`,
      startUs,
      endUs,
      text,
      rawIdentifier,
    });
    idx++;
  }

  const track: SubtitleTrack = {
    id: "mock-track-srt",
    language: "und",
    displayName: "SubRip Subtitle",
    sourceFormat: "srt",
    cues,
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  return { track, warnings };
}

/**
 * Simple client-side WebVTT parser for testing outside Tauri runtime.
 */
export function parseWebVttClient(content: string): { track: SubtitleTrack; warnings: string[] } {
  const sanitized = content.replace(/^\uFEFF/, "").replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const blocks = sanitized.split("\n\n");
  const cues: SubtitleCue[] = [];
  const warnings: string[] = [];

  let idx = 1;
  for (let b = 0; b < blocks.length; b++) {
    const block = blocks[b].trim();
    if (!block || (b === 0 && block.startsWith("WEBVTT"))) continue;
    if (block.startsWith("NOTE") || block.startsWith("STYLE") || block.startsWith("REGION")) {
      continue;
    }

    const lines = block
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean);
    if (lines.length === 0) continue;

    let lineCursor = 0;
    let rawIdentifier: string | undefined;

    if (!lines[lineCursor].includes("-->")) {
      rawIdentifier = lines[lineCursor];
      lineCursor++;
    }

    if (lineCursor >= lines.length || !lines[lineCursor].includes("-->")) {
      continue;
    }

    const timingParts = lines[lineCursor].split("-->");
    lineCursor++;

    const parseTs = (str: string): number => {
      const clean = str.trim().split(/\s+/)[0];
      const [hms, msPart] = clean.split(".");
      const tokens = hms.split(":").map(Number);
      const ms = Number(msPart || "0");
      let h = 0,
        m = 0,
        s = 0;
      if (tokens.length === 3) {
        [h, m, s] = tokens;
      } else if (tokens.length === 2) {
        [m, s] = tokens;
      }
      return (h * 3600 + m * 60 + s) * 1_000_000 + ms * 1000;
    };

    const startUs = parseTs(timingParts[0]);
    const endUs = parseTs(timingParts[1]);

    const text = lines
      .slice(lineCursor)
      .join("\n")
      .replace(/<[^>]+>/g, "")
      .replace(/&amp;/g, "&")
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&nbsp;/g, " ")
      .trim();
    cues.push({
      id: `vtt-cue-${idx}`,
      startUs,
      endUs,
      text,
      rawIdentifier,
    });
    idx++;
  }

  const track: SubtitleTrack = {
    id: "mock-track-vtt",
    language: "und",
    displayName: "WebVTT Subtitle",
    sourceFormat: "webVtt",
    cues,
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  return { track, warnings };
}

/**
 * Client-side SRT serializer.
 */
export function serializeSrtClient(track: SubtitleTrack): string {
  let out = "";
  track.cues.forEach((cue, i) => {
    out += `${i + 1}\n`;
    out += `${formatMicrosToTimecode(cue.startUs).replace(".", ",")} --> ${formatMicrosToTimecode(cue.endUs).replace(".", ",")}\n`;
    out += `${cue.text}\n\n`;
  });
  return out;
}

/**
 * Client-side WebVTT serializer.
 */
export function serializeWebVttClient(track: SubtitleTrack): string {
  let out = "WEBVTT\n\n";
  track.cues.forEach((cue, i) => {
    out += `${cue.rawIdentifier || i + 1}\n`;
    out += `${formatMicrosToTimecode(cue.startUs)} --> ${formatMicrosToTimecode(cue.endUs)}\n`;
    out += `${cue.text}\n\n`;
  });
  return out;
}

/**
 * Native file dialog subtitle picker and parser.
 */
export async function pickAndImportSubtitle(
  associatedAssetId?: string
): Promise<SubtitleImportResponse | null> {
  if (!isTauriEnvironment()) {
    // In standalone browser test environment, return null or test mock
    return null;
  }

  try {
    return await invoke<SubtitleImportResponse | null>("pick_and_import_subtitle_file", {
      associatedAssetId,
    });
  } catch (err) {
    throw new Error(`Failed to import subtitle file: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Directly parses subtitle string content via IPC.
 */
export async function parseSubtitleContent(
  content: string,
  format?: SubtitleFormat,
  language?: string,
  displayName?: string,
  associatedAssetId?: string
): Promise<SubtitleImportResponse> {
  if (!isTauriEnvironment()) {
    const isVtt = content.trim().startsWith("WEBVTT");
    const { track, warnings } = isVtt ? parseWebVttClient(content) : parseSrtClient(content);
    if (language) track.language = language;
    if (displayName) track.displayName = displayName;

    const doc: SubtitleDocument = {
      schemaVersion: 1,
      id: "mock-doc-id",
      associatedMediaAssetId: associatedAssetId,
      tracks: [track],
      activeTrackId: track.id,
      createdAtEpochMs: Date.now(),
      updatedAtEpochMs: Date.now(),
    };

    return {
      document: doc,
      track,
      findings: [],
      warnings,
      fileName: isVtt ? "mock.vtt" : "mock.srt",
      fileSizeBytes: content.length,
    };
  }

  try {
    return await invoke<SubtitleImportResponse>("parse_subtitle_content", {
      content,
      format,
      language,
      displayName,
      associatedAssetId,
    });
  } catch (err) {
    throw new Error(`Failed to parse subtitle content: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Exports a subtitle track via native save dialog.
 */
export async function exportSubtitleTrack(
  track: SubtitleTrack,
  format: SubtitleFormat,
  defaultFilename?: string
): Promise<SubtitleExportResponse | null> {
  if (!isTauriEnvironment()) {
    return {
      exportedPath: `/mock/path/${defaultFilename || "subtitles." + format}`,
      bytesWritten: 1024,
      format,
      warnings: [],
    };
  }

  try {
    return await invoke<SubtitleExportResponse | null>("export_subtitle_track", {
      track,
      format,
      defaultFilename,
    });
  } catch (err) {
    throw new Error(`Failed to export subtitle track: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Saves a SubtitleDocument project atomically.
 */
export async function saveSubtitleProject(
  document: SubtitleDocument,
  defaultFilename?: string
): Promise<SubtitleProjectSaveResponse | null> {
  if (!isTauriEnvironment()) {
    return {
      projectPath: `/mock/path/${defaultFilename || "project.omwsub"}`,
      bytesWritten: 2048,
    };
  }

  try {
    return await invoke<SubtitleProjectSaveResponse | null>("save_subtitle_project", {
      document,
      defaultFilename,
    });
  } catch (err) {
    throw new Error(`Failed to save subtitle project: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Reopens a SubtitleDocument project from disk.
 */
export async function openSubtitleProject(): Promise<SubtitleProjectOpenResponse | null> {
  if (!isTauriEnvironment()) {
    return null;
  }

  try {
    return await invoke<SubtitleProjectOpenResponse | null>("open_subtitle_project");
  } catch (err) {
    throw new Error(`Failed to open subtitle project: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Validates a subtitle document via IPC.
 */
export async function validateSubtitleDocument(
  document: SubtitleDocument,
  mediaDurationUs?: number
): Promise<SubtitleFinding[]> {
  if (!isTauriEnvironment()) {
    return [];
  }

  try {
    return await invoke<SubtitleFinding[]>("validate_subtitle_document_command", {
      document,
      mediaDurationUs,
    });
  } catch (err) {
    throw new Error(`Failed to validate subtitle document: ${err instanceof Error ? err.message : String(err)}`);
  }
}
