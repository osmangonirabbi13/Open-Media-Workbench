/**
 * Workspace Domain Types for Open Media Workbench
 * Defines typed workspace identities, capabilities, and persistent/transient states.
 */

export type WorkspaceId = "repair" | "localization";

export type CapabilityStatus = "available" | "in_development" | "planned";

export interface WorkspaceCapability {
  id: string;
  label: string;
  description: string;
  status: CapabilityStatus;
  roadmapStep?: number;
}

export interface WorkspaceDefinition {
  id: WorkspaceId;
  label: string;
  shortLabel: string;
  description: string;
  badge: string;
  capabilities: WorkspaceCapability[];
}

export const WORKSPACE_DEFINITIONS: Record<WorkspaceId, WorkspaceDefinition> = {
  repair: {
    id: "repair",
    label: "Repair Studio",
    shortLabel: "Repair",
    description: "Video corruption diagnosis, MP4/MOV atom reconstruction & stream salvage",
    badge: "Recovery",
    capabilities: [
      {
        id: "media_ingestion",
        label: "Secure Media Ingestion",
        description: "Zero-copy native file dialogs and container probing",
        status: "available",
        roadmapStep: 6,
      },
      {
        id: "corruption_diagnostics",
        label: "Deep Container Diagnostics",
        description: "Atom hierarchy parsing, sample table inspection & PTS/DTS validation",
        status: "planned",
        roadmapStep: 12,
      },
      {
        id: "moov_reconstruction",
        label: "Reference-Based MOOV Repair",
        description: "Reconstruct unfinalized MP4/MOV recordings using a working reference file",
        status: "planned",
        roadmapStep: 13,
      },
      {
        id: "stream_salvage",
        label: "Elementary Stream Salvage",
        description: "Extract uncorrupted audio/video packets from broken bitstreams",
        status: "planned",
        roadmapStep: 14,
      },
      {
        id: "repair_validation",
        label: "A/B Repair Preview Validation",
        description: "Side-by-side synchronized comparison before export",
        status: "planned",
        roadmapStep: 15,
      },
    ],
  },
  localization: {
    id: "localization",
    label: "Localization Studio",
    shortLabel: "Localization",
    description: "Subtitle editor, local AI transcription, translation & voice dubbing",
    badge: "AI Studio",
    capabilities: [
      {
        id: "media_playback",
        label: "Synchronized Video & Audio Preview",
        description: "Custom protocol streaming, proxy engine & frame-accurate seeking",
        status: "available",
        roadmapStep: 7,
      },
      {
        id: "waveform_timeline",
        label: "Multi-LOD Waveform & Canonical EDL",
        description: "Peak/RMS caching and microsecond non-destructive timeline",
        status: "available",
        roadmapStep: 8,
      },
      {
        id: "subtitle_editor",
        label: "Interactive Subtitle Editor",
        description: "Cue list manipulation, timing snapping & SRT/VTT format export",
        status: "planned",
        roadmapStep: 16,
      },
      {
        id: "local_transcription",
        label: "Local Offline Whisper Transcription",
        description: "Privacy-first on-device speech-to-text with word-level timestamps",
        status: "planned",
        roadmapStep: 18,
      },
      {
        id: "local_translation",
        label: "Local Multilingual Translation",
        description: "On-device neural machine translation for subtitle text",
        status: "planned",
        roadmapStep: 22,
      },
      {
        id: "neural_dubbing",
        label: "Neural Voice Dubbing & Sync",
        description: "Offline speech synthesis with automatic audio duration alignment",
        status: "planned",
        roadmapStep: 24,
      },
    ],
  },
};

export interface WorkspaceState {
  activeWorkspace: WorkspaceId;
  selectedAssetIdByWorkspace: Record<WorkspaceId, string | null>;
  repair: {
    activeWorkflowStep: number;
    selectedRecoveryMethod: string | null;
  };
  localization: {
    activeInspectorTab: "subtitles" | "translation" | "dubbing" | "export";
    selectedSourceLanguage: string;
    selectedTargetLanguage: string;
  };
}

export const INITIAL_WORKSPACE_STATE: WorkspaceState = {
  activeWorkspace: "repair",
  selectedAssetIdByWorkspace: {
    repair: null,
    localization: null,
  },
  repair: {
    activeWorkflowStep: 1,
    selectedRecoveryMethod: null,
  },
  localization: {
    activeInspectorTab: "subtitles",
    selectedSourceLanguage: "auto",
    selectedTargetLanguage: "es",
  },
};
