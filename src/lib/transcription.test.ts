import { test } from "node:test";
import assert from "node:assert/strict";
import {
  getWhisperRuntimeCapabilities,
  startOfflineTranscription,
  getOfflineTranscriptionStatus,
  cancelOfflineTranscription,
  getOfflineTranscriptionResult,
  applyTranscriptionResultToSubtitles,
} from "./transcription";
import type { SubtitleDocument } from "../types/subtitles";

test("Transcription: getWhisperRuntimeCapabilities returns structured status", async () => {
  const caps = await getWhisperRuntimeCapabilities();
  assert.equal(typeof caps.isAvailable, "boolean");
  assert.equal(typeof caps.runtimePath, "string");
  assert.ok(caps.runtimeVersion && caps.runtimeVersion.length > 0);
  assert.ok(Array.isArray(caps.supportedLanguages));
  const codes = caps.supportedLanguages.map((l) => l.code);
  assert.ok(codes.includes("en"));
  assert.ok(codes.includes("bn"));
});

test("Transcription: job lifecycle from start to result in mock environment", async () => {
  const jobId = await startOfflineTranscription({
    assetId: "asset-123",
    modelId: "whisper-tiny",
    language: "en",
    outputTrackName: "English AI Subtitles",
  });

  assert.ok(jobId.startsWith("mock-job-"));

  const status = await getOfflineTranscriptionStatus(jobId);
  assert.ok(status);
  assert.equal(status.jobId, jobId);
  assert.equal(status.stage, "completed");
  assert.equal(status.progressPercent, 100);

  const result = await getOfflineTranscriptionResult(jobId);
  assert.equal(result.jobId, jobId);
  assert.equal(result.assetId, "asset-123");
  assert.equal(result.modelId, "whisper-tiny");
  assert.ok(result.segments.length > 0);

  const firstSeg = result.segments[0];
  assert.ok(firstSeg.startUs >= 0);
  assert.ok(firstSeg.endUs > firstSeg.startUs);
  assert.ok(firstSeg.text.length > 0);
});

test("Transcription: cancellation halts job in mock environment", async () => {
  const jobId = await startOfflineTranscription({
    assetId: "asset-cancel-test",
    modelId: "whisper-tiny",
    language: "auto",
  });

  await cancelOfflineTranscription(jobId);

  const status = await getOfflineTranscriptionStatus(jobId);
  assert.ok(status);
  assert.equal(status.stage, "cancelled");
});

test("Transcription: applying result creates new document when none exists", async () => {
  const jobId = await startOfflineTranscription({
    assetId: "asset-new-doc",
    modelId: "whisper-tiny",
    language: "en",
    outputTrackName: "Primary Subtitles",
  });

  const newDoc = await applyTranscriptionResultToSubtitles(jobId, "Primary Subtitles", null);
  assert.ok(newDoc.id.startsWith("doc-"));
  assert.equal(newDoc.tracks.length, 1);
  assert.equal(newDoc.tracks[0].displayName, "Primary Subtitles");
  assert.equal(newDoc.tracks[0].cues.length, 2);
  assert.equal(newDoc.tracks[0].cues[0].text, "Welcome to Open Media Workbench.");
  assert.equal(newDoc.tracks[0].cues[0].customMetadata?.confidence, "0.980");
});

test("Transcription: applying result appends track non-destructively to existing document", async () => {
  const jobId = await startOfflineTranscription({
    assetId: "asset-existing-doc",
    modelId: "whisper-tiny",
    language: "bn",
    outputTrackName: "Bengali Track",
  });

  const existingDoc: SubtitleDocument = {
    schemaVersion: 1,
    id: "doc-manual-1",
    associatedMediaAssetId: "asset-existing-doc",
    tracks: [
      {
        id: "track-manual-en",
        language: "en",
        displayName: "Manual English",
        sourceFormat: "srt",
        cues: [
          {
            id: "cue-1",
            startUs: 0,
            endUs: 1_000_000,
            text: "Hello world",
          },
        ],
        isEnabled: true,
        isVisible: true,
        schemaVersion: 1,
      },
    ],
    createdAtEpochMs: 1000,
    updatedAtEpochMs: 1000,
  };

  const updatedDoc = await applyTranscriptionResultToSubtitles(jobId, "AI Bengali", existingDoc);
  assert.equal(updatedDoc.id, "doc-manual-1");
  assert.equal(updatedDoc.tracks.length, 2, "Must preserve existing track and append new track");
  assert.equal(updatedDoc.tracks[0].displayName, "Manual English", "Existing track must remain unchanged");
  assert.equal(updatedDoc.tracks[1].displayName, "AI Bengali", "New track must be appended");
  assert.ok(updatedDoc.updatedAtEpochMs > 1000, "Timestamp must be updated");
});
