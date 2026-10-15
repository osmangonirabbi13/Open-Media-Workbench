import test from "node:test";
import assert from "node:assert/strict";
import type { SubtitleDocument, SubtitleTrack } from "../types/subtitles";
import type { Glossary } from "../types/translationReview";
import {
  getOrCreateTranslationReview,
  updateReviewEntryTargetText,
  setReviewEntryStatus,
  batchSetReviewStatus,
  listGlossaries,
  saveGlossary,
  deleteGlossary,
  getTranslationMemory,
  recordApprovedTranslation,
  lookupTranslationMemory,
  deleteTranslationMemoryEntry,
  clearTranslationMemory,
  computeReviewStats,
} from "./translationReview";

test("Translation Review: Dual-track rendering and stable cue mapping", async () => {
  const sourceTrack: SubtitleTrack = {
    id: "track-src",
    displayName: "English (Source)",
    language: "en",
    sourceFormat: "nativeProject",
    cues: [
      {
        id: "cue-1",
        startUs: 1_000_000,
        endUs: 3_000_000,
        text: "Welcome to Open Media Workbench.",
      },
      {
        id: "cue-2",
        startUs: 4_000_000,
        endUs: 6_000_000,
        text: "Offline subtitle translation studio.",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const targetTrack: SubtitleTrack = {
    id: "track-tgt",
    displayName: "Spanish (Translated)",
    language: "es",
    sourceFormat: "nativeProject",
    cues: [
      {
        id: "cue-1-tr-es",
        startUs: 1_000_000,
        endUs: 3_000_000,
        text: "Bienvenido a Open Media Workbench.",
      },
      {
        id: "cue-2-tr-es",
        startUs: 4_000_000,
        endUs: 6_000_000,
        text: "Estudio de traducción de subtítulos sin conexión.",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const doc: SubtitleDocument = {
    id: "doc-1",
    schemaVersion: 1,
    tracks: [sourceTrack, targetTrack],
    createdAtEpochMs: Date.now(),
    updatedAtEpochMs: Date.now(),
    metadata: {},
  };

  const review = await getOrCreateTranslationReview(doc, "track-src", "track-tgt");
  assert.equal(review.entries.length, 2);
  assert.equal(review.entries[0].sourceCueId, "cue-1");
  assert.equal(review.entries[0].targetCueId, "cue-1-tr-es");
  assert.equal(review.entries[0].sourceText, "Welcome to Open Media Workbench.");
  assert.equal(review.entries[0].targetText, "Bienvenido a Open Media Workbench.");
  assert.equal(review.entries[0].status, "NeedsReview");
});

test("Translation Review: In-line editing invalidates Approved status to NeedsReview", async () => {
  const sourceTrack: SubtitleTrack = {
    id: "track-src",
    displayName: "English (Source)",
    language: "en",
    sourceFormat: "nativeProject",
    cues: [
      {
        id: "cue-1",
        startUs: 1_000_000,
        endUs: 3_000_000,
        text: "Hello world.",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const targetTrack: SubtitleTrack = {
    id: "track-tgt",
    displayName: "Spanish",
    language: "es",
    sourceFormat: "nativeProject",
    cues: [
      {
        id: "cue-1-tr-es",
        startUs: 1_000_000,
        endUs: 3_000_000,
        text: "Hola mundo.",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const doc: SubtitleDocument = {
    id: "doc-1",
    schemaVersion: 1,
    tracks: [sourceTrack, targetTrack],
    createdAtEpochMs: Date.now(),
    updatedAtEpochMs: Date.now(),
    metadata: {},
  };

  let review = await getOrCreateTranslationReview(doc, "track-src", "track-tgt");
  const entryId = review.entries[0].id;

  // 1. Mark as Approved
  review = await setReviewEntryStatus(review, entryId, "Approved");
  assert.equal(review.entries[0].status, "Approved");

  // 2. User edits translated text
  const [updatedDoc, updatedReview] = await updateReviewEntryTargetText(
    doc,
    review,
    "track-tgt",
    entryId,
    "Hola a todo el mundo.",
  );

  // Invariant: Status must reset to NeedsReview
  assert.equal(updatedReview.entries[0].status, "NeedsReview");
  assert.equal(updatedReview.entries[0].targetText, "Hola a todo el mundo.");

  // Target subtitle track cue must be updated in document
  const cueInDoc = updatedDoc.tracks.find((t) => t.id === "track-tgt")?.cues[0];
  assert.equal(cueInDoc?.text, "Hola a todo el mundo.");
});

test("Translation Review: Batch review status updates", async () => {
  const sourceTrack: SubtitleTrack = {
    id: "src",
    displayName: "Src",
    language: "en",
    sourceFormat: "nativeProject",
    cues: [
      { id: "c1", startUs: 1000, endUs: 2000, text: "One" },
      { id: "c2", startUs: 3000, endUs: 4000, text: "Two" },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const targetTrack: SubtitleTrack = {
    id: "tgt",
    displayName: "Tgt",
    language: "es",
    sourceFormat: "nativeProject",
    cues: [
      { id: "c1-tr-es", startUs: 1000, endUs: 2000, text: "Uno" },
      { id: "c2-tr-es", startUs: 3000, endUs: 4000, text: "Dos" },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const doc: SubtitleDocument = {
    id: "doc-batch",
    schemaVersion: 1,
    tracks: [sourceTrack, targetTrack],
    createdAtEpochMs: Date.now(),
    updatedAtEpochMs: Date.now(),
    metadata: {},
  };

  let review = await getOrCreateTranslationReview(doc, "src", "tgt");
  const allIds = review.entries.map((e) => e.id);

  // Batch mark Reviewed
  review = await batchSetReviewStatus(review, allIds, "Reviewed");
  assert.equal(review.entries[0].status, "Reviewed");
  assert.equal(review.entries[1].status, "Reviewed");

  // Batch mark Approved
  review = await batchSetReviewStatus(review, allIds, "Approved");
  assert.equal(review.entries[0].status, "Approved");
  assert.equal(review.entries[1].status, "Approved");
});

test("Translation Review: Progress statistics computation", async () => {
  const doc = {
    schemaVersion: 1,
    reviewId: "r1",
    documentId: "d1",
    sourceTrackId: "s",
    targetTrackId: "t",
    sourceLanguage: "en",
    targetLanguage: "es",
    sourceRevision: "1",
    targetRevision: "1",
    referencedGlossaryIds: [],
    createdAtEpochMs: 0,
    updatedAtEpochMs: 0,
    entries: [
      {
        id: "e1",
        sourceCueId: "c1",
        targetCueId: "t1",
        sourceText: "A",
        targetText: "B",
        sourceStartUs: 0,
        sourceEndUs: 1000,
        targetStartUs: 0,
        targetEndUs: 1000,
        status: "Approved" as const,
        findings: [],
        createdAtEpochMs: 0,
        updatedAtEpochMs: 0,
      },
      {
        id: "e2",
        sourceCueId: "c2",
        targetCueId: "t2",
        sourceText: "C",
        targetText: "D",
        sourceStartUs: 2000,
        sourceEndUs: 3000,
        targetStartUs: 2000,
        targetEndUs: 3000,
        status: "NeedsReview" as const,
        findings: [{
          code: "ProtectedTermChanged" as const,
          severity: "Error" as const,
          message: "Protected term modified",
        }],
        createdAtEpochMs: 0,
        updatedAtEpochMs: 0,
      },
    ],
  };

  const stats = computeReviewStats(doc);
  assert.equal(stats.totalMappedCues, 2);
  assert.equal(stats.approvedCount, 1);
  assert.equal(stats.needsReviewCount, 1);
  assert.equal(stats.cuesWithErrorsCount, 1);
  assert.equal(stats.approvedPercentage, 50);
});

test("Glossary Management: Lifecycle CRUD", async () => {
  const glossary: Glossary = {
    schemaVersion: 1,
    id: "glos-test-1",
    name: "Brand Terms",
    sourceLanguage: "en",
    targetLanguage: "es",
    isGlobal: true,
    entries: [
      {
        id: "entry-1",
        sourceTerm: "Open Media Workbench",
        preferredTerm: "Open Media Workbench",
        isProtected: true,
        caseSensitive: false,
        createdAtEpochMs: Date.now(),
        updatedAtEpochMs: Date.now(),
      },
    ],
    createdAtEpochMs: Date.now(),
    updatedAtEpochMs: Date.now(),
  };

  await saveGlossary(glossary);
  const list = await listGlossaries();
  assert(list.some((g) => g.id === "glos-test-1"));

  await deleteGlossary("glos-test-1");
  const listAfter = await listGlossaries();
  assert(!listAfter.some((g) => g.id === "glos-test-1"));
});

test("Translation Memory: Record, lookup, and deletion", async () => {
  await clearTranslationMemory();

  // Record approved translation
  const entry = await recordApprovedTranslation(
    "Open source media workbench.",
    "Mesa de trabajo de medios de código abierto.",
    "en",
    "es",
  );

  assert.equal(entry.sourceTextNormalized, "open source media workbench.");
  assert.equal(entry.targetText, "Mesa de trabajo de medios de código abierto.");

  const allMem = await getTranslationMemory();
  assert.equal(allMem.entries.length, 1);

  // Lookup
  const match = await lookupTranslationMemory("  Open   Source Media Workbench. ", "en", "es");
  assert(match !== null);
  assert.equal(match.targetText, "Mesa de trabajo de medios de código abierto.");

  // Delete
  await deleteTranslationMemoryEntry(entry.id);
  const matchAfter = await lookupTranslationMemory("Open source media workbench.", "en", "es");
  assert.equal(matchAfter, null);
});

test("Translation Review: Bengali & Unicode text editing preservation", async () => {
  const sourceTrack: SubtitleTrack = {
    id: "src-bn",
    displayName: "English",
    language: "en",
    sourceFormat: "nativeProject",
    cues: [
      {
        id: "c-bn-1",
        startUs: 1_000_000,
        endUs: 4_000_000,
        text: "Language and culture are rich.",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const targetTrack: SubtitleTrack = {
    id: "tgt-bn",
    displayName: "Bengali",
    language: "bn",
    sourceFormat: "nativeProject",
    cues: [
      {
        id: "c-bn-1-tr-bn",
        startUs: 1_000_000,
        endUs: 4_000_000,
        text: "ভাষা ও সংস্কৃতি অত্যন্ত সমৃদ্ধ।",
      },
    ],
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const doc: SubtitleDocument = {
    id: "doc-bn",
    schemaVersion: 1,
    tracks: [sourceTrack, targetTrack],
    createdAtEpochMs: Date.now(),
    updatedAtEpochMs: Date.now(),
    metadata: {},
  };

  let review = await getOrCreateTranslationReview(doc, "src-bn", "tgt-bn");
  assert.equal(review.entries[0].targetText, "ভাষা ও সংস্কৃতি অত্যন্ত সমৃদ্ধ।");

  // Human edits Bengali text with complex grapheme clusters
  const updatedBengali = "আমাদের বাংলা ভাষা ও সংস্কৃতি অত্যন্ত সমৃদ্ধ।";
  const [updatedDoc, updatedReview] = await updateReviewEntryTargetText(
    doc,
    review,
    "tgt-bn",
    review.entries[0].id,
    updatedBengali,
  );

  assert.equal(updatedReview.entries[0].targetText, updatedBengali);
  const trackInDoc = updatedDoc.tracks.find((t) => t.id === "tgt-bn");
  assert.equal(trackInDoc?.cues[0].text, updatedBengali);
});
