import test from "node:test";
import assert from "node:assert/strict";
import {
  runMediaDiagnostics,
  cancelMediaDiagnostics,
  pickAndImportReferenceAsset,
  analyzeReferenceCompatibility,
  startMp4Reconstruction,
  registerRecoveryCandidatePreview,
  getRecoveryReview,
  runAdvancedOutputValidation,
  exportRecoveryReviewReport,
  getRecoveryHistory,
  clearRecoveryHistory,
} from "./repair";
import type {
  DiagnosticReport,
  DiagnosticFinding,
  RepairRecommendation,
  ReferenceCompatibilityReport,
  ReconstructionReport,
  RecoveryReview,
  RecoveryHistoryEntry,
} from "../types/repair";

test("Repair Diagnostics: executes mock scan when outside Tauri desktop", async () => {
  const report = await runMediaDiagnostics("asset-mock-123", "quick");

  assert.equal(report.schemaVersion, 1);
  assert.equal(report.assetId, "asset-mock-123");
  assert.equal(report.analysisMode, "quick");
  assert.equal(report.analysisStatus, "completed");
  assert.ok(report.findings.length > 0);
  assert.ok(report.recommendations.length > 0);
  assert.equal(report.inspectionCoverage.boxCountFound, 3);
  assert.equal(report.inspectionCoverage.decodeSampled, false);
});

test("Repair Diagnostics: deep mode flags decode sampling correctly", async () => {
  const report = await runMediaDiagnostics("asset-mock-deep", "deep");

  assert.equal(report.analysisMode, "deep");
  assert.equal(report.inspectionCoverage.decodeSampled, true);
  assert.equal(report.inspectionCoverage.decodeSampleFrames, 30);
});

test("Repair Diagnostics: cancel returns true in mock environment", async () => {
  const cancelled = await cancelMediaDiagnostics("asset-mock-123");
  assert.equal(cancelled, true);
});

test("Repair Diagnostics Data Contract: validates Finding structure", () => {
  const finding: DiagnosticFinding = {
    code: "MP4_MOOV_MISSING",
    severity: "critical",
    category: "boxStructure",
    summary: "Missing 'moov' atom header in MP4/MOV container",
    evidence: "Scanned 2 top-level boxes: 'mdat' was found at offset 24, but 'moov' is missing.",
    affectedStream: null,
    mediaTimeRangeMs: null,
    byteRange: { start: 0, length: 24 },
    confidence: "High",
    recommendedAction: "Requires reference-based MOOV atom reconstruction (Scheduled for STEP 14).",
  };

  assert.equal(finding.code, "MP4_MOOV_MISSING");
  assert.equal(finding.severity, "critical");
  assert.equal(finding.category, "boxStructure");
  assert.ok(finding.evidence.includes("mdat"));
  assert.ok(finding.recommendedAction?.includes("STEP 14"));
});

test("Repair Diagnostics Data Contract: validates Recommendation rules", () => {
  const rec: RepairRecommendation = {
    strategyId: "REFERENCE_BASED_RECONSTRUCTION",
    title: "Reference-Based MP4/MOV Reconstruction",
    description: "The container media payload (mdat) is present, but the crucial index atom (moov) is missing.",
    confidence: "High",
    requiredEvidence: ["mdat payload atom detected", "moov atom absent from container"],
    targetStep: 14,
    availableNow: false,
  };

  assert.equal(rec.strategyId, "REFERENCE_BASED_RECONSTRUCTION");
  assert.equal(rec.targetStep, 14);
  assert.equal(rec.availableNow, false);
  assert.equal(rec.requiredEvidence.length, 2);
});

test("Repair Diagnostics Data Contract: JSON report serialization round-trip", () => {
  const sampleReport: DiagnosticReport = {
    schemaVersion: 1,
    analysisId: "diag-sample-uuid",
    assetId: "asset-456",
    fileName: "damaged_video.mp4",
    detectedContainer: {
      formatType: "MP4 (ISO BMFF)",
      extension: "mp4",
      signatureMatched: true,
      signatureDetails: "ISO BMFF ftyp box with brand 'isom'",
      mismatchWarning: null,
    },
    analysisMode: "quick",
    startedAtEpochMs: 1700000000000,
    completedAtEpochMs: 1700000000500,
    analysisStatus: "completed",
    findings: [],
    recommendations: [],
    inspectionCoverage: {
      headerScannedBytes: 512,
      totalFileBytes: 10485760,
      boxCountFound: 4,
      probeExecuted: true,
      decodeSampled: false,
      decodeSampleFrames: null,
    },
    toolVersions: "FFmpeg/FFprobe integrated toolchain",
  };

  const jsonStr = JSON.stringify(sampleReport);
  const parsed = JSON.parse(jsonStr) as DiagnosticReport;

  assert.deepEqual(parsed, sampleReport);
  assert.equal(parsed.schemaVersion, 1);
  assert.equal(parsed.fileName, "damaged_video.mp4");
});

test("Recovery Pipeline: executes mock recovery job in browser environment", async () => {
  const { startMediaRecovery, cancelMediaRecovery } = await import("./repair");
  const jobStatus = await startMediaRecovery({
    assetId: "asset-test-rec",
    method: "losslessRemux",
    outputContainerPreference: "mp4",
  });

  assert.ok(jobStatus.jobId.startsWith("mock-job-"));
  assert.equal(jobStatus.assetId, "asset-test-rec");
  assert.equal(jobStatus.status, "completed");
  assert.ok(jobStatus.report);
  assert.equal(jobStatus.report?.recoveryMethod, "losslessRemux");
  assert.equal(jobStatus.report?.validation.passed, true);
  assert.equal(jobStatus.report?.validation.isPlayable, true);

  const cancelled = await cancelMediaRecovery("mock-job-123");
  assert.equal(cancelled, true);
});

test("Recovery Pipeline Data Contract: validates RecoveryReport structure", () => {
  const report: import("../types/repair").RecoveryReport = {
    schemaVersion: 1,
    jobId: "job-12345",
    sourceAssetId: "asset-src-1",
    sourceFileName: "source.mp4",
    recoveryMethod: "streamSalvage",
    originalContainer: "mp4",
    intendedOutputContainer: "mp4",
    actualOutputContainer: "mp4",
    inputStreams: [
      {
        index: 0,
        streamType: "video",
        codecName: "h264",
        durationSeconds: 15.0,
      },
    ],
    outputStreams: [
      {
        index: 0,
        streamType: "video",
        codecName: "h264",
        durationSeconds: 15.0,
      },
    ],
    processingStatus: "completed",
    validation: {
      passed: true,
      isPlayable: true,
      validationLevel: "sampledDecode",
      streamsPresent: ["Video stream (codec: h264)"],
      streamsExpected: ["1 video stream(s)"],
      streamLossDetected: false,
      outputDurationSeconds: 15.0,
      details: "Validation succeeded with 1 video stream.",
      warnings: [],
    },
    warnings: [],
    observedFfmpegErrors: [],
    outputFileSizeBytes: 2048576,
    outputFilePath: "/safe/staging/job-12345/recovered.mp4",
    sourceDurationSeconds: 15.0,
    outputDurationSeconds: 15.0,
    durationDifferenceSeconds: 0.0,
    startedAtEpochMs: 1700000000000,
    completedAtEpochMs: 1700000001000,
    limitations: ["Audio stream was omitted per stream salvage configuration."],
  };

  assert.equal(report.schemaVersion, 1);
  assert.equal(report.recoveryMethod, "streamSalvage");
  assert.equal(report.processingStatus, "completed");
  assert.equal(report.validation.isPlayable, true);
  assert.equal(report.limitations.length, 1);
});

test("Reconstruction Pipeline: executes mock reconstruction job in browser environment", async () => {
  const refAsset = await pickAndImportReferenceAsset();
  assert.ok(refAsset);
  assert.equal(refAsset.fileName, "mock_reference.mp4");

  const compat = await analyzeReferenceCompatibility("damaged-1", refAsset.id);
  assert.equal(compat.compatibility, "compatible");
  assert.ok(compat.tracks.length > 0);

  const job = await startMp4Reconstruction({
    damagedAssetId: "damaged-1",
    referenceAssetId: refAsset.id,
    outputContainerPreference: "mp4",
  });

  assert.ok(job.jobId.startsWith("mock-reconst-"));
  assert.equal(job.status, "completed");
  assert.ok(job.report);
  assert.equal(job.report.validation.passed, true);
  assert.equal(job.report.validation.isPlayable, true);
});

test("Reconstruction Data Contracts: validates ReferenceCompatibilityReport and ReconstructionReport", () => {
  const compat: ReferenceCompatibilityReport = {
    schemaVersion: 1,
    referenceAssetId: "ref-456",
    damagedAssetId: "dam-123",
    referenceContainer: "MP4",
    damagedContainer: "MP4",
    tracks: [
      {
        trackId: 0,
        mediaType: "video",
        codecName: "h264",
        codecProfile: "High",
        resolution: "1920x1080",
        frameRate: 30,
        timebase: "1/15360",
        sampleRate: null,
        channels: null,
        extradataSize: 42,
      },
    ],
    compatibility: "compatible",
    summary: "Compatible donor reference",
    warnings: [],
    unknownFields: [],
  };

  assert.equal(compat.schemaVersion, 1);
  assert.equal(compat.compatibility, "compatible");
  assert.equal(compat.tracks[0].codecName, "h264");

  const reconstReport: ReconstructionReport = {
    schemaVersion: 1,
    operationId: "op-123",
    engineName: "Built-in Safe Rust Reconstructor",
    engineVersion: "1.0.0-native",
    damagedAssetId: "dam-123",
    referenceAssetId: "ref-456",
    damagedFileName: "unfinalized.mp4",
    referenceFileName: "ref.mp4",
    referenceCompatibility: compat,
    attemptedMode: "SPS/PPS Donor Injection",
    reconstructedStreams: [
      {
        index: 0,
        streamType: "video",
        codecName: "h264",
        durationSeconds: 12.5,
      },
    ],
    recoveredDurationSeconds: 12.5,
    validation: {
      passed: true,
      isPlayable: true,
      validationLevel: "sampledDecode",
      streamsPresent: ["h264"],
      streamsExpected: ["h264"],
      streamLossDetected: false,
      outputDurationSeconds: 12.5,
      details: "Validation passed",
      warnings: [],
    },
    outputFileSizeBytes: 1048576,
    outputFilePath: "/staging/output.mp4",
    processingStatus: "completed",
    failureDetails: null,
    warnings: [],
    uncertainties: [],
    startedAtEpochMs: 1700000000000,
    completedAtEpochMs: 1700000001000,
  };

  assert.equal(reconstReport.engineName, "Built-in Safe Rust Reconstructor");
  assert.equal(reconstReport.validation.isPlayable, true);
  assert.equal(reconstReport.reconstructedStreams.length, 1);
});

test("Recovery Review: registers mock preview asset and returns review data", async () => {
  const asset = await registerRecoveryCandidatePreview("job-test-123");
  assert.equal(asset.id, "mock-candidate-job-test-123");

  const review = await getRecoveryReview("job-test-123");
  assert.equal(review.schemaVersion, 1);
  assert.equal(review.recoveryJobId, "job-test-123");
  assert.equal(review.recoveryMethod, "Lossless Remux");
  assert.equal(review.recoveryStatus, "completed");
  assert.ok(review.sourceSummary);
  assert.ok(review.outputSummary);
  assert.equal(review.validationSummary.overallOutcome, "passed");
  assert.equal(review.timelineEvidence?.recoveredDurationSeconds, 8.0);
  assert.equal(review.limitations.length, 1);
});

test("Recovery Review: runs advanced full decode validation in mock environment", async () => {
  const summary = await runAdvancedOutputValidation("job-test-123", "fullDecode");
  assert.equal(summary.level, "fullDecode");
  assert.equal(summary.overallOutcome, "passed");
  assert.ok(summary.inspectionCoverage.includes("Full file decode"));
  assert.ok(summary.decodeChecks.some((c) => c.title === "Full Bitstream Decode"));
});

test("Recovery Review: exports HTML and JSON reports", async () => {
  const jsonPath = await exportRecoveryReviewReport("job-test-123", "json");
  assert.ok(jsonPath);
  assert.ok(jsonPath.endsWith(".json"));

  const htmlPath = await exportRecoveryReviewReport("job-test-123", "html");
  assert.ok(htmlPath);
  assert.ok(htmlPath.endsWith(".html"));
});

test("Recovery History: queries and clears history entries in mock environment", async () => {
  const history: RecoveryHistoryEntry[] = await getRecoveryHistory();
  assert.ok(Array.isArray(history));
  assert.ok(history.length > 0);
  assert.equal(history[0].recoveryMethod, "Lossless Container Remux");

  const cleared = await clearRecoveryHistory();
  assert.equal(cleared, true);
});

test("Recovery Review Data Contract: validates RecoveryReview structure", () => {
  const review: RecoveryReview = {
    schemaVersion: 1,
    reviewId: "rev-999",
    sourceAssetId: "dam-123",
    recoveryJobId: "job-999",
    recoveryMethod: "Stream Salvage",
    recoveryStatus: "completed",
    outputAssetId: "asset-rec-999",
    outputCandidateId: "cand-999",
    sourceSummary: {
      fileName: "damaged.mp4",
      fileSizeBytes: 5000000,
      containerFormat: "QuickTime / MOV",
      durationSeconds: 10.0,
      streams: [
        {
          index: 0,
          streamType: "video",
          codecName: "h264",
          durationSeconds: 10.0,
        },
      ],
      isPlayable: false,
    },
    outputSummary: {
      fileName: "recovered.mp4",
      fileSizeBytes: 4800000,
      containerFormat: "MP4",
      durationSeconds: 10.0,
      streams: [
        {
          index: 0,
          streamType: "video",
          codecName: "h264",
          durationSeconds: 10.0,
        },
      ],
      isPlayable: true,
    },
    validationSummary: {
      level: "sampledDecode",
      overallOutcome: "warning",
      containerChecks: [
        {
          code: "CNT_VALID",
          outcome: "passed",
          title: "Container Structure",
          message: "Valid MP4 header",
        },
      ],
      streamChecks: [],
      timestampChecks: [],
      decodeChecks: [],
      audioChecks: [
        {
          code: "AUD_ABSENT_INTENTIONAL",
          outcome: "warning",
          title: "Audio Presence",
          message: "No audio stream present (video-only salvage)",
        },
      ],
      durationComparison: {
        sourceDurationSeconds: 10.0,
        recoveredDurationSeconds: 10.0,
        differenceSeconds: 0.0,
        isSourceDurationUnknown: false,
        explanation: "Durations match exactly",
      },
      inspectionCoverage: "Initial 8.0s inspected.",
    },
    timelineEvidence: {
      sourceDurationSeconds: 10.0,
      recoveredDurationSeconds: 10.0,
      videoStartOffsetSeconds: 0.0,
      audioStartOffsetSeconds: null,
      avDriftSeconds: null,
      knownDecodeFailures: [],
      uninspectedRegionsNote: "Remainder unverified",
    },
    warnings: ["Audio track omitted"],
    limitations: ["Salvaged stream only"],
    createdAtEpochMs: 1700000000000,
  };

  assert.equal(review.schemaVersion, 1);
  assert.equal(review.validationSummary.overallOutcome, "warning");
  assert.equal(review.validationSummary.audioChecks[0].code, "AUD_ABSENT_INTENTIONAL");
  assert.equal(review.timelineEvidence?.recoveredDurationSeconds, 10.0);
});

