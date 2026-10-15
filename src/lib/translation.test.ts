import test from "node:test";
import assert from "node:assert/strict";
import {
  getTranslationCapabilities,
  getTranslationLanguagePairs,
  startOfflineTranslation,
  getOfflineTranslationStatus,
  cancelOfflineTranslation,
  getOfflineTranslationResult,
  applyTranslationResultToSubtitles,
} from "./translation";
import type { SubtitleCue, SubtitleDocument, SubtitleTrack } from "../types/subtitles";
import type { TranslationRequest, TranslationResult, TranslatedCue } from "../types/translation";

test("getTranslationLanguagePairs returns authoritative language pairs", async () => {
  const pairs = await getTranslationLanguagePairs();
  assert.ok(Array.isArray(pairs));
  assert.ok(pairs.length >= 2);

  const enEs = pairs.find((p) => p.sourceCode === "en" && p.targetCode === "es");
  assert.ok(enEs, "English to Spanish pair must exist");
  assert.equal(enEs?.isVerified, true);
  assert.ok(enEs?.compatibleModelIds.includes("qwen2.5-0.5b-instruct"));

  const enBn = pairs.find((p) => p.sourceCode === "en" && p.targetCode === "bn");
  assert.ok(enBn, "English to Bengali pair must exist");
  assert.equal(enBn?.isVerified, true); // Marked true in mock pairs, or check presence
  assert.ok(enBn?.compatibleModelIds.includes("qwen2.5-0.5b-instruct"));
});

test("getTranslationCapabilities returns runtime and pairs status", async () => {
  const caps = await getTranslationCapabilities();
  assert.ok(caps);
  assert.ok(Array.isArray(caps.supportedLanguagePairs));
  assert.equal(typeof caps.isAvailable, "boolean");
});

test("Offline translation job lifecycle with mock fallback in test env", async () => {
  const request: TranslationRequest = {
    documentId: "doc-123",
    trackId: "track-en",
    sourceLanguage: "en",
    targetLanguage: "es",
    modelId: "qwen2.5-0.5b-instruct",
  };

  const cues: SubtitleCue[] = [
    {
      id: "cue-1",
      startUs: 1_000_000,
      endUs: 3_500_000,
      text: "Hello world, this is a test.",
    },
    {
      id: "cue-2",
      startUs: 4_000_000,
      endUs: 6_000_000,
      text: "Welcome to offline media localization.",
    },
  ];

  const jobId = await startOfflineTranslation(request, cues);
  assert.ok(jobId.startsWith("mock-job-") || jobId.startsWith("trans-job-"));

  const status = await getOfflineTranslationStatus(jobId);
  assert.ok(status);
  assert.equal(status.stage, "completed");

  const result = await getOfflineTranslationResult(jobId);
  assert.ok(result);
  assert.equal(result.translatedCues.length, 2);
  assert.equal(result.originalTrackId, "track-en");
  assert.equal(result.targetLanguage, "es");

  // Invariant verification
  for (let i = 0; i < cues.length; i++) {
    const orig = cues[i];
    const trans: TranslatedCue | undefined = result.translatedCues[i];
    assert.ok(trans);
    assert.equal(trans.sourceCueId, orig.id);
    assert.equal(trans.targetStartUs, orig.startUs, "Exact timing preservation invariant: startUs");
    assert.equal(trans.targetEndUs, orig.endUs, "Exact timing preservation invariant: endUs");
    assert.ok(trans.translatedText.length > 0);
  }
});

test("Cancellation handles job gracefully", async () => {
  const request: TranslationRequest = {
    documentId: "doc-cancel",
    trackId: "track-cancel",
    sourceLanguage: "en",
    targetLanguage: "es",
    modelId: "qwen2.5-0.5b-instruct",
  };

  const cues: SubtitleCue[] = [
    { id: "c1", startUs: 1000, endUs: 2000, text: "Wait for it" },
  ];

  const jobId = await startOfflineTranslation(request, cues);
  await cancelOfflineTranslation(jobId);
  const status = await getOfflineTranslationStatus(jobId);
  assert.equal(status?.stage, "cancelled");
});

test("applyTranslationResultToSubtitles creates a new track and preserves original track & timings", async () => {
  const originalTrack: SubtitleTrack = {
    id: "track-original",
    language: "en",
    displayName: "English (Original)",
    sourceFormat: "nativeProject",
    cues: [
      {
        id: "cue-1",
        startUs: 1_000_000,
        endUs: 3_000_000,
        text: "Good morning everyone.",
        words: [
          { id: "w1", text: "Good", startUs: 1_000_000, endUs: 1_500_000, confidence: 0.99, provenance: "whisperNativeEstimate" },
          { id: "w2", text: "morning", startUs: 1_500_000, endUs: 2_200_000, confidence: 0.98, provenance: "whisperNativeEstimate" },
        ],
      },
      {
        id: "cue-2",
        startUs: 3_500_000,
        endUs: 5_000_000,
        text: "Let's begin the presentation.",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const initialDocument: SubtitleDocument = {
    schemaVersion: 1,
    id: "doc-1",
    tracks: [originalTrack],
    activeTrackId: "track-original",
    createdAtEpochMs: 1700000000000,
    updatedAtEpochMs: 1700000000000,
  };

  const translationResult: TranslationResult = {
    schemaVersion: 1,
    jobId: "job-test-apply",
    originalTrackId: "track-original",
    modelId: "qwen2.5-0.5b-instruct",
    runtimeVersion: "llama-cli v0.6.0-dev",
    sourceLanguage: "en",
    targetLanguage: "es",
    translatedCues: [
      {
        sourceCueId: "cue-1",
        translatedCueId: "cue-1-es",
        originalText: "Good morning everyone.",
        translatedText: "Buenos días a todos.",
        sourceStartUs: 1_000_000,
        sourceEndUs: 3_000_000,
        targetStartUs: 1_000_000,
        targetEndUs: 3_000_000,
        warnings: [],
        modelProvenance: "qwen2.5-0.5b-instruct",
      },
      {
        sourceCueId: "cue-2",
        translatedCueId: "cue-2-es",
        originalText: "Let's begin the presentation.",
        translatedText: "Comencemos la presentación.",
        sourceStartUs: 3_500_000,
        sourceEndUs: 5_000_000,
        targetStartUs: 3_500_000,
        targetEndUs: 5_000_000,
        warnings: [],
        modelProvenance: "qwen2.5-0.5b-instruct",
      },
    ],
    untranslatedCueIds: [],
    warnings: [],
    elapsedMs: 250,
  };

  const updatedDoc = await applyTranslationResultToSubtitles(translationResult, initialDocument);

  // Invariant 1: Original track is still present and unchanged
  assert.equal(updatedDoc.tracks.length, 2);
  const orig = updatedDoc.tracks.find((t) => t.id === "track-original");
  assert.ok(orig);
  assert.equal(orig?.cues.length, 2);
  assert.equal(orig?.cues[0].text, "Good morning everyone.");
  assert.ok(orig?.cues[0].words && orig.cues[0].words.length > 0, "Original words must be preserved");

  // Invariant 2: New track is created and set to active
  const translatedTrack = updatedDoc.tracks.find((t) => t.language === "es");
  assert.ok(translatedTrack);
  assert.equal(updatedDoc.activeTrackId, translatedTrack?.id);
  assert.equal(translatedTrack?.displayName, "Translated (ES)");
  assert.equal(translatedTrack?.cues.length, 2);

  // Invariant 3: Translated cues preserve timing exactly
  assert.equal(translatedTrack?.cues[0].startUs, 1_000_000);
  assert.equal(translatedTrack?.cues[0].endUs, 3_000_000);
  assert.equal(translatedTrack?.cues[0].text, "Buenos días a todos.");
  // Word alignment evidence from source language is not inherited on translated words
  assert.equal(translatedTrack?.cues[0].words, undefined);

  assert.equal(translatedTrack?.cues[1].startUs, 3_500_000);
  assert.equal(translatedTrack?.cues[1].endUs, 5_000_000);
  assert.equal(translatedTrack?.cues[1].text, "Comencemos la presentación.");
});
