//! Client IPC and fallback mock services for the Offline AI Voice Library.

import { invoke } from '@tauri-apps/api/core';
import type { SubtitleDocument } from '../types/subtitles';
import type {
  ProjectVoiceAssignments,
  VoiceCatalogEntry,
  VoiceComparisonRequest,
  VoiceComparisonResult,
  VoiceCompatibility,
  VoicePreferences,
  VoicePreviewRequest,
  VoicePreviewResult,
} from '../types/voices';

const MOCK_CATALOG: VoiceCatalogEntry[] = [
  {
    id: 'piper-en-lessac-medium',
    displayName: 'Lessac (English - US)',
    modelId: 'piper-en-lessac-medium',
    runtimeBackend: 'piper',
    language: 'en',
    locale: 'en-US',
    version: 'v1.0.0',
    qualityTier: 'medium',
    artifactSizeBytes: 63206179,
    sourceUrl: 'https://huggingface.co/rhasspy/piper-voices',
    license: 'Public Domain',
    attribution: 'Donald Derrick / Lessac Dataset (Public Domain)',
    speakerCount: 1,
    supportedSpeakerIds: [0],
    sampleRate: 22050,
    description: 'Clear, natural North American English voice suitable for narration, subtitles, and video voiceover.',
    accent: 'North American English',
  },
  {
    id: 'piper-en-libritts-high',
    displayName: 'LibriTTS (English - Multi-speaker)',
    modelId: 'piper-en-libritts-high',
    runtimeBackend: 'piper',
    language: 'en',
    locale: 'en-US',
    version: 'v1.0.0',
    qualityTier: 'high',
    artifactSizeBytes: 110450000,
    sourceUrl: 'https://huggingface.co/rhasspy/piper-voices',
    license: 'CC-BY-4.0',
    attribution: 'Heiga Zen / LibriTTS Corpus (CC-BY-4.0)',
    speakerCount: 904,
    supportedSpeakerIds: Array.from({ length: 904 }, (_, i) => i),
    sampleRate: 22050,
    description: 'Expressive multi-speaker English voice model with 904 selectable speaker variants.',
    accent: 'Varied English Accents',
  },
  {
    id: 'piper-es-sharvard-medium',
    displayName: 'Sharvard (Spanish - Spain)',
    modelId: 'piper-es-sharvard-medium',
    runtimeBackend: 'piper',
    language: 'es',
    locale: 'es-ES',
    version: 'v1.0.0',
    qualityTier: 'medium',
    artifactSizeBytes: 64800000,
    sourceUrl: 'https://huggingface.co/rhasspy/piper-voices',
    license: 'Public Domain',
    attribution: 'Spanish Harvard Sentences Dataset (Public Domain)',
    speakerCount: 1,
    supportedSpeakerIds: [0],
    sampleRate: 22050,
    description: 'Standard Castilian Spanish voice suitable for Spanish subtitling and voiceover.',
    accent: 'European Spanish',
  },
  {
    id: 'piper-bn-ibrahim-medium',
    displayName: 'Ibrahim (Bengali - Bangladesh - Catalog Only)',
    modelId: 'piper-bn-ibrahim-medium',
    runtimeBackend: 'piper',
    language: 'bn',
    locale: 'bn-BD',
    version: 'v1.0.0',
    qualityTier: 'medium',
    artifactSizeBytes: 62100000,
    sourceUrl: 'https://huggingface.co/rhasspy/piper-voices',
    license: 'Public Domain',
    attribution: 'Ibrahim Bengali Speech Dataset (Public Domain)',
    speakerCount: 1,
    supportedSpeakerIds: [0],
    sampleRate: 22050,
    description: 'Bengali speech synthesis voice. Not verified for inference in the current environment.',
    accent: 'Bangladeshi Bengali (বাংলা)',
  },
];

let mockPreferences: VoicePreferences = {
  schemaVersion: 1,
  favoriteVoiceIds: ['piper-en-lessac-medium'],
  preferredVoiceId: 'piper-en-lessac-medium',
  recentVoiceIds: ['piper-en-lessac-medium'],
  lastSpeakerIds: {},
  updatedAtEpochMs: Date.now(),
};

function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function listVoiceLibrary(filterLang?: string): Promise<VoiceCatalogEntry[]> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<VoiceCatalogEntry[]>('list_voice_library', { filterLang });
    } catch {
      // fallback
    }
  }

  if (filterLang && filterLang.trim() !== '') {
    const clean = filterLang.trim().toLowerCase();
    return MOCK_CATALOG.filter(
      (v) =>
        v.language.toLowerCase() === clean ||
        v.locale.toLowerCase() === clean ||
        v.locale.toLowerCase().startsWith(`${clean}-`)
    );
  }
  return MOCK_CATALOG;
}

export async function getVoiceDetails(voiceId: string): Promise<VoiceCatalogEntry | null> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<VoiceCatalogEntry | null>('get_voice_details', { voiceId });
    } catch {
      // fallback
    }
  }
  return MOCK_CATALOG.find((v) => v.id === voiceId) ?? null;
}

export async function getVoiceCompatibility(
  voiceId: string,
  targetLanguage?: string
): Promise<VoiceCompatibility> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<VoiceCompatibility>('get_voice_compatibility', { voiceId, targetLanguage });
    } catch {
      // fallback
    }
  }

  const voice = MOCK_CATALOG.find((v) => v.id === voiceId);
  const isLessac = voiceId === 'piper-en-lessac-medium';

  if (targetLanguage && voice) {
    const clean = targetLanguage.trim().toLowerCase();
    const matches =
      voice.language.toLowerCase() === clean ||
      voice.locale.toLowerCase() === clean ||
      voice.locale.toLowerCase().startsWith(`${clean}-`);

    if (!matches) {
      return {
        voiceId,
        status: 'unsupportedLanguage',
        isReady: false,
        isInstalled: isLessac,
        runtimeReady: true,
        supportedLanguages: [voice.language, voice.locale],
        speakerCount: voice.speakerCount,
        warnings: [`Voice does not support requested language '${targetLanguage}'`],
      };
    }
  }

  return {
    voiceId,
    status: isLessac ? 'ready' : 'availableForInstall',
    isReady: isLessac,
    isInstalled: isLessac,
    runtimeReady: true,
    supportedLanguages: voice ? [voice.language, voice.locale] : [],
    speakerCount: voice ? voice.speakerCount : 1,
    warnings: isLessac ? [] : ['Model not installed in mock environment'],
  };
}

export async function startVoicePreview(
  request: VoicePreviewRequest
): Promise<VoicePreviewResult> {
  if (isTauriEnvironment()) {
    return await invoke<VoicePreviewResult>('start_voice_preview', { request });
  }

  const text = request.customText || 'Welcome to Open Media Workbench.';
  return {
    previewId: `mock-preview-${request.voiceId}-${Date.now()}`,
    voiceId: request.voiceId,
    text,
    stagedAudioPath: 'mock-audio.wav',
    sampleRate: 22050,
    channelCount: 1,
    durationUs: 2_170_000,
    cached: false,
    createdAtEpochMs: Date.now(),
  };
}

export async function compareVoices(
  request: VoiceComparisonRequest
): Promise<VoiceComparisonResult> {
  if (isTauriEnvironment()) {
    return await invoke<VoiceComparisonResult>('compare_voices', { request });
  }

  if (request.voiceIdA === request.voiceIdB) {
    throw new Error('Comparison requires two distinct installed voices');
  }

  const text = request.text || 'Welcome to Open Media Workbench.';
  const voiceA = await startVoicePreview({ voiceId: request.voiceIdA, customText: text });
  const voiceB = await startVoicePreview({ voiceId: request.voiceIdB, customText: text });

  return {
    voiceA,
    voiceB,
    durationDiffUs: Number(voiceA.durationUs) - Number(voiceB.durationUs),
  };
}

export async function getVoicePreferences(): Promise<VoicePreferences> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<VoicePreferences>('get_voice_preferences');
    } catch {
      // fallback
    }
  }
  return { ...mockPreferences };
}

export async function toggleVoiceFavorite(voiceId: string): Promise<boolean> {
  if (isTauriEnvironment()) {
    return await invoke<boolean>('toggle_voice_favorite', { voiceId });
  }

  const idx = mockPreferences.favoriteVoiceIds.indexOf(voiceId);
  if (idx >= 0) {
    mockPreferences.favoriteVoiceIds.splice(idx, 1);
    return false;
  } else {
    mockPreferences.favoriteVoiceIds.push(voiceId);
    return true;
  }
}

export async function setPreferredVoice(voiceId?: string | null): Promise<void> {
  if (isTauriEnvironment()) {
    return await invoke<void>('set_preferred_voice', { voiceId: voiceId || null });
  }
  mockPreferences.preferredVoiceId = voiceId || undefined;
}

export async function clearVoicePreviewCache(): Promise<number> {
  if (isTauriEnvironment()) {
    return await invoke<number>('clear_voice_preview_cache');
  }
  return 0;
}

export async function assignVoiceToTrack(
  document: SubtitleDocument,
  trackId: string,
  voiceId: string,
  speakerId?: number
): Promise<SubtitleDocument> {
  if (isTauriEnvironment()) {
    return await invoke<SubtitleDocument>('assign_voice_to_track_command', {
      document,
      trackId,
      voiceId,
      speakerId: speakerId ?? null,
    });
  }

  const existingAssignments: ProjectVoiceAssignments = document.metadata?.voiceAssignments
    ? JSON.parse(document.metadata.voiceAssignments)
    : { trackAssignments: [] };

  existingAssignments.trackAssignments = existingAssignments.trackAssignments.filter(
    (a) => a.trackId !== trackId
  );
  existingAssignments.trackAssignments.push({
    trackId,
    voiceId,
    language: 'en',
    speakerId,
    assignedAtEpochMs: Date.now(),
  });

  return {
    ...document,
    metadata: {
      ...document.metadata,
      voiceAssignments: JSON.stringify(existingAssignments),
    },
    updatedAtEpochMs: Date.now(),
  };
}

export async function removeVoiceAssignment(
  document: SubtitleDocument,
  trackId: string
): Promise<SubtitleDocument> {
  if (isTauriEnvironment()) {
    return await invoke<SubtitleDocument>('remove_voice_assignment_command', {
      document,
      trackId,
    });
  }

  const existingAssignments: ProjectVoiceAssignments = document.metadata?.voiceAssignments
    ? JSON.parse(document.metadata.voiceAssignments)
    : { trackAssignments: [] };

  existingAssignments.trackAssignments = existingAssignments.trackAssignments.filter(
    (a) => a.trackId !== trackId
  );

  return {
    ...document,
    metadata: {
      ...document.metadata,
      voiceAssignments: JSON.stringify(existingAssignments),
    },
    updatedAtEpochMs: Date.now(),
  };
}

export async function setProjectDefaultVoice(
  document: SubtitleDocument,
  voiceId?: string | null
): Promise<SubtitleDocument> {
  if (isTauriEnvironment()) {
    return await invoke<SubtitleDocument>('set_project_default_voice_command', {
      document,
      voiceId: voiceId || null,
    });
  }

  const existingAssignments: ProjectVoiceAssignments = document.metadata?.voiceAssignments
    ? JSON.parse(document.metadata.voiceAssignments)
    : { trackAssignments: [] };

  existingAssignments.defaultVoiceId = voiceId || undefined;

  return {
    ...document,
    metadata: {
      ...document.metadata,
      voiceAssignments: JSON.stringify(existingAssignments),
    },
    updatedAtEpochMs: Date.now(),
  };
}

export function getProjectVoiceAssignments(document: SubtitleDocument): ProjectVoiceAssignments {
  if (document.metadata?.voiceAssignments) {
    try {
      return JSON.parse(document.metadata.voiceAssignments);
    } catch {
      // fallback
    }
  }
  return { trackAssignments: [] };
}
