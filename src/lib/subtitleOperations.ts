import { invoke } from "@tauri-apps/api/core";
import type {
  SnapResult,
  SubtitleCue,
  SubtitleDocument,
  SubtitleOperation,
  SubtitleOperationResponse,
  SubtitleTrack,
} from "../types/subtitles";

/**
 * Parses a timestamp formatted as HH:MM:SS.mmm (or MM:SS.mmm) into integer microseconds.
 * Returns null if the string is invalid or negative.
 */
export function parseHmsMsToMicros(str: string): number | null {
  const clean = str.trim();
  if (!clean) return null;

  // Pattern: Optional HH:, MM:, SS, optional .mmm or ,mmm
  const regex = /^(?:(?:(\d{1,2}):)?(\d{1,2}):)?(\d{1,2})(?:[.,](\d{1,3}))?$/;
  const match = clean.match(regex);
  if (!match) return null;

  const hours = match[1] ? parseInt(match[1], 10) : 0;
  const minutes = match[2] ? parseInt(match[2], 10) : 0;
  const seconds = parseInt(match[3], 10);
  const msRaw = match[4] || "0";
  const ms = parseInt(msRaw.padEnd(3, "0").slice(0, 3), 10);

  if (hours < 0 || minutes < 0 || minutes >= 60 || seconds < 0 || seconds >= 60 || ms < 0 || ms >= 1000) {
    return null;
  }

  const totalUs =
    hours * 3_600_000_000 +
    minutes * 60_000_000 +
    seconds * 1_000_000 +
    ms * 1_000;

  return totalUs;
}

/**
 * Formats integer microseconds into HH:MM:SS.mmm format for display.
 */
export function formatMicrosToHmsMs(micros: number): string {
  const totalMs = Math.round(Math.max(0, micros) / 1000);
  const ms = totalMs % 1000;
  const totalSec = Math.floor(totalMs / 1000);
  const sec = totalSec % 60;
  const totalMin = Math.floor(totalSec / 60);
  const min = totalMin % 60;
  const hours = Math.floor(totalMin / 60);

  const pad = (n: number, w: number = 2) => String(n).padStart(w, "0");
  return `${pad(hours)}:${pad(min)}:${pad(sec)}.${pad(ms, 3)}`;
}

/**
 * Splits subtitle text cleanly at a newline or midpoint word boundary, safely preserving Unicode and Bengali.
 */
export function splitSubtitleText(text: string): [string, string] {
  const trimmed = text.trim();
  if (!trimmed) return ["", ""];

  if (trimmed.includes("\n")) {
    const idx = trimmed.indexOf("\n");
    const first = trimmed.slice(0, idx).trim();
    const second = trimmed.slice(idx + 1).trim();
    if (first && second) return [first, second];
  }

  const words = trimmed.split(/\s+/);
  if (words.length >= 2) {
    const mid = Math.floor(words.length / 2);
    return [words.slice(0, mid).join(" "), words.slice(mid).join(" ")];
  }

  const chars = Array.from(trimmed);
  if (chars.length >= 2) {
    const mid = Math.floor(chars.length / 2);
    return [chars.slice(0, mid).join(""), chars.slice(mid).join("")];
  }

  return [trimmed, ""];
}

/**
 * Applies a subtitle editing operation to a track deterministically in pure TypeScript.
 * Enforces microsecond timestamps and sorting invariants matching the Rust core.
 */
export function applyTrackOperation(
  track: SubtitleTrack,
  op: SubtitleOperation
): SubtitleTrack {
  const cues = [...track.cues];

  switch (op.type) {
    case "addCue": {
      const { cue } = op.payload;
      if (cue.startUs < 0 || cue.endUs <= cue.startUs) {
        throw new Error(`Invalid cue timing interval: [${cue.startUs}, ${cue.endUs}]`);
      }
      cues.push({ ...cue });
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "deleteCue": {
      const filtered = cues.filter((c) => c.id !== op.payload.cueId);
      if (filtered.length === cues.length) {
        throw new Error(`Cue with ID '${op.payload.cueId}' not found for deletion`);
      }
      return { ...track, cues: filtered };
    }

    case "deleteCues": {
      const filtered = cues.filter((c) => !op.payload.cueIds.includes(c.id));
      return { ...track, cues: filtered };
    }

    case "updateCueText": {
      const idx = cues.findIndex((c) => c.id === op.payload.cueId);
      if (idx === -1) {
        throw new Error(`Cue with ID '${op.payload.cueId}' not found`);
      }
      cues[idx] = { ...cues[idx], text: op.payload.text };
      return { ...track, cues };
    }

    case "updateCueSpeaker": {
      const idx = cues.findIndex((c) => c.id === op.payload.cueId);
      if (idx === -1) {
        throw new Error(`Cue with ID '${op.payload.cueId}' not found`);
      }
      cues[idx] = { ...cues[idx], speaker: op.payload.speaker };
      return { ...track, cues };
    }

    case "setCueTiming": {
      const { cueId, startUs, endUs } = op.payload;
      if (startUs < 0 || endUs <= startUs) {
        throw new Error(`Invalid cue timing: [${startUs}, ${endUs}]`);
      }
      const idx = cues.findIndex((c) => c.id === cueId);
      if (idx === -1) {
        throw new Error(`Cue with ID '${cueId}' not found`);
      }
      cues[idx] = { ...cues[idx], startUs, endUs };
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "moveCue": {
      const { cueId, deltaUs } = op.payload;
      const idx = cues.findIndex((c) => c.id === cueId);
      if (idx === -1) {
        throw new Error(`Cue with ID '${cueId}' not found`);
      }
      const newStart = cues[idx].startUs + deltaUs;
      const newEnd = cues[idx].endUs + deltaUs;
      if (newStart < 0) {
        throw new Error(`Moving cue would result in negative start time: ${newStart}µs`);
      }
      cues[idx] = { ...cues[idx], startUs: newStart, endUs: newEnd };
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "resizeCueStart": {
      const { cueId, newStartUs } = op.payload;
      const idx = cues.findIndex((c) => c.id === cueId);
      if (idx === -1) {
        throw new Error(`Cue with ID '${cueId}' not found`);
      }
      if (newStartUs < 0 || newStartUs >= cues[idx].endUs) {
        throw new Error(`Invalid new start time: ${newStartUs}µs`);
      }
      cues[idx] = { ...cues[idx], startUs: newStartUs };
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "resizeCueEnd": {
      const { cueId, newEndUs } = op.payload;
      const idx = cues.findIndex((c) => c.id === cueId);
      if (idx === -1) {
        throw new Error(`Cue with ID '${cueId}' not found`);
      }
      if (newEndUs <= cues[idx].startUs) {
        throw new Error(`Invalid new end time: ${newEndUs}µs`);
      }
      cues[idx] = { ...cues[idx], endUs: newEndUs };
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "splitCue": {
      const { cueId, splitTimeUs } = op.payload;
      const idx = cues.findIndex((c) => c.id === cueId);
      if (idx === -1) {
        throw new Error(`Cue with ID '${cueId}' not found`);
      }
      const target = cues[idx];
      if (splitTimeUs <= target.startUs || splitTimeUs >= target.endUs) {
        throw new Error(`Split time ${splitTimeUs}µs is outside cue bounds`);
      }

      const [firstText, secondText] = splitSubtitleText(target.text);
      const originalEnd = target.endUs;

      cues[idx] = {
        ...target,
        endUs: splitTimeUs,
        text: firstText,
      };

      const newCue: SubtitleCue = {
        id: crypto.randomUUID(),
        startUs: splitTimeUs,
        endUs: originalEnd,
        text: secondText,
        speaker: target.speaker,
        settings: target.settings,
      };

      cues.push(newCue);
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "mergeCues": {
      const { firstCueId, secondCueId } = op.payload;
      const firstIdx = cues.findIndex((c) => c.id === firstCueId);
      const secondIdx = cues.findIndex((c) => c.id === secondCueId);
      if (firstIdx === -1 || secondIdx === -1 || firstIdx === secondIdx) {
        throw new Error("Invalid cues specified for merge");
      }

      const [earlierIdx, laterIdx] =
        cues[firstIdx].startUs <= cues[secondIdx].startUs
          ? [firstIdx, secondIdx]
          : [secondIdx, firstIdx];

      const earlier = cues[earlierIdx];
      const later = cues[laterIdx];

      const mergedEnd = Math.max(earlier.endUs, later.endUs);
      const mergedText = earlier.text.trim()
        ? later.text.trim()
          ? `${earlier.text.trim()}\n${later.text.trim()}`
          : earlier.text
        : later.text;

      cues[earlierIdx] = {
        ...earlier,
        endUs: mergedEnd,
        text: mergedText,
      };

      cues.splice(laterIdx, 1);
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "shiftSelectedCues": {
      const { cueIds, deltaUs } = op.payload;
      if (cueIds.length === 0 || deltaUs === 0) return track;

      // Validate all first (atomic)
      for (const cue of cues) {
        if (cueIds.includes(cue.id)) {
          const newStart = cue.startUs + deltaUs;
          if (newStart < 0) {
            throw new Error(`Batch shift would cause cue '${cue.id}' to have negative time`);
          }
        }
      }

      const updated = cues.map((cue) => {
        if (cueIds.includes(cue.id)) {
          return {
            ...cue,
            startUs: cue.startUs + deltaUs,
            endUs: cue.endUs + deltaUs,
          };
        }
        return cue;
      });

      updated.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues: updated };
    }

    case "duplicateCue": {
      const target = cues.find((c) => c.id === op.payload.cueId);
      if (!target) {
        throw new Error(`Cue with ID '${op.payload.cueId}' not found`);
      }
      const duration = target.endUs - target.startUs;
      const duplicated: SubtitleCue = {
        ...target,
        id: crypto.randomUUID(),
        startUs: target.endUs,
        endUs: target.endUs + duration,
      };
      cues.push(duplicated);
      cues.sort((a, b) => a.startUs - b.startUs || a.endUs - b.endUs);
      return { ...track, cues };
    }

    case "updateTrackMetadata": {
      return {
        ...track,
        language: op.payload.language ?? track.language,
        displayName: op.payload.displayName ?? track.displayName,
      };
    }

    default:
      return track;
  }
}

/**
 * Applies an operation to a SubtitleDocument client-side.
 */
export function applySubtitleOperation(
  doc: SubtitleDocument,
  op: SubtitleOperation
): SubtitleDocument {
  const trackId = "trackId" in op.payload ? op.payload.trackId : doc.activeTrackId;
  const tracks = doc.tracks.map((t) => {
    if (t.id === trackId) {
      return applyTrackOperation(t, op);
    }
    return t;
  });

  return {
    ...doc,
    tracks,
    updatedAtEpochMs: Date.now(),
  };
}

/**
 * Snapping engine for timeline cue dragging and boundary resizing.
 */
export function calculateSnap({
  targetTimeUs,
  playheadUs,
  cues,
  excludeCueIds = [],
  viewportStartUs,
  viewportEndUs,
  pixelWidth,
  snapThresholdPx = 8,
  enabled = true,
}: {
  targetTimeUs: number;
  playheadUs: number;
  cues: SubtitleCue[];
  excludeCueIds?: string[];
  viewportStartUs: number;
  viewportEndUs: number;
  pixelWidth: number;
  snapThresholdPx?: number;
  enabled?: boolean;
}): SnapResult | null {
  if (!enabled || pixelWidth <= 0) return null;

  const spanUs = Math.max(1, viewportEndUs - viewportStartUs);
  const snapThresholdUs = (snapThresholdPx / pixelWidth) * spanUs;

  let bestSnap: SnapResult | null = null;
  let minDiff = Infinity;

  // 1. Check Playhead
  const playheadDiff = Math.abs(targetTimeUs - playheadUs);
  if (playheadDiff <= snapThresholdUs && playheadDiff < minDiff) {
    minDiff = playheadDiff;
    const px = ((playheadUs - viewportStartUs) / spanUs) * pixelWidth;
    bestSnap = {
      snappedTimeUs: playheadUs,
      snappedPixelX: px,
      targetType: "playhead",
      targetTimeUs: playheadUs,
      label: "Playhead",
    };
  }

  // 2. Check Neighboring Cues (Start and End times)
  for (const cue of cues) {
    if (excludeCueIds.includes(cue.id)) continue;

    const startDiff = Math.abs(targetTimeUs - cue.startUs);
    if (startDiff <= snapThresholdUs && startDiff < minDiff) {
      minDiff = startDiff;
      const px = ((cue.startUs - viewportStartUs) / spanUs) * pixelWidth;
      bestSnap = {
        snappedTimeUs: cue.startUs,
        snappedPixelX: px,
        targetType: "cueStart",
        targetTimeUs: cue.startUs,
        label: `Cue Start`,
      };
    }

    const endDiff = Math.abs(targetTimeUs - cue.endUs);
    if (endDiff <= snapThresholdUs && endDiff < minDiff) {
      minDiff = endDiff;
      const px = ((cue.endUs - viewportStartUs) / spanUs) * pixelWidth;
      bestSnap = {
        snappedTimeUs: cue.endUs,
        snappedPixelX: px,
        targetType: "cueEnd",
        targetTimeUs: cue.endUs,
        label: `Cue End`,
      };
    }
  }

  return bestSnap;
}

/**
 * Calculates subtitle readability metrics and non-blocking warnings.
 */
export function calculateReadability(cue: SubtitleCue): {
  charCount: number;
  lineCount: number;
  maxCharsPerLine: number;
  durationSeconds: number;
  charsPerSecond: number;
  warning?: string;
} {
  const text = cue.text || "";
  const lines = text.split("\n");
  const charCount = text.length;
  const lineCount = lines.length;
  const maxCharsPerLine = Math.max(...lines.map((l) => l.length), 0);
  const durationSeconds = Math.max(0.1, (cue.endUs - cue.startUs) / 1_000_000);
  const charsPerSecond = Math.round((charCount / durationSeconds) * 10) / 10;

  let warning: string | undefined;
  if (maxCharsPerLine > 42) {
    warning = `Line exceeds recommended 42 characters (${maxCharsPerLine} chars)`;
  } else if (charsPerSecond > 25) {
    warning = `Reading speed is high (${charsPerSecond} chars/s)`;
  }

  return {
    charCount,
    lineCount,
    maxCharsPerLine,
    durationSeconds,
    charsPerSecond,
    warning,
  };
}

/**
 * IPC invocation to apply an operation in Rust core with full validation.
 */
export async function applySubtitleOperationIPC(
  document: SubtitleDocument,
  operation: SubtitleOperation,
  mediaDurationUs?: number
): Promise<SubtitleOperationResponse> {
  const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  if (!isTauri) {
    const updatedDoc = applySubtitleOperation(document, operation);
    return {
      document: updatedDoc,
      findings: [],
    };
  }

  return invoke<SubtitleOperationResponse>("apply_subtitle_operation_command", {
    document,
    operation,
    mediaDurationUs,
  });
}
