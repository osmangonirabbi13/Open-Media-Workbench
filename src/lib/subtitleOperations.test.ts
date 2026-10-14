import { test } from "node:test";
import assert from "node:assert/strict";
import type { SubtitleCue, SubtitleDocument, SubtitleTrack } from "../types/subtitles";
import {
  applySubtitleOperation,
  applyTrackOperation,
  calculateReadability,
  calculateSnap,
  formatMicrosToHmsMs,
  parseHmsMsToMicros,
  splitSubtitleText,
} from "./subtitleOperations";
import {
  saveAutosaveSnapshot,
  loadAutosaveSnapshot,
  clearAutosaveSnapshot,
} from "./subtitleAutosave";

function createMockTrack(): SubtitleTrack {
  return {
    id: "track-1",
    language: "en",
    displayName: "English (Original)",
    sourceFormat: "srt",
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
    cues: [
      {
        id: "c-1",
        startUs: 1_000_000,
        endUs: 3_000_000,
        text: "First subtitle line",
      },
      {
        id: "c-2",
        startUs: 4_000_000,
        endUs: 7_000_000,
        text: "Second subtitle dialogue\nWith multiple lines",
      },
    ],
  };
}

function createMockDoc(): SubtitleDocument {
  return {
    schemaVersion: 1,
    id: "doc-1",
    associatedMediaAssetId: "asset-1",
    tracks: [createMockTrack()],
    activeTrackId: "track-1",
    createdAtEpochMs: 1000,
    updatedAtEpochMs: 1000,
  };
}

test("Timestamp Formatter and Parser: round-trip precision", () => {
  const micros = 3_661_234_000; // 1 hour, 1 min, 1.234s
  const formatted = formatMicrosToHmsMs(micros);
  assert.equal(formatted, "01:01:01.234");

  const parsed = parseHmsMsToMicros(formatted);
  assert.equal(parsed, micros);

  // Short format MM:SS.mmm
  assert.equal(parseHmsMsToMicros("02:30.500"), 150_500_000);
  assert.equal(parseHmsMsToMicros("invalid"), null);
  assert.equal(parseHmsMsToMicros(""), null);
});

test("Subtitle Operation: Add Cue and Invariant Ordering", () => {
  const track = createMockTrack();
  const newCue: SubtitleCue = {
    id: "c-3",
    startUs: 500_000,
    endUs: 900_000,
    text: "Intro cue",
  };

  const updated = applyTrackOperation(track, {
    type: "addCue",
    payload: { trackId: track.id, cue: newCue },
  });

  assert.equal(updated.cues.length, 3);
  // Chronologically sorted: c-3 should be first
  assert.equal(updated.cues[0].id, "c-3");
});

test("Subtitle Operation: Update Text and Bengali Unicode Preservation", () => {
  const track = createMockTrack();
  const bengali = "বাংলা সাবটাইটেল সম্পাদনা পরীক্ষা";
  const updated = applyTrackOperation(track, {
    type: "updateCueText",
    payload: { trackId: track.id, cueId: "c-1", text: bengali },
  });

  assert.equal(updated.cues[0].text, bengali);
});

test("Subtitle Operation: Move and Boundary Resize", () => {
  const track = createMockTrack();
  // Move c-1 by +500ms
  const moved = applyTrackOperation(track, {
    type: "moveCue",
    payload: { trackId: track.id, cueId: "c-1", deltaUs: 500_000 },
  });
  assert.equal(moved.cues[0].startUs, 1_500_000);
  assert.equal(moved.cues[0].endUs, 3_500_000);

  // Resize start
  const resizedStart = applyTrackOperation(moved, {
    type: "resizeCueStart",
    payload: { trackId: track.id, cueId: "c-1", newStartUs: 1_200_000 },
  });
  assert.equal(resizedStart.cues[0].startUs, 1_200_000);

  // Resize end
  const resizedEnd = applyTrackOperation(resizedStart, {
    type: "resizeCueEnd",
    payload: { trackId: track.id, cueId: "c-1", newEndUs: 3_800_000 },
  });
  assert.equal(resizedEnd.cues[0].endUs, 3_800_000);
});

test("Subtitle Operation: Split and Merge", () => {
  const track = createMockTrack();
  // Split c-2 (4s to 7s) at 5.5s
  const split = applyTrackOperation(track, {
    type: "splitCue",
    payload: { trackId: track.id, cueId: "c-2", splitTimeUs: 5_500_000 },
  });
  assert.equal(split.cues.length, 3);
  assert.equal(split.cues[1].endUs, 5_500_000);
  assert.equal(split.cues[2].startUs, 5_500_000);
  assert.equal(split.cues[2].endUs, 7_000_000);

  // Merge back
  const merged = applyTrackOperation(split, {
    type: "mergeCues",
    payload: {
      trackId: track.id,
      firstCueId: split.cues[1].id,
      secondCueId: split.cues[2].id,
    },
  });
  assert.equal(merged.cues.length, 2);
  assert.equal(merged.cues[1].startUs, 4_000_000);
  assert.equal(merged.cues[1].endUs, 7_000_000);
});

test("Subtitle Operation: Batch Shift and Negative Time Rejection", () => {
  const track = createMockTrack();
  // Valid batch shift
  const shifted = applyTrackOperation(track, {
    type: "shiftSelectedCues",
    payload: { trackId: track.id, cueIds: ["c-1", "c-2"], deltaUs: 200_000 },
  });
  assert.equal(shifted.cues[0].startUs, 1_200_000);
  assert.equal(shifted.cues[1].startUs, 4_200_000);

  // Negative time shift must throw and reject
  assert.throws(() => {
    applyTrackOperation(track, {
      type: "shiftSelectedCues",
      payload: { trackId: track.id, cueIds: ["c-1", "c-2"], deltaUs: -2_000_000 },
    });
  });
});

test("Snapping Engine: Snapping to Playhead and Cue Boundaries", () => {
  const cues: SubtitleCue[] = [
    { id: "c-1", startUs: 2_000_000, endUs: 4_000_000, text: "A" },
    { id: "c-2", startUs: 5_000_000, endUs: 8_000_000, text: "B" },
  ];

  // Snapping to playhead at 2,050,000 µs (with 10s span and 1000px width, 1px = 10,000µs, threshold 80,000µs)
  const snap1 = calculateSnap({
    targetTimeUs: 2_050_000,
    playheadUs: 2_000_000,
    cues,
    excludeCueIds: ["c-1"],
    viewportStartUs: 0,
    viewportEndUs: 10_000_000,
    pixelWidth: 1000,
    snapThresholdPx: 8,
    enabled: true,
  });
  assert.ok(snap1);
  assert.equal(snap1.targetType, "playhead");
  assert.equal(snap1.snappedTimeUs, 2_000_000);

  // Snapping disabled
  const snapDisabled = calculateSnap({
    targetTimeUs: 2_050_000,
    playheadUs: 2_000_000,
    cues,
    viewportStartUs: 0,
    viewportEndUs: 10_000_000,
    pixelWidth: 1000,
    enabled: false,
  });
  assert.equal(snapDisabled, null);
});

test("Readability Metrics: Character Count and Warnings", () => {
  const normalCue: SubtitleCue = {
    id: "c-norm",
    startUs: 0,
    endUs: 2_000_000, // 2s
    text: "Short dialogue line",
  };
  const normRead = calculateReadability(normalCue);
  assert.equal(normRead.charCount, 19);
  assert.equal(normRead.warning, undefined);

  // Very long line (> 42 chars)
  const longCue: SubtitleCue = {
    id: "c-long",
    startUs: 0,
    endUs: 2_000_000,
    text: "This is an extremely long subtitle line that surpasses standard forty-two character length recommendations",
  };
  const longRead = calculateReadability(longCue);
  assert.ok(longRead.warning?.includes("exceeds recommended 42 characters"));
});

test("Autosave and Recovery Persistence", () => {
  // Mock localStorage for node environment
  const store: Record<string, string> = {};
  globalThis.localStorage = {
    getItem: (k: string) => store[k] || null,
    setItem: (k: string, v: string) => {
      store[k] = v;
    },
    removeItem: (k: string) => {
      delete store[k];
    },
    clear: () => {},
    key: () => null,
    length: 0,
  };

  const doc = createMockDoc();
  saveAutosaveSnapshot(doc, "asset-1");

  const loaded = loadAutosaveSnapshot("asset-1");
  assert.ok(loaded);
  assert.equal(loaded.document.id, doc.id);
  assert.equal(loaded.document.tracks[0].cues.length, 2);

  clearAutosaveSnapshot("asset-1");
  assert.equal(loadAutosaveSnapshot("asset-1"), null);
});

test("Subtitle Text Split: multiline and word boundary division", () => {
  const [l1, l2] = splitSubtitleText("Line one\nLine two");
  assert.equal(l1, "Line one");
  assert.equal(l2, "Line two");

  const [w1, w2] = splitSubtitleText("Four words in dialogue");
  assert.equal(w1, "Four words");
  assert.equal(w2, "in dialogue");
});

test("Subtitle Document Operation: document level dispatch", () => {
  const doc = createMockDoc();
  const updated = applySubtitleOperation(doc, {
    type: "updateCueText",
    payload: {
      trackId: "track-1",
      cueId: "c-1",
      text: "Document level edit",
    },
  });
  assert.equal(updated.tracks[0].cues[0].text, "Document level edit");
});

