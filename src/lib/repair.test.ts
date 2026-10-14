import test from "node:test";
import assert from "node:assert/strict";
import {
  runMediaDiagnostics,
  cancelMediaDiagnostics,
} from "./repair";
import type { DiagnosticReport, DiagnosticFinding, RepairRecommendation } from "../types/repair";

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
