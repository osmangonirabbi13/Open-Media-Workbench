import test from "node:test";
import assert from "node:assert/strict";
import {
  clamp,
  sanitizePreferences,
} from "./preferences";
import {
  DEFAULT_WORKSPACE_PREFERENCES,
  LAYOUT_BOUNDS,
  PREFERENCES_SCHEMA_VERSION,
} from "../types/preferences";

test("clamp: returns value within boundaries", () => {
  assert.equal(clamp(300, 200, 500), 300);
  assert.equal(clamp(100, 200, 500), 200);
  assert.equal(clamp(600, 200, 500), 500);
  assert.equal(clamp(NaN, 200, 500), 200);
});

test("sanitizePreferences: returns defaults for null, undefined, or primitive values", () => {
  assert.deepEqual(sanitizePreferences(null), DEFAULT_WORKSPACE_PREFERENCES);
  assert.deepEqual(sanitizePreferences(undefined), DEFAULT_WORKSPACE_PREFERENCES);
  assert.deepEqual(sanitizePreferences("invalid json"), DEFAULT_WORKSPACE_PREFERENCES);
  assert.deepEqual(sanitizePreferences(12345), DEFAULT_WORKSPACE_PREFERENCES);
});

test("sanitizePreferences: rejects mismatched schema version", () => {
  const outdated = {
    schemaVersion: 999,
    mediaPanelWidth: 300,
    timelineDeckHeight: 300,
  };
  assert.deepEqual(sanitizePreferences(outdated), DEFAULT_WORKSPACE_PREFERENCES);
});

test("sanitizePreferences: clamps out-of-bounds dimensions", () => {
  const outOfBounds = {
    schemaVersion: PREFERENCES_SCHEMA_VERSION,
    mediaPanelWidth: 9999, // Too wide
    timelineDeckHeight: 10, // Too short
    theme: "dark",
    timecodeFormat: "timecode",
    snapEnabled: true,
  };

  const sanitized = sanitizePreferences(outOfBounds);
  assert.equal(sanitized.mediaPanelWidth, LAYOUT_BOUNDS.MEDIA_MAX_WIDTH);
  assert.equal(sanitized.timelineDeckHeight, LAYOUT_BOUNDS.TIMELINE_MIN_HEIGHT);
});

test("sanitizePreferences: handles invalid timecodeFormat by defaulting to timecode", () => {
  const invalidFormat = {
    schemaVersion: PREFERENCES_SCHEMA_VERSION,
    mediaPanelWidth: 300,
    timelineDeckHeight: 300,
    theme: "dark",
    timecodeFormat: "invalid-format",
    snapEnabled: false,
  };

  const sanitized = sanitizePreferences(invalidFormat);
  assert.equal(sanitized.timecodeFormat, "timecode");
  assert.equal(sanitized.snapEnabled, false);
});

test("sanitizePreferences: preserves valid preferences", () => {
  const valid = {
    schemaVersion: PREFERENCES_SCHEMA_VERSION,
    mediaPanelWidth: 350,
    timelineDeckHeight: 400,
    theme: "dark" as const,
    timecodeFormat: "frames" as const,
    snapEnabled: false,
  };

  const sanitized = sanitizePreferences(valid);
  assert.deepEqual(sanitized, valid);
});
