import { test } from "node:test";
import assert from "node:assert/strict";
import {
  getWaveformStatus,
  requestWaveformAnalysis,
  cancelWaveformAnalysis,
  getWaveformSegment,
} from "./waveform";

test("waveform IPC helpers: reject gracefully when executed outside Tauri desktop runtime", async () => {
  await assert.rejects(
    async () => {
      await getWaveformStatus("test-asset-1");
    },
    /outside desktop shell/
  );

  await assert.rejects(
    async () => {
      await requestWaveformAnalysis("test-asset-1");
    },
    /outside desktop shell/
  );

  await assert.rejects(
    async () => {
      await cancelWaveformAnalysis("test-asset-1");
    },
    /outside desktop shell/
  );

  await assert.rejects(
    async () => {
      await getWaveformSegment("test-asset-1", 0, 1_000_000, 800);
    },
    /outside desktop shell/
  );
});
