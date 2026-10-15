import { test } from "node:test";
import assert from "node:assert/strict";
import {
  cancelAiModelDownload,
  formatBytes,
  formatCapabilityLabel,
  formatStatusBadge,
  getAiHardwareInfo,
  getAiStorageUsage,
  listAiModels,
  removeAiModel,
  startAiModelDownload,
  verifyAiModel,
} from "./aiManager";

test("AI Model Catalog: contains verified model entries with accurate schema", async () => {
  const models = await listAiModels();
  assert.ok(models.length >= 4, "Must contain at least 4 catalog entries");

  for (const item of models) {
    const { manifest } = item;
    assert.ok(manifest.id.length > 0);
    assert.ok(manifest.name.length > 0);
    assert.ok(manifest.family.length > 0);
    assert.ok(manifest.expectedSizeBytes > 0);
    assert.ok(manifest.licenseName.length > 0);

    if (manifest.installable) {
      assert.ok(manifest.artifactUrl?.startsWith("https://huggingface.co/"));
      assert.equal(manifest.expectedSha256?.length, 64);
    } else {
      assert.equal(manifest.installable, false);
    }
  }
});

test("AI Model Catalog: distinguishes installable vs catalog-only models", async () => {
  const models = await listAiModels();

  const tiny = models.find((m) => m.manifest.id === "whisper-tiny");
  assert.ok(tiny);
  assert.equal(tiny.manifest.installable, true);

  const opus = models.find((m) => m.manifest.id === "opus-mt-en-es");
  assert.ok(opus);
  assert.equal(opus.manifest.installable, false, "Catalog-only model must not be installable");
});

test("AI Hardware Info: reports hardware capability and uncertainty honestly", async () => {
  const hw = await getAiHardwareInfo();
  assert.ok(hw.os.length > 0);
  assert.ok(hw.arch.length > 0);
  assert.ok(hw.logicalCpus >= 1);
  assert.equal(hw.gpuBackendInstalled, false);
  assert.equal(hw.gpuRuntimeUsable, false);
});

test("AI Formatters: formatBytes formats accurately across scale", () => {
  assert.equal(formatBytes(0), "0 B");
  assert.equal(formatBytes(1024), "1.0 KB");
  assert.equal(formatBytes(77691713), "74.1 MB");
  assert.equal(formatBytes(1073741824), "1.0 GB");
});

test("AI Formatters: formatCapabilityLabel produces human-readable labels", () => {
  assert.equal(formatCapabilityLabel("speechToText"), "Speech to Text");
  assert.equal(formatCapabilityLabel("textGeneration"), "Local LLM / Text");
  assert.equal(formatCapabilityLabel("translation"), "Neural Translation");
  assert.equal(formatCapabilityLabel("textToSpeech"), "Text to Speech (TTS)");
});

test("AI Formatters: formatStatusBadge provides accessible badges and descriptions", () => {
  const installedBadge = formatStatusBadge("installed");
  assert.equal(installedBadge.label, "Installed (Runtime Pending)");
  assert.ok(installedBadge.bg.includes("emerald"));

  const notInstalledBadge = formatStatusBadge("notInstalled");
  assert.equal(notInstalledBadge.label, "Available to Download");

  const corruptBadge = formatStatusBadge("corrupted");
  assert.equal(corruptBadge.label, "Corrupted / Missing");
});

test("AI Lifecycle: download, verify, and safe removal flow in test environment", async () => {
  const modelId = "whisper-tiny";

  // Start download
  const progress = await startAiModelDownload(modelId);
  assert.equal(progress.modelId, modelId);
  assert.equal(progress.state, "complete");

  // Verify model
  const isValid = await verifyAiModel(modelId);
  assert.equal(isValid, true);

  // Check storage usage
  const storage = await getAiStorageUsage();
  assert.ok(storage.installedModelsCount >= 1);
  assert.ok(storage.totalModelsSizeBytes > 0);

  // Remove model
  await removeAiModel(modelId);

  // Check storage cleaned up
  const storageAfter = await getAiStorageUsage();
  assert.equal(storageAfter.installedModelsCount, 0);
  assert.equal(storageAfter.totalModelsSizeBytes, 0);
});

test("AI Cancellation: cancels pending download safely", async () => {
  await startAiModelDownload("piper-en-lessac-medium");
  await cancelAiModelDownload("piper-en-lessac-medium");

  const storage = await getAiStorageUsage();
  assert.equal(storage.installedModelsCount, 0);
});

test("Zero AI models required for base app execution", async () => {
  const storage = await getAiStorageUsage();
  assert.equal(storage.installedModelsCount, 0, "No models bundled by default");
});
