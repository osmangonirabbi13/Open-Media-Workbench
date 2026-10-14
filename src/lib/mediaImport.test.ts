import test from "node:test";
import assert from "node:assert/strict";
import {
  formatDurationMicros,
  formatFileSize,
  formatFps,
} from "../types/media";
import {
  pickAndImportFiles,
  importFilesByPath,
  getMediaLibraryAssets,
  removeMediaLibraryAsset,
  clearMediaLibrary,
} from "./mediaImport";

test("formatDurationMicros: formats microseconds accurately", () => {
  assert.strictEqual(formatDurationMicros(undefined), "--:--");
  assert.strictEqual(formatDurationMicros(0), "00:00");
  assert.strictEqual(formatDurationMicros(15_000_000), "00:15");
  assert.strictEqual(formatDurationMicros(75_000_000), "01:15");
  assert.strictEqual(formatDurationMicros(3_665_000_000), "01:01:05");
});

test("formatFileSize: formats bytes into human-readable strings", () => {
  assert.strictEqual(formatFileSize(0), "0 B");
  assert.strictEqual(formatFileSize(1024), "1 KB");
  assert.strictEqual(formatFileSize(1024 * 1024 * 5.5), "5.5 MB");
  assert.strictEqual(formatFileSize(1024 * 1024 * 1024 * 2.1), "2.1 GB");
});

test("formatFps: formats rational frame rate correctly", () => {
  assert.strictEqual(formatFps(undefined), "Unknown fps");
  assert.strictEqual(formatFps({ num: 0, den: 0 }), "Unknown fps");
  assert.strictEqual(formatFps({ num: 30, den: 1 }), "30 fps");
  assert.strictEqual(formatFps({ num: 25, den: 1 }), "25 fps");
  assert.strictEqual(formatFps({ num: 30000, den: 1001 }), "29.97 fps");
  assert.strictEqual(formatFps({ num: 60000, den: 1001 }), "59.94 fps");
});

test("mediaImport: handles browser context outside Tauri gracefully", async () => {
  await assert.rejects(
    async () => {
      await pickAndImportFiles();
    },
    {
      message: /unavailable outside the OpenTake Tauri desktop environment/,
    }
  );

  await assert.rejects(
    async () => {
      await importFilesByPath(["/tmp/test.mp4"]);
    },
    {
      message: /unavailable outside the OpenTake Tauri desktop environment/,
    }
  );

  const assets = await getMediaLibraryAssets();
  assert.deepStrictEqual(assets, []);

  const removed = await removeMediaLibraryAsset("any-id");
  assert.strictEqual(removed, false);

  await clearMediaLibrary();
});
