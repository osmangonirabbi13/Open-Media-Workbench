import test from "node:test";
import assert from "node:assert/strict";
import {
  getWordAlignmentResult,
  segmentWordsToCues,
  refineSubtitleCues,
  updateWordTimingInSubtitleCue,
  applySmartSegmentedSubtitles,
} from "./alignment";
import type { AlignedWord, SegmentationPolicy } from "../types/alignment";
import { DEFAULT_SEGMENTATION_POLICY } from "../types/alignment";
import type { SubtitleCue, SubtitleDocument } from "../types/subtitles";

test("Alignment: getWordAlignmentResult returns structured alignment data", async () => {
  const result = await getWordAlignmentResult("job-mock-123");
  assert.equal(result.jobId, "job-mock-123");
  assert.equal(result.timingQuality, "aligned");
  assert.equal(result.words.length, 5);
  assert.equal(result.words[0].text, "Welcome");
  assert.equal(result.words[0].startUs, 0);
  assert.equal(result.words[0].provenance, "whisperNativeEstimate");
});

test("Smart Segmentation: breaks cues on punctuation and silence gaps", async () => {
  const words: AlignedWord[] = [
    {
      id: "w1",
      text: "Hello",
      startUs: 0,
      endUs: 400_000,
      provenance: "whisperNativeEstimate",
    },
    {
      id: "w2",
      text: "world",
      startUs: 450_000,
      endUs: 900_000,
      provenance: "whisperNativeEstimate",
      punctuationSuffix: ".",
    },
    {
      id: "w3",
      text: "How",
      startUs: 1_600_000, // 700ms silence gap (> 400ms)
      endUs: 1_900_000,
      provenance: "whisperNativeEstimate",
    },
    {
      id: "w4",
      text: "are",
      startUs: 1_950_000,
      endUs: 2_150_000,
      provenance: "whisperNativeEstimate",
    },
    {
      id: "w5",
      text: "you",
      startUs: 2_200_000,
      endUs: 2_600_000,
      provenance: "whisperNativeEstimate",
      punctuationSuffix: "?",
    },
  ];

  const policy: SegmentationPolicy = {
    ...DEFAULT_SEGMENTATION_POLICY,
    splitOnSilenceGapUs: 400_000,
    splitOnPunctuation: true,
  };

  const cues = await segmentWordsToCues(words, policy, "en");
  assert.equal(cues.length, 2);
  assert.equal(cues[0].text, "Hello world.");
  assert.equal(cues[0].words?.length, 2);
  assert.equal(cues[1].text, "How are you?");
  assert.equal(cues[1].words?.length, 3);
});

test("Smart Segmentation: Bengali Unicode and Dari punctuation preservation", async () => {
  const words: AlignedWord[] = [
    {
      id: "bn1",
      text: "ওপেন",
      startUs: 100_000,
      endUs: 600_000,
      provenance: "whisperNativeEstimate",
    },
    {
      id: "bn2",
      text: "মিডিয়া",
      startUs: 650_000,
      endUs: 1_200_000,
      provenance: "whisperNativeEstimate",
    },
    {
      id: "bn3",
      text: "ওয়ার্কবেঞ্চ",
      startUs: 1_250_000,
      endUs: 2_000_000,
      provenance: "whisperNativeEstimate",
      punctuationSuffix: "।",
    },
    {
      id: "bn4",
      text: "স্বাগতম",
      startUs: 2_800_000,
      endUs: 3_500_000,
      provenance: "whisperNativeEstimate",
      punctuationSuffix: "!",
    },
  ];

  const policy: SegmentationPolicy = {
    ...DEFAULT_SEGMENTATION_POLICY,
    splitOnPunctuation: true,
  };

  const cues = await segmentWordsToCues(words, policy, "bn");
  assert.equal(cues.length, 2);
  assert.equal(cues[0].text, "ওপেন মিডিয়া ওয়ার্কবেঞ্চ।");
  assert.equal(cues[0].startUs, 100_000);
  assert.equal(cues[0].endUs, 2_000_000);
  assert.equal(cues[1].text, "স্বাগতম!");
});

test("Timing Refinement: snaps cue boundaries to first and last word timestamps", async () => {
  const cues: SubtitleCue[] = [
    {
      id: "cue-1",
      startUs: 0,
      endUs: 5_000_000, // bloated interval
      text: "Spoken phrase",
      words: [
        {
          id: "w1",
          text: "Spoken",
          startUs: 1_200_000,
          endUs: 1_800_000,
          provenance: "whisperNativeEstimate",
        },
        {
          id: "w2",
          text: "phrase",
          startUs: 1_850_000,
          endUs: 2_400_000,
          provenance: "whisperNativeEstimate",
        },
      ],
    },
  ];

  const refined = await refineSubtitleCues(cues, 1_000_000);
  assert.equal(refined.length, 1);
  assert.equal(refined[0].startUs, 1_200_000);
  // Duration is max(2.4s - 1.2s = 1.2s, min_duration=1s) = 1.2s
  assert.equal(refined[0].endUs, 2_400_000);
});

test("Manual Word Timing Correction: updates timing and marks provenance as userAdjusted", async () => {
  const cue: SubtitleCue = {
    id: "cue-10",
    startUs: 0,
    endUs: 3_000_000,
    text: "Quick fix",
    words: [
      {
        id: "w10",
        text: "Quick",
        startUs: 500_000,
        endUs: 1_000_000,
        provenance: "whisperNativeEstimate",
      },
    ],
  };

  const updatedCue = await updateWordTimingInSubtitleCue(cue, "w10", 600_000, 1_100_000);
  assert.equal(updatedCue.words?.[0].startUs, 600_000);
  assert.equal(updatedCue.words?.[0].endUs, 1_100_000);
  assert.equal(updatedCue.words?.[0].provenance, "userAdjusted");
});

test("Apply Smart Subtitles: non-destructively appends track with word timing", async () => {
  const initialDoc: SubtitleDocument = {
    schemaVersion: 1,
    id: "doc-1",
    tracks: [
      {
        id: "track-existing",
        language: "en",
        displayName: "Existing Track",
        sourceFormat: "srt",
        cues: [],
        isEnabled: true,
        isVisible: true,
        schemaVersion: 1,
      },
    ],
    createdAtEpochMs: 1000,
    updatedAtEpochMs: 1000,
  };

  const newDoc = await applySmartSegmentedSubtitles(
    "job-mock-123",
    DEFAULT_SEGMENTATION_POLICY,
    "Smart AI Track",
    initialDoc
  );

  assert.equal(newDoc.tracks.length, 2);
  assert.equal(newDoc.tracks[0].id, "track-existing");
  assert.equal(newDoc.tracks[1].displayName, "Smart AI Track");
  assert.ok(newDoc.tracks[1].cues.length > 0);
  assert.ok(newDoc.tracks[1].cues[0].words && newDoc.tracks[1].cues[0].words.length > 0);
});
