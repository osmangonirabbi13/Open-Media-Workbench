import { invoke } from "@tauri-apps/api/core";
import type { SubtitleDocument } from "../types/subtitles";
import type {
  Glossary,
  ReviewProgressStats,
  ReviewStatus,
  TranslationMemory,
  TranslationMemoryEntry,
  TranslationReviewDocument,
  TranslationReviewEntry,
} from "../types/translationReview";

function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// In-memory mock storage for browser/test environments
const mockGlossaries: Map<string, Glossary> = new Map();
let mockMemory: TranslationMemory = {
  schemaVersion: 1,
  entries: [],
  updatedAtEpochMs: Date.now(),
};

export async function getOrCreateTranslationReview(
  document: SubtitleDocument,
  sourceTrackId: string,
  targetTrackId: string,
  glossaryId?: string,
): Promise<TranslationReviewDocument> {
  if (isTauriEnvironment()) {
    return invoke<TranslationReviewDocument>("get_or_create_translation_review", {
      document,
      sourceTrackId,
      targetTrackId,
      glossaryId: glossaryId || null,
    });
  }

  // Browser/Mock fallback
  const sourceTrack = document.tracks.find((t) => t.id === sourceTrackId);
  const targetTrack = document.tracks.find((t) => t.id === targetTrackId);
  const now = Date.now();

  const entries: TranslationReviewEntry[] = (sourceTrack?.cues || []).map((sc, idx) => {
    const tc = targetTrack?.cues.find(
      (c) => c.id.startsWith(`${sc.id}-tr-`) || c.id === sc.id || Math.abs(c.startUs - sc.startUs) < 100000,
    ) || targetTrack?.cues[idx];

    return {
      id: `rev-${sc.id}`,
      sourceCueId: sc.id,
      targetCueId: tc ? tc.id : "",
      sourceText: sc.text,
      targetText: tc ? tc.text : "",
      sourceStartUs: sc.startUs,
      sourceEndUs: sc.endUs,
      targetStartUs: tc ? tc.startUs : sc.startUs,
      targetEndUs: tc ? tc.endUs : sc.endUs,
      status: "NeedsReview",
      findings: !tc ? [{
        code: "UnmappedCue",
        severity: "Error",
        message: "No matching target subtitle cue found.",
      }] : [],
      createdAtEpochMs: now,
      updatedAtEpochMs: now,
    };
  });

  return {
    schemaVersion: 1,
    reviewId: `rev-doc-${now}`,
    documentId: document.id,
    sourceTrackId,
    targetTrackId,
    sourceLanguage: sourceTrack?.language || "en",
    targetLanguage: targetTrack?.language || "es",
    sourceRevision: "rev-1",
    targetRevision: "rev-1",
    entries,
    referencedGlossaryIds: glossaryId ? [glossaryId] : [],
    createdAtEpochMs: now,
    updatedAtEpochMs: now,
  };
}

export async function saveTranslationReviewToDocument(
  document: SubtitleDocument,
  review: TranslationReviewDocument,
): Promise<SubtitleDocument> {
  if (isTauriEnvironment()) {
    return invoke<SubtitleDocument>("save_translation_review_to_document", {
      document,
      review,
    });
  }

  return {
    ...document,
    metadata: {
      ...document.metadata,
      translationReview: JSON.stringify(review),
    },
    updatedAtEpochMs: Date.now(),
  };
}

export async function updateReviewEntryTargetText(
  document: SubtitleDocument,
  review: TranslationReviewDocument,
  targetTrackId: string,
  entryId: string,
  newText: string,
  glossaryId?: string,
): Promise<[SubtitleDocument, TranslationReviewDocument]> {
  if (isTauriEnvironment()) {
    const res = await invoke<[SubtitleDocument, TranslationReviewDocument]>(
      "update_review_entry_target_text",
      {
        document,
        review,
        targetTrackId,
        entryId,
        newText,
        glossaryId: glossaryId || null,
      },
    );
    return res;
  }

  // Browser fallback: update document track cue and review entry
  const now = Date.now();
  const updatedTracks = document.tracks.map((t) => {
    if (t.id !== targetTrackId) return t;
    return {
      ...t,
      cues: t.cues.map((c) => {
        const matchingEntry = review.entries.find((e) => e.id === entryId);
        if (matchingEntry && c.id === matchingEntry.targetCueId) {
          return { ...c, text: newText };
        }
        return c;
      }),
    };
  });

  const updatedEntries = review.entries.map((e) => {
    if (e.id !== entryId) return e;
    const prevStatus = e.status;
    return {
      ...e,
      targetText: newText,
      // Approval invalidation invariant: any text change invalidates Approved status
      status: prevStatus === "Approved" ? ("NeedsReview" as ReviewStatus) : prevStatus,
      approvedFingerprint: undefined,
      updatedAtEpochMs: now,
    };
  });

  const updatedReview: TranslationReviewDocument = {
    ...review,
    entries: updatedEntries,
    updatedAtEpochMs: now,
  };

  const updatedDoc: SubtitleDocument = {
    ...document,
    tracks: updatedTracks,
    metadata: {
      ...document.metadata,
      translationReview: JSON.stringify(updatedReview),
    },
    updatedAtEpochMs: now,
  };

  return [updatedDoc, updatedReview];
}

export async function setReviewEntryStatus(
  review: TranslationReviewDocument,
  entryId: string,
  status: ReviewStatus,
  allowOverride = false,
): Promise<TranslationReviewDocument> {
  if (isTauriEnvironment()) {
    return invoke<TranslationReviewDocument>("set_review_entry_status_command", {
      review,
      entryId,
      status,
      allowOverride,
    });
  }

  const now = Date.now();
  const updatedEntries = review.entries.map((e) => {
    if (e.id !== entryId) return e;
    return {
      ...e,
      status,
      approvedAtEpochMs: status === "Approved" ? now : undefined,
      updatedAtEpochMs: now,
    };
  });

  return {
    ...review,
    entries: updatedEntries,
    updatedAtEpochMs: now,
  };
}

export async function batchSetReviewStatus(
  review: TranslationReviewDocument,
  entryIds: string[],
  status: ReviewStatus,
  allowOverride = false,
): Promise<TranslationReviewDocument> {
  if (isTauriEnvironment()) {
    return invoke<TranslationReviewDocument>("batch_set_review_status_command", {
      review,
      entryIds,
      status,
      allowOverride,
    });
  }

  const idSet = new Set(entryIds);
  const now = Date.now();
  const updatedEntries = review.entries.map((e) => {
    if (!idSet.has(e.id)) return e;
    return {
      ...e,
      status,
      approvedAtEpochMs: status === "Approved" ? now : undefined,
      updatedAtEpochMs: now,
    };
  });

  return {
    ...review,
    entries: updatedEntries,
    updatedAtEpochMs: now,
  };
}

// =========================================================================
// GLOSSARY MANAGEMENT
// =========================================================================

export async function listGlossaries(): Promise<Glossary[]> {
  if (isTauriEnvironment()) {
    return invoke<Glossary[]>("list_glossaries_command");
  }
  return Array.from(mockGlossaries.values());
}

export async function saveGlossary(glossary: Glossary): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("save_glossary_command", { glossary });
  }
  mockGlossaries.set(glossary.id, { ...glossary, updatedAtEpochMs: Date.now() });
}

export async function deleteGlossary(glossaryId: string): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("delete_glossary_command", { glossaryId });
  }
  mockGlossaries.delete(glossaryId);
}

// =========================================================================
// TRANSLATION MEMORY
// =========================================================================

export async function getTranslationMemory(): Promise<TranslationMemory> {
  if (isTauriEnvironment()) {
    return invoke<TranslationMemory>("get_translation_memory_command");
  }
  return mockMemory;
}

function normalizeText(text: string): string {
  return text.trim().split(/\s+/).join(" ").toLowerCase();
}

export async function recordApprovedTranslation(
  sourceText: string,
  targetText: string,
  sourceLanguage: string,
  targetLanguage: string,
  domain?: string,
): Promise<TranslationMemoryEntry> {
  if (isTauriEnvironment()) {
    return invoke<TranslationMemoryEntry>("record_approved_translation_command", {
      sourceText,
      targetText,
      sourceLanguage,
      targetLanguage,
      domain: domain || null,
    });
  }

  const norm = normalizeText(sourceText);
  const now = Date.now();
  const existingIdx = mockMemory.entries.findIndex(
    (e) =>
      e.sourceTextNormalized === norm &&
      e.sourceLanguage.toLowerCase() === sourceLanguage.toLowerCase() &&
      e.targetLanguage.toLowerCase() === targetLanguage.toLowerCase(),
  );

  let entry: TranslationMemoryEntry;
  if (existingIdx >= 0) {
    entry = {
      ...mockMemory.entries[existingIdx],
      targetText: targetText.trim(),
      usageCount: mockMemory.entries[existingIdx].usageCount + 1,
      updatedAtEpochMs: now,
      domain: domain || mockMemory.entries[existingIdx].domain,
    };
    mockMemory.entries[existingIdx] = entry;
  } else {
    entry = {
      id: `tm-${now}-${Math.random().toString(36).substring(2, 7)}`,
      sourceTextNormalized: norm,
      targetText: targetText.trim(),
      sourceLanguage: sourceLanguage.trim().toLowerCase(),
      targetLanguage: targetLanguage.trim().toLowerCase(),
      domain,
      usageCount: 1,
      createdAtEpochMs: now,
      updatedAtEpochMs: now,
    };
    mockMemory.entries.push(entry);
  }

  mockMemory.updatedAtEpochMs = now;
  return entry;
}

export async function lookupTranslationMemory(
  sourceText: string,
  sourceLanguage: string,
  targetLanguage: string,
): Promise<TranslationMemoryEntry | null> {
  if (isTauriEnvironment()) {
    return invoke<TranslationMemoryEntry | null>("lookup_translation_memory_command", {
      sourceText,
      sourceLanguage,
      targetLanguage,
    });
  }

  const norm = normalizeText(sourceText);
  const found = mockMemory.entries.find(
    (e) =>
      e.sourceTextNormalized === norm &&
      e.sourceLanguage.toLowerCase() === sourceLanguage.toLowerCase() &&
      e.targetLanguage.toLowerCase() === targetLanguage.toLowerCase(),
  );
  return found || null;
}

export async function deleteTranslationMemoryEntry(entryId: string): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("delete_translation_memory_entry_command", { entryId });
  }
  mockMemory.entries = mockMemory.entries.filter((e) => e.id !== entryId);
  mockMemory.updatedAtEpochMs = Date.now();
}

export async function clearTranslationMemory(): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("clear_translation_memory_command");
  }
  mockMemory.entries = [];
  mockMemory.updatedAtEpochMs = Date.now();
}

// =========================================================================
// HELPER: REVIEW STATISTICS COMPUTATION
// =========================================================================

export function computeReviewStats(doc: TranslationReviewDocument): ReviewProgressStats {
  const total = doc.entries.length;
  let pending = 0;
  let needsReview = 0;
  let reviewed = 0;
  let approved = 0;
  let rejected = 0;
  let withWarnings = 0;
  let withErrors = 0;
  let unmapped = 0;

  for (const entry of doc.entries) {
    switch (entry.status) {
      case "Pending":
        pending += 1;
        break;
      case "NeedsReview":
        needsReview += 1;
        break;
      case "Reviewed":
        reviewed += 1;
        break;
      case "Approved":
        approved += 1;
        break;
      case "Rejected":
        rejected += 1;
        break;
    }

    if (!entry.targetCueId) {
      unmapped += 1;
    }

    if (entry.findings.some((f) => f.severity === "Error")) {
      withErrors += 1;
    } else if (entry.findings.some((f) => f.severity === "Warning")) {
      withWarnings += 1;
    }
  }

  const reviewedPercentage = total > 0 ? ((reviewed + approved) / total) * 100 : 0;
  const approvedPercentage = total > 0 ? (approved / total) * 100 : 0;

  return {
    totalMappedCues: total - unmapped,
    pendingCount: pending,
    needsReviewCount: needsReview,
    reviewedCount: reviewed,
    approvedCount: approved,
    rejectedCount: rejected,
    cuesWithWarningsCount: withWarnings,
    cuesWithErrorsCount: withErrors,
    unmappedCuesCount: unmapped,
    reviewedPercentage: Math.round(reviewedPercentage * 10) / 10,
    approvedPercentage: Math.round(approvedPercentage * 10) / 10,
  };
}
