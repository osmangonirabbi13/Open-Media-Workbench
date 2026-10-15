import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

import {
  listVoiceLibrary,
  getVoiceDetails,
  getVoiceCompatibility,
  startVoicePreview,
  compareVoices,
  getVoicePreferences,
  toggleVoiceFavorite,
  setPreferredVoice,
  clearVoicePreviewCache,
  assignVoiceToTrack,
  removeVoiceAssignment,
  setProjectDefaultVoice,
  getProjectVoiceAssignments,
} from './voices';
import type { SubtitleDocument } from '../types/subtitles';

describe('AI Voice Library IPC & Mock Services', () => {
  it('lists voices and filters by language and locale', async () => {
    const all = await listVoiceLibrary();
    assert.ok(all.length >= 3);
    assert.ok(all.some((v) => v.id === 'piper-en-lessac-medium'));

    const englishOnly = await listVoiceLibrary('en');
    assert.ok(englishOnly.length >= 2);
    assert.ok(englishOnly.every((v) => v.language === 'en' || v.locale.startsWith('en-')));

    const spanishOnly = await listVoiceLibrary('es');
    assert.equal(spanishOnly.length, 1);
    assert.equal(spanishOnly[0].id, 'piper-es-sharvard-medium');
  });

  it('retrieves detailed metadata for a specific voice', async () => {
    const voice = await getVoiceDetails('piper-en-lessac-medium');
    assert.ok(voice);
    assert.equal(voice.displayName, 'Lessac (English - US)');
    assert.equal(voice.license, 'Public Domain');
    assert.equal(voice.speakerCount, 1);
    assert.equal(voice.sampleRate, 22050);

    const nonExistent = await getVoiceDetails('non-existent-voice');
    assert.equal(nonExistent, null);
  });

  it('evaluates voice compatibility with language support checks', async () => {
    // English Lessac with English text -> ready
    const compatEn = await getVoiceCompatibility('piper-en-lessac-medium', 'en');
    assert.equal(compatEn.status, 'ready');
    assert.equal(compatEn.isReady, true);

    // English Lessac with Spanish text -> unsupported language
    const compatEs = await getVoiceCompatibility('piper-en-lessac-medium', 'es');
    assert.equal(compatEs.status, 'unsupportedLanguage');
    assert.equal(compatEs.isReady, false);

    // Uninstalled voice -> availableForInstall
    const compatLibri = await getVoiceCompatibility('piper-en-libritts-high');
    assert.equal(compatLibri.status, 'availableForInstall');
    assert.equal(compatLibri.isReady, false);
  });

  it('manages voice preferences: favorites, recents, and preferred default', async () => {
    const initial = await getVoicePreferences();
    assert.ok(initial);

    // Toggle favorite
    await toggleVoiceFavorite('piper-es-sharvard-medium');
    const updatedPrefs = await getVoicePreferences();
    assert.ok(updatedPrefs.favoriteVoiceIds.includes('piper-es-sharvard-medium'));

    // Set preferred voice
    await setPreferredVoice('piper-en-lessac-medium');
    const prefsWithDefault = await getVoicePreferences();
    assert.equal(prefsWithDefault.preferredVoiceId, 'piper-en-lessac-medium');

    // Untoggle favorite
    await toggleVoiceFavorite('piper-es-sharvard-medium');
    const finalPrefs = await getVoicePreferences();
    assert.ok(!finalPrefs.favoriteVoiceIds.includes('piper-es-sharvard-medium'));
  });

  it('synthesizes voice preview and compares two voices', async () => {
    const preview = await startVoicePreview({
      voiceId: 'piper-en-lessac-medium',
      customText: 'Custom preview text for testing.',
    });
    assert.equal(preview.voiceId, 'piper-en-lessac-medium');
    assert.ok(preview.durationUs > 0);
    assert.equal(preview.sampleRate, 22050);

    // A/B comparison
    const comparison = await compareVoices({
      voiceIdA: 'piper-en-lessac-medium',
      voiceIdB: 'piper-es-sharvard-medium',
      text: 'Identical sample text for A/B comparison.',
    });
    assert.ok(comparison.voiceA);
    assert.ok(comparison.voiceB);
    assert.equal(comparison.voiceA.voiceId, 'piper-en-lessac-medium');
    assert.equal(comparison.voiceB.voiceId, 'piper-es-sharvard-medium');

    // Comparing identical voice rejects
    await assert.rejects(
      async () => {
        await compareVoices({
          voiceIdA: 'piper-en-lessac-medium',
          voiceIdB: 'piper-en-lessac-medium',
        });
      },
      /Comparison requires two distinct installed voices/
    );

    // Clear preview cache
    const cleared = await clearVoicePreviewCache();
    assert.ok(typeof cleared === 'number');
  });

  it('binds voice assignments to SubtitleDocument tracks and project defaults', async () => {
    const baseDoc: SubtitleDocument = {
      schemaVersion: 1,
      id: 'doc-1',
      associatedMediaAssetId: 'asset-1',
      tracks: [
        {
          id: 'track-en',
          language: 'en',
          displayName: 'English (Original)',
          sourceFormat: 'srt',
          cues: [],
          isEnabled: true,
          isVisible: true,
          schemaVersion: 1,
        },
      ],
      activeTrackId: 'track-en',
      createdAtEpochMs: Date.now(),
      updatedAtEpochMs: Date.now(),
      metadata: {},
    };

    // Set default voice
    const withDefault = await setProjectDefaultVoice(baseDoc, 'piper-en-lessac-medium');
    const assignments1 = getProjectVoiceAssignments(withDefault);
    assert.equal(assignments1.defaultVoiceId, 'piper-en-lessac-medium');

    // Assign voice to track
    const withTrackVoice = await assignVoiceToTrack(withDefault, 'track-en', 'piper-en-lessac-medium');
    const assignments2 = getProjectVoiceAssignments(withTrackVoice);
    assert.equal(assignments2.trackAssignments.length, 1);
    assert.equal(assignments2.trackAssignments[0].trackId, 'track-en');
    assert.equal(assignments2.trackAssignments[0].voiceId, 'piper-en-lessac-medium');

    // Remove track assignment
    const removed = await removeVoiceAssignment(withTrackVoice, 'track-en');
    const assignments3 = getProjectVoiceAssignments(removed);
    assert.equal(assignments3.trackAssignments.length, 0);
  });
});
