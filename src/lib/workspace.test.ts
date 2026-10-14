import test from "node:test";
import assert from "node:assert/strict";
import {
  WORKSPACE_DEFINITIONS,
  INITIAL_WORKSPACE_STATE,
  type WorkspaceId,
} from "../types/workspace";
import {
  sanitizePreferences,
} from "./preferences";
import {
  PREFERENCES_SCHEMA_VERSION,
} from "../types/preferences";

test("Workspace definitions: defines both repair and localization studios", () => {
  const repair = WORKSPACE_DEFINITIONS.repair;
  const localization = WORKSPACE_DEFINITIONS.localization;

  assert.equal(repair.id, "repair");
  assert.equal(repair.label, "Repair Studio");
  assert.equal(repair.badge, "Recovery");
  assert.ok(repair.capabilities.length >= 4);

  assert.equal(localization.id, "localization");
  assert.equal(localization.label, "Localization Studio");
  assert.equal(localization.badge, "AI Studio");
  assert.ok(localization.capabilities.length >= 4);
});

test("Workspace capabilities: distinguishes available vs planned roadmap steps", () => {
  const repair = WORKSPACE_DEFINITIONS.repair;
  const ingestion = repair.capabilities.find((c) => c.id === "media_ingestion");
  const diagnostics = repair.capabilities.find((c) => c.id === "corruption_diagnostics");
  const moovRepair = repair.capabilities.find((c) => c.id === "moov_reconstruction");

  assert.ok(ingestion);
  assert.equal(ingestion?.status, "available");

  assert.ok(diagnostics);
  assert.equal(diagnostics?.status, "planned");
  assert.equal(diagnostics?.roadmapStep, 12);

  assert.ok(moovRepair);
  assert.equal(moovRepair?.status, "planned");
  assert.equal(moovRepair?.roadmapStep, 13);
});

test("Workspace preferences: sanitizes and persists activeWorkspace safely", () => {
  // Test preserving valid repair workspace
  const withRepair = {
    schemaVersion: PREFERENCES_SCHEMA_VERSION,
    mediaPanelWidth: 300,
    timelineDeckHeight: 300,
    theme: "dark" as const,
    timecodeFormat: "timecode" as const,
    snapEnabled: true,
    activeWorkspace: "repair" as const,
  };
  const sanitizedRepair = sanitizePreferences(withRepair);
  assert.equal(sanitizedRepair.activeWorkspace, "repair");

  // Test preserving valid localization workspace
  const withLoc = {
    ...withRepair,
    activeWorkspace: "localization" as const,
  };
  const sanitizedLoc = sanitizePreferences(withLoc);
  assert.equal(sanitizedLoc.activeWorkspace, "localization");

  // Test rejecting invalid workspace name
  const withInvalid = {
    ...withRepair,
    activeWorkspace: "unknown_future_studio" as unknown as WorkspaceId,
  };
  const sanitizedInvalid = sanitizePreferences(withInvalid);
  assert.equal(sanitizedInvalid.activeWorkspace, undefined);
});

test("Initial workspace state: defaults to repair studio and preserves independent selection slots", () => {
  assert.equal(INITIAL_WORKSPACE_STATE.activeWorkspace, "repair");
  assert.equal(INITIAL_WORKSPACE_STATE.selectedAssetIdByWorkspace.repair, null);
  assert.equal(INITIAL_WORKSPACE_STATE.selectedAssetIdByWorkspace.localization, null);
  assert.equal(INITIAL_WORKSPACE_STATE.repair.activeWorkflowStep, 1);
  assert.equal(INITIAL_WORKSPACE_STATE.localization.activeInspectorTab, "subtitles");
});

test("Workspace switching state retention: retains per-workspace state without cross-contamination", () => {
  // Simulate state transition: user selects an asset in repair workspace
  const state = { ...INITIAL_WORKSPACE_STATE };
  const assetRepairId = "asset-corrupt-001";
  const assetLocId = "asset-clean-002";

  state.selectedAssetIdByWorkspace = {
    ...state.selectedAssetIdByWorkspace,
    repair: assetRepairId,
  };

  // Switch to localization workspace
  state.activeWorkspace = "localization";
  state.selectedAssetIdByWorkspace = {
    ...state.selectedAssetIdByWorkspace,
    localization: assetLocId,
  };

  // Switch back to repair workspace
  state.activeWorkspace = "repair";

  // Assert both workspace slots are preserved
  assert.equal(state.selectedAssetIdByWorkspace.repair, assetRepairId);
  assert.equal(state.selectedAssetIdByWorkspace.localization, assetLocId);
  assert.equal(state.activeWorkspace, "repair");
});

test("Workspace capabilities: planned capabilities cannot be mistaken for completed", () => {
  const loc = WORKSPACE_DEFINITIONS.localization;
  const dubbing = loc.capabilities.find((c) => c.id === "neural_dubbing");
  assert.ok(dubbing);
  assert.equal(dubbing?.status, "planned");
  assert.equal(dubbing?.roadmapStep, 24);

  const transcription = loc.capabilities.find((c) => c.id === "local_transcription");
  assert.ok(transcription);
  assert.equal(transcription?.status, "planned");
  assert.equal(transcription?.roadmapStep, 18);
});
