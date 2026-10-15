import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  getTtsRuntimeCapabilities,
  listAvailableTtsVoices,
  getTtsVoiceStatus,
  startOfflineTts,
  getOfflineTtsStatus,
  getOfflineTtsResult,
  cancelOfflineTts,
  exportTtsAudio,
} from "./tts";
import type { SubtitleTrack } from "../types/subtitles";

describe("TTS IPC Client & Mock Lifecycle", () => {
  it("getTtsRuntimeCapabilities returns capabilities report", async () => {
    const caps = await getTtsRuntimeCapabilities();
    assert.strictEqual(caps.discovered, true);
    assert.ok(caps.version?.includes("Piper"));
  });

  it("listAvailableTtsVoices returns curated voices including Lessac", async () => {
    const voices = await listAvailableTtsVoices();
    assert.ok(voices.length >= 1);
    const lessac = voices.find((v) => v.id === "piper-en-lessac-medium");
    assert.ok(lessac);
    assert.strictEqual(lessac.language, "en-US");
    assert.strictEqual(lessac.sampleRate, 22050);
  });

  it("getTtsVoiceStatus returns voice by ID", async () => {
    const voice = await getTtsVoiceStatus("piper-en-lessac-medium");
    assert.strictEqual(voice.id, "piper-en-lessac-medium");
    assert.strictEqual(voice.voiceFamily, "piper");
  });

  it("synthesizes script text to multiple segments", async () => {
    const jobId = await startOfflineTts({
      source: {
        type: "script",
        text: "Sentence one. Sentence two! Sentence three?",
      },
      voiceId: "piper-en-lessac-medium",
      language: "en-US",
      settings: {
        speedScale: 1.0,
        noiseScale: 0.667,
        noiseW: 0.8,
      },
    });

    assert.ok(jobId.startsWith("mock_tts_"));

    const status = await getOfflineTtsStatus(jobId);
    assert.strictEqual(status.stage, "completed");
    assert.strictEqual(status.completedSegments, 3);

    const result = await getOfflineTtsResult(jobId);
    assert.strictEqual(result.segments.length, 3);
    assert.strictEqual(result.segments[0].originalText, "Sentence one");
    assert.strictEqual(result.segments[1].originalText, "Sentence two");
    assert.strictEqual(result.segments[2].originalText, "Sentence three");
  });

  it("synthesizes subtitle cues and measures timing differences", async () => {
    const mockTrack: SubtitleTrack = {
      id: "track_test",
      language: "en",
      displayName: "English",
      sourceFormat: "srt",
      cues: [
        {
          id: "cue_1",
          startUs: 1000000,
          endUs: 2000000, // 1 second
          text: "This is a longer cue that takes more than 1 second to speak out loud.",
        },
      ],
      isEnabled: true,
      isVisible: true,
      schemaVersion: 1,
    };

    const jobId = await startOfflineTts(
      {
        source: {
          type: "subtitleTrack",
          trackId: "track_test",
        },
        voiceId: "piper-en-lessac-medium",
        language: "en-US",
        settings: {
          speedScale: 1.0,
          noiseScale: 0.667,
          noiseW: 0.8,
        },
      },
      mockTrack
    );

    const result = await getOfflineTtsResult(jobId);
    assert.strictEqual(result.segments.length, 1);
    const seg = result.segments[0];
    assert.strictEqual(seg.sourceCueId, "cue_1");
    assert.strictEqual(seg.cueStartUs, 1000000);
    assert.strictEqual(seg.cueEndUs, 2000000);
    assert.ok(seg.durationUs > 0);
    assert.ok(seg.timingDifferenceUs !== undefined);
  });

  it("cancels active job and exports WAV", async () => {
    const jobId = await startOfflineTts({
      source: {
        type: "script",
        text: "Quick test.",
      },
      voiceId: "piper-en-lessac-medium",
      language: "en-US",
      settings: {
        speedScale: 1.0,
        noiseScale: 0.667,
        noiseW: 0.8,
      },
    });

    const cancelled = await cancelOfflineTts(jobId);
    assert.strictEqual(cancelled, true);

    const exportPath = await exportTtsAudio(jobId, "/tmp/test.wav");
    assert.strictEqual(exportPath, "/tmp/test.wav");
  });
});
