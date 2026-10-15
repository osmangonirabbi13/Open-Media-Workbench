//! Client bridge and mock fallbacks for Word-level Timestamp Alignment and Smart Segmentation.

import { invoke } from "@tauri-apps/api/core";
import type { SubtitleCue, SubtitleDocument, SubtitleTrack } from "../types/subtitles";
import type {
  AlignedWord,
  AlignmentResult,
  SegmentationPolicy,
} from "../types/alignment";
import { DEFAULT_SEGMENTATION_POLICY } from "../types/alignment";

function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function getWordAlignmentResult(jobId: string): Promise<AlignmentResult> {
  if (isTauriEnvironment()) {
    return invoke<AlignmentResult>("get_word_alignment_result", { jobId });
  }

  return {
    schemaVersion: 1,
    sourceAssetId: "mock-asset",
    jobId,
    modelId: "whisper-tiny",
    language: "en",
    timingQuality: "aligned",
    words: [
      {
        id: "w-0-0",
        text: "Welcome",
        startUs: 0,
        endUs: 600_000,
        provenance: "whisperNativeEstimate",
        confidence: 0.98,
      },
      {
        id: "w-0-1",
        text: "to",
        startUs: 650_000,
        endUs: 850_000,
        provenance: "whisperNativeEstimate",
        confidence: 0.99,
      },
      {
        id: "w-0-2",
        text: "Open",
        startUs: 900_000,
        endUs: 1_400_000,
        provenance: "whisperNativeEstimate",
        confidence: 0.96,
      },
      {
        id: "w-0-3",
        text: "Media",
        startUs: 1_450_000,
        endUs: 1_900_000,
        provenance: "whisperNativeEstimate",
        confidence: 0.95,
      },
      {
        id: "w-0-4",
        text: "Workbench",
        startUs: 1_950_000,
        endUs: 2_600_000,
        provenance: "whisperNativeEstimate",
        punctuationSuffix: ".",
        confidence: 0.97,
      },
    ],
    warnings: [],
  };
}

export async function segmentWordsToCues(
  words: AlignedWord[],
  policy: SegmentationPolicy = DEFAULT_SEGMENTATION_POLICY,
  language = "en",
): Promise<SubtitleCue[]> {
  if (isTauriEnvironment()) {
    return invoke<SubtitleCue[]>("segment_words_to_cues", {
      words,
      policy,
      language,
    });
  }

  if (words.length === 0) {
    return [];
  }

  // Pure TS fallback for web/unit test mock
  const cues: SubtitleCue[] = [];
  let currentGroup: AlignedWord[] = [];

  for (let i = 0; i < words.length; i++) {
    const word = words[i];
    currentGroup.push(word);

    const isLast = i === words.length - 1;
    const nextWord = words[i + 1];
    const silenceGap = nextWord ? nextWord.startUs - word.endUs : 0;
    const hasPunctuation =
      policy.splitOnPunctuation &&
      (word.punctuationSuffix?.includes(".") ||
        word.punctuationSuffix?.includes("?") ||
        word.punctuationSuffix?.includes("!") ||
        word.punctuationSuffix?.includes("।"));

    const cueDuration = word.endUs - currentGroup[0].startUs;
    const shouldSplit =
      isLast ||
      (policy.splitOnSilenceGapUs > 0 && silenceGap >= policy.splitOnSilenceGapUs) ||
      hasPunctuation ||
      cueDuration >= policy.maxCueDurationUs;

    if (shouldSplit) {
      const text = currentGroup.map((w) => `${w.punctuationPrefix ?? ""}${w.text}${w.punctuationSuffix ?? ""}`).join(" ");
      cues.push({
        id: `cue-mock-${cues.length + 1}`,
        startUs: currentGroup[0].startUs,
        endUs: word.endUs,
        text,
        words: [...currentGroup],
      });
      currentGroup = [];
    }
  }

  return cues;
}

export async function refineSubtitleCues(
  cues: SubtitleCue[],
  minDurationUs: number,
): Promise<SubtitleCue[]> {
  if (isTauriEnvironment()) {
    return invoke<SubtitleCue[]>("refine_subtitle_cues", {
      cues,
      minDurationUs,
    });
  }

  // Refine in JS for mock
  return cues.map((cue) => {
    if (!cue.words || cue.words.length === 0) return cue;
    const firstStart = cue.words[0].startUs;
    const lastEnd = cue.words[cue.words.length - 1].endUs;
    const dur = Math.max(lastEnd - firstStart, minDurationUs);
    return {
      ...cue,
      startUs: firstStart,
      endUs: firstStart + dur,
    };
  });
}

export async function updateWordTimingInSubtitleCue(
  cue: SubtitleCue,
  wordId: string,
  startUs: number,
  endUs: number,
): Promise<SubtitleCue> {
  if (isTauriEnvironment()) {
    return invoke<SubtitleCue>("update_word_timing_in_subtitle_cue", {
      cue,
      wordId,
      startUs,
      endUs,
    });
  }

  if (!cue.words) return cue;
  const updatedWords = cue.words.map((w) => {
    if (w.id === wordId) {
      return {
        ...w,
        startUs,
        endUs,
        provenance: "userAdjusted" as const,
      };
    }
    return w;
  });

  return {
    ...cue,
    words: updatedWords,
  };
}

export async function applySmartSegmentedSubtitles(
  jobId: string,
  policy: SegmentationPolicy = DEFAULT_SEGMENTATION_POLICY,
  customTrackName?: string,
  document?: SubtitleDocument,
): Promise<SubtitleDocument> {
  if (isTauriEnvironment()) {
    return invoke<SubtitleDocument>("apply_smart_segmented_subtitles", {
      jobId,
      policy,
      customTrackName,
      document,
    });
  }

  const alignment = await getWordAlignmentResult(jobId);
  const cues = await segmentWordsToCues(alignment.words, policy, alignment.language);
  const track: SubtitleTrack = {
    id: `track-smart-${Date.now()}`,
    displayName: customTrackName || `Smart Subtitles (${alignment.language})`,
    language: alignment.language,
    sourceFormat: "nativeProject",
    cues,
    isEnabled: true,
    isVisible: true,
    schemaVersion: 1,
  };

  const baseDoc = document || {
    schemaVersion: 1,
    id: `doc-${Date.now()}`,
    tracks: [],
    createdAtEpochMs: Date.now(),
    updatedAtEpochMs: Date.now(),
  };

  return {
    ...baseDoc,
    tracks: [...baseDoc.tracks, track],
    activeTrackId: track.id,
    updatedAtEpochMs: Date.now(),
  };
}
