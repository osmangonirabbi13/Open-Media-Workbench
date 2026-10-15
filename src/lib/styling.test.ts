import test from "node:test";
import assert from "node:assert/strict";
import {
  colorRgbaToCss,
  colorRgbaToHex,
  DEFAULT_SUBTITLE_STYLE,
  deleteCustomSubtitleStylePreset,
  finalizeBurnInExport,
  generateAssScriptPreview,
  getSubtitleBurnInStatus,
  getSubtitleStylePresets,
  getSystemFonts,
  parseHexToColorRgba,
  saveCustomSubtitleStylePreset,
  startSubtitleBurnInExport,
} from "./styling";
import type { SubtitleStylePreset } from "../types/styling";
import type { SubtitleTrack } from "../types/subtitles";

test("Color formatters and parsers", () => {
  const white = { r: 255, g: 255, b: 255, a: 255 };
  assert.equal(colorRgbaToCss(white), "rgba(255, 255, 255, 1.000)");
  assert.equal(colorRgbaToHex(white), "#FFFFFF");

  const yellow = { r: 245, g: 158, b: 11, a: 255 };
  assert.equal(colorRgbaToHex(yellow), "#F59E0B");

  const parsed = parseHexToColorRgba("#F59E0B");
  assert.equal(parsed.r, 245);
  assert.equal(parsed.g, 158);
  assert.equal(parsed.b, 11);
  assert.equal(parsed.a, 255);
});

test("Built-in presets list contains 8 professional templates", async () => {
  const presets = await getSubtitleStylePresets();
  assert.equal(presets.length >= 8, true);

  const ids = presets.map((p) => p.id);
  assert.equal(ids.includes("clean-minimal"), true);
  assert.equal(ids.includes("cinematic"), true);
  assert.equal(ids.includes("modern-outline"), true);
  assert.equal(ids.includes("social-bold"), true);
  assert.equal(ids.includes("karaoke-highlight"), true);
  assert.equal(ids.includes("documentary"), true);
  assert.equal(ids.includes("accessible-high-contrast"), true);
  assert.equal(ids.includes("soft-background-box"), true);
});

test("Custom preset creation and deletion in test environment", async () => {
  const customPreset: SubtitleStylePreset = {
    id: "test-custom-preset",
    displayName: "Test Custom Preset",
    description: "Custom style for test suite",
    isBuiltin: false,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "test-custom-preset",
      displayName: "Test Custom Preset",
      fontSize: 42,
    },
  };

  await saveCustomSubtitleStylePreset(customPreset);
  const presetsAfter = await getSubtitleStylePresets();
  assert.equal(presetsAfter.some((p) => p.id === "test-custom-preset"), true);

  await deleteCustomSubtitleStylePreset("test-custom-preset");
  const presetsFinal = await getSubtitleStylePresets();
  assert.equal(presetsFinal.some((p) => p.id === "test-custom-preset"), false);
});

test("System fonts discovery returns fonts with Bengali support flag", async () => {
  const fonts = await getSystemFonts();
  assert.equal(fonts.length > 0, true);

  const bengaliFont = fonts.find((f) => f.family.includes("Bengali"));
  assert.equal(Boolean(bengaliFont), true);
  if (bengaliFont) {
    assert.equal(bengaliFont.supportsBengali, true);
  }
});

test("ASS script preview generation produces valid header and dialogue", async () => {
  const mockTrack: SubtitleTrack = {
    id: "track-test",
    language: "en",
    displayName: "English",
    sourceFormat: "srt",
    cues: [
      {
        id: "cue-1",
        startUs: 0,
        endUs: 3_000_000,
        text: "Hello World",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const ass = await generateAssScriptPreview(mockTrack, DEFAULT_SUBTITLE_STYLE);
  assert.equal(ass.includes("Script Info"), true);
  assert.equal(ass.includes("Dialogue:"), true);
});

test("Burn-in export lifecycle in mock environment", async () => {
  const mockTrack: SubtitleTrack = {
    id: "track-1",
    language: "en",
    displayName: "English",
    sourceFormat: "srt",
    cues: [{ id: "c1", startUs: 0, endUs: 2_000_000, text: "Sample" }],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const jobId = await startSubtitleBurnInExport({
    jobId: "test-job-1",
    sourceMediaPath: "/tmp/source.mp4",
    track: mockTrack,
    style: DEFAULT_SUBTITLE_STYLE,
    profile: {
      container: "mp4",
      resolution: { type: "original" },
      videoCodec: "libx264",
      crf: 23,
      preset: "medium",
      audioPolicy: "copy",
      selectedTrackId: "track-1",
      style: DEFAULT_SUBTITLE_STYLE,
    },
    stagingDir: "/tmp/staging",
  });

  assert.equal(jobId, "test-job-1");

  const status = await getSubtitleBurnInStatus();
  assert.equal(status.stage, "readyToSave");

  const finalized = await finalizeBurnInExport("/tmp/final.mp4", "/tmp/source.mp4");
  assert.equal(finalized, "/tmp/final.mp4");
});
