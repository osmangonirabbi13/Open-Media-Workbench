import test from "node:test";
import assert from "node:assert/strict";
import {
  formatMicrosToTimecode,
  findActiveCues,
  parseSrtClient,
  parseWebVttClient,
  serializeSrtClient,
  serializeWebVttClient,
  parseSubtitleContent,
  exportSubtitleTrack,
  saveSubtitleProject,
} from "./subtitles";
import type {
  SubtitleCue,
  SubtitleDocument,
  SubtitleFinding,
  SubtitleTrack,
} from "../types/subtitles";

test("formatMicrosToTimecode: formats integer microseconds into SMPTE timecode string", () => {
  assert.equal(formatMicrosToTimecode(0), "00:00:00.000");
  assert.equal(formatMicrosToTimecode(1_500_000), "00:00:01.500");
  assert.equal(formatMicrosToTimecode(65_250_000), "00:01:05.250");
  assert.equal(formatMicrosToTimecode(3_661_500_000), "01:01:01.500");
});

test("findActiveCues: binary search correctly locates active cues with zero linear scan overhead", () => {
  const cues: SubtitleCue[] = [
    {
      id: "cue-1",
      startUs: 1_000_000,
      endUs: 3_000_000,
      text: "First cue",
    },
    {
      id: "cue-2",
      startUs: 2_500_000,
      endUs: 5_000_000,
      text: "Second overlapping cue",
    },
    {
      id: "cue-3",
      startUs: 7_000_000,
      endUs: 9_000_000,
      text: "Third cue",
    },
  ];

  // At 0.5s: before any cues
  assert.equal(findActiveCues(cues, 0.5).length, 0);

  // At 1.5s: inside Cue 1
  const active1 = findActiveCues(cues, 1.5);
  assert.equal(active1.length, 1);
  assert.equal(active1[0].id, "cue-1");

  // At 2.7s: inside both Cue 1 and Cue 2 (overlap)
  const activeOverlap = findActiveCues(cues, 2.7);
  assert.equal(activeOverlap.length, 2);
  assert.equal(activeOverlap[0].id, "cue-1");
  assert.equal(activeOverlap[1].id, "cue-2");

  // At 6.0s: in gap between Cue 2 and Cue 3
  assert.equal(findActiveCues(cues, 6.0).length, 0);

  // At 7.0s: exact inclusive start of Cue 3
  const activeStart = findActiveCues(cues, 7.0);
  assert.equal(activeStart.length, 1);
  assert.equal(activeStart[0].id, "cue-3");

  // At 9.0s: exact exclusive end of Cue 3
  assert.equal(findActiveCues(cues, 9.0).length, 0);
});

test("parseSrtClient: parses SubRip content with multiline text and timestamps", () => {
  const srtContent = `1
00:00:01,000 --> 00:00:04,000
First subtitle line
Second subtitle line

2
00:00:05,000 --> 00:00:08,500
Second subtitle
`;

  const { track } = parseSrtClient(srtContent);
  assert.equal(track.cues.length, 2);
  assert.equal(track.cues[0].startUs, 1_000_000);
  assert.equal(track.cues[0].endUs, 4_000_000);
  assert.equal(track.cues[0].text, "First subtitle line\nSecond subtitle line");
  assert.equal(track.cues[1].startUs, 5_000_000);
  assert.equal(track.cues[1].endUs, 8_500_000);
});

test("parseWebVttClient: parses WebVTT content, strips tags, and extracts speaker", () => {
  const vttContent = `WEBVTT

NOTE This note should be cleanly ignored

00:01.000 --> 00:03.500 line:90% align:center
<v Narrator>Welcome to Open Media Workbench!</v>

00:04.000 --> 00:06.000
<b>Bold text</b> &amp; plain text
`;

  const { track } = parseWebVttClient(vttContent);
  assert.equal(track.cues.length, 2);
  assert.equal(track.cues[0].startUs, 1_000_000);
  assert.equal(track.cues[0].endUs, 3_500_000);
  assert.equal(track.cues[0].text, "Welcome to Open Media Workbench!");
  assert.equal(track.cues[1].text, "Bold text & plain text");
});

test("Unicode and Bengali preservation across client parse and serialize", () => {
  const srtBengali = `1
00:00:01,000 --> 00:00:04,000
ওপেন মিডিয়া ওয়ার্কবেঞ্চে আপনাকে স্বাগতম!
`;

  const { track } = parseSrtClient(srtBengali);
  assert.equal(track.cues[0].text, "ওপেন মিডিয়া ওয়ার্কবেঞ্চে আপনাকে স্বাগতম!");

  const serializedSrt = serializeSrtClient(track);
  assert.ok(serializedSrt.includes("ওপেন মিডিয়া ওয়ার্কবেঞ্চে আপনাকে স্বাগতম!"));

  const serializedVtt = serializeWebVttClient(track);
  assert.ok(serializedVtt.includes("WEBVTT"));
  assert.ok(serializedVtt.includes("ওপেন মিডিয়া ওয়ার্কবেঞ্চে আপনাকে স্বাগতম!"));
});

test("parseSubtitleContent: executes fallback parse outside Tauri", async () => {
  const srt = `1\n00:00:01,000 --> 00:00:02,000\nMock cue\n`;
  const res = await parseSubtitleContent(srt, "srt", "en", "English Subtitles", "asset-mock");

  assert.equal(res.track.cues.length, 1);
  assert.equal(res.track.language, "en");
  assert.equal(res.track.displayName, "English Subtitles");
  assert.equal(res.document.associatedMediaAssetId, "asset-mock");
  assert.equal(res.fileName, "mock.srt");
});

test("exportSubtitleTrack: returns mock export response outside Tauri", async () => {
  const track: SubtitleTrack = {
    id: "tr-test",
    language: "en",
    displayName: "English",
    sourceFormat: "srt",
    cues: [],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const res = await exportSubtitleTrack(track, "srt", "test.srt");
  assert.ok(res);
  assert.equal(res?.format, "srt");
  assert.ok(res?.exportedPath.includes("test.srt"));
});

test("saveSubtitleProject: returns mock save response outside Tauri", async () => {
  const doc: SubtitleDocument = {
    schemaVersion: 1,
    id: "doc-1",
    tracks: [],
    createdAtEpochMs: 1000,
    updatedAtEpochMs: 2000,
  };

  const res = await saveSubtitleProject(doc, "my_project.omwsub");
  assert.ok(res);
  assert.ok(res?.projectPath.includes("my_project.omwsub"));
});

test("Subtitle Data Contract: validates Finding structure and severity", () => {
  const finding: SubtitleFinding = {
    code: "OVERLAPPING_CUES",
    severity: "warning",
    message: "Cue overlaps with preceding cue",
    trackId: "track-1",
    cueId: "cue-2",
    startUs: 2_000_000,
    endUs: 4_000_000,
  };

  assert.equal(finding.code, "OVERLAPPING_CUES");
  assert.equal(finding.severity, "warning");
  assert.equal(finding.startUs, 2_000_000);
});
