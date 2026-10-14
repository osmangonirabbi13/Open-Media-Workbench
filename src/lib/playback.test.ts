import { test } from "node:test";
import assert from "node:assert/strict";
import {
  formatTimecode,
  formatFrames,
  getMediaPlaybackUrl,
  requestPreviewProxy,
  cancelPreviewProxy,
  getProxyStatus,
} from "./playback";

test("formatTimecode: formats fractional seconds to SMPTE timecode (HH:MM:SS:FF)", () => {
  assert.equal(formatTimecode(0, 30), "00:00:00:00");
  assert.equal(formatTimecode(1.5, 30), "00:00:01:15");
  assert.equal(formatTimecode(65.0, 30), "00:01:05:00");
  assert.equal(formatTimecode(3661.0, 30), "01:01:01:00");
  assert.equal(formatTimecode(-5, 30), "00:00:00:00");
  assert.equal(formatTimecode(NaN, 30), "00:00:00:00");
  assert.equal(formatTimecode(Infinity, 30), "00:00:00:00");
});

test("formatTimecode: handles different frame rates accurately", () => {
  // 24 fps
  assert.equal(formatTimecode(1.0, 24), "00:00:01:00");
  assert.equal(formatTimecode(1.5, 24), "00:00:01:12");

  // 25 fps
  assert.equal(formatTimecode(1.4, 25), "00:00:01:10");

  // 60 fps
  assert.equal(formatTimecode(0.5, 60), "00:00:00:30");
});

test("formatFrames: formats fractional seconds into padded frame count", () => {
  assert.equal(formatFrames(0, 30), "00000 f");
  assert.equal(formatFrames(1, 30), "00030 f");
  assert.equal(formatFrames(2.5, 30), "00075 f");
  assert.equal(formatFrames(-10, 30), "00000 f");
});

test("playback IPC helpers: reject safely when executed outside Tauri runtime", async () => {
  await assert.rejects(
    async () => {
      await getMediaPlaybackUrl("test-asset-1");
    },
    /outside desktop shell/
  );

  await assert.rejects(
    async () => {
      await requestPreviewProxy("test-asset-1");
    },
    /outside desktop shell/
  );

  await assert.rejects(
    async () => {
      await cancelPreviewProxy("test-asset-1");
    },
    /outside desktop shell/
  );

  await assert.rejects(
    async () => {
      await getProxyStatus("test-asset-1");
    },
    /outside desktop shell/
  );
});
