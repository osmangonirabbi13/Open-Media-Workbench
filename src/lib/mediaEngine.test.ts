import test from "node:test";
import assert from "node:assert/strict";
import { getMediaEngineStatus } from "./mediaEngine";

test("getMediaEngineStatus: throws safe error in browser context outside Tauri", async () => {
  await assert.rejects(
    async () => {
      await getMediaEngineStatus(false);
    },
    {
      message: /Media Engine IPC unavailable: OpenTake is running in a browser window/,
    }
  );
});
