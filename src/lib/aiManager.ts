//! Client bridge, mock fallbacks, and utilities for Shared AI Model Manager.

import { invoke } from "@tauri-apps/api/core";
import type {
  AiHardwareInfo,
  AiModelCapability,
  AiModelDownloadProgress,
  AiModelManifest,
  AiModelStatus,
  AiModelSummary,
  AiStorageUsage,
} from "../types/ai";

export const MOCK_CATALOG: AiModelManifest[] = [
  {
    id: "whisper-tiny",
    name: "Whisper Tiny (Multilingual)",
    family: "whisper",
    version: "v1",
    capability: "speechToText",
    supportedLanguages: ["en", "bn", "es", "fr", "de", "hi", "ja", "zh", "ar", "ru"],
    runtimeBackend: "whisper.cpp",
    runtimeFormat: "ggml",
    sourceRepository: "https://huggingface.co/ggerganov/whisper.cpp",
    artifactUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    artifactFilename: "ggml-tiny.bin",
    expectedSizeBytes: 77691713,
    expectedSha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    licenseName: "MIT",
    licenseUrl: "https://huggingface.co/openai/whisper/blob/main/LICENSE",
    modelCardUrl: "https://huggingface.co/openai/whisper-tiny",
    minRamBytes: 1073741824,
    cpuCompatible: true,
    gpuRecommended: false,
    installable: true,
    description: "Fast, lightweight multilingual speech recognition model for quick draft subtitles.",
    schemaVersion: 1,
  },
  {
    id: "whisper-base",
    name: "Whisper Base (Multilingual)",
    family: "whisper",
    version: "v1",
    capability: "speechToText",
    supportedLanguages: ["en", "bn", "es", "fr", "de", "hi", "ja", "zh", "ar", "ru"],
    runtimeBackend: "whisper.cpp",
    runtimeFormat: "ggml",
    sourceRepository: "https://huggingface.co/ggerganov/whisper.cpp",
    artifactUrl: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    artifactFilename: "ggml-base.bin",
    expectedSizeBytes: 147951465,
    expectedSha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    licenseName: "MIT",
    licenseUrl: "https://huggingface.co/openai/whisper/blob/main/LICENSE",
    modelCardUrl: "https://huggingface.co/openai/whisper-base",
    minRamBytes: 2147483648,
    cpuCompatible: true,
    gpuRecommended: false,
    installable: true,
    description: "Balanced accuracy and speed for multilingual speech-to-text in production.",
    schemaVersion: 1,
  },
  {
    id: "piper-en-lessac-medium",
    name: "Piper Voice (English - Lessac Medium)",
    family: "piper",
    version: "v1.0.0",
    capability: "textToSpeech",
    supportedLanguages: ["en-US"],
    runtimeBackend: "piper",
    runtimeFormat: "onnx",
    sourceRepository: "https://huggingface.co/rhasspy/piper-voices",
    artifactUrl: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx",
    artifactFilename: "en_US-lessac-medium.onnx",
    expectedSizeBytes: 63201294,
    expectedSha256: "5efe09e69902187827af646e1a6e9d269dee769f9877d17b16b1b46eeaaf019f",
    licenseName: "Public Domain",
    licenseUrl: "https://github.com/rhasspy/piper/blob/master/VOICES.md",
    modelCardUrl: "https://huggingface.co/rhasspy/piper-voices",
    minRamBytes: 1073741824,
    cpuCompatible: true,
    gpuRecommended: false,
    installable: true,
    description: "High-quality neural text-to-speech voice for video localization and voiceover.",
    schemaVersion: 1,
  },
  {
    id: "qwen2.5-0.5b-instruct",
    name: "Qwen 2.5 0.5B Instruct (GGUF Q4_K_M)",
    family: "qwen",
    version: "v2.5",
    capability: "textGeneration",
    supportedLanguages: ["en", "zh", "es", "fr", "de", "bn", "ru", "ja"],
    runtimeBackend: "llama.cpp",
    runtimeFormat: "gguf",
    sourceRepository: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF",
    artifactUrl: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
    artifactFilename: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
    expectedSizeBytes: 491400032,
    expectedSha256: "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db",
    licenseName: "Apache-2.0",
    licenseUrl: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/blob/main/LICENSE",
    modelCardUrl: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct",
    minRamBytes: 2147483648,
    cpuCompatible: true,
    gpuRecommended: false,
    installable: true,
    description: "Compact quantized language model for offline metadata, title, and description generation.",
    schemaVersion: 1,
  },
  {
    id: "opus-mt-en-es",
    name: "Opus-MT English to Spanish (Catalog Only)",
    family: "opus-mt",
    version: "v1.0",
    capability: "translation",
    supportedLanguages: ["en", "es"],
    runtimeBackend: "marian",
    runtimeFormat: "onnx",
    sourceRepository: "https://huggingface.co/Helsinki-NLP/opus-mt-en-es",
    artifactFilename: "opus-mt-en-es.onnx",
    expectedSizeBytes: 312000000,
    licenseName: "CC-BY-4.0",
    licenseUrl: "https://huggingface.co/Helsinki-NLP/opus-mt-en-es",
    modelCardUrl: "https://huggingface.co/Helsinki-NLP/opus-mt-en-es",
    minRamBytes: 2147483648,
    cpuCompatible: true,
    gpuRecommended: false,
    installable: false,
    description: "Catalog entry for neural machine translation. Installation disabled pending Marian runtime integration.",
    schemaVersion: 1,
  },
];

const mockHardware: AiHardwareInfo = {
  os: "linux",
  arch: "x86_64",
  logicalCpus: 8,
  totalRamBytes: 16 * 1024 * 1024 * 1024,
  availableRamBytes: 8 * 1024 * 1024 * 1024,
  gpuDetected: false,
  gpuBackendInstalled: false,
  gpuRuntimeUsable: false,
  storageTotalBytes: 500 * 1024 * 1024 * 1024,
  storageAvailableBytes: 120 * 1024 * 1024 * 1024,
  supportedRuntimeBackends: [],
};

let mockInstalledModels: Set<string> = new Set();

function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function listAiModels(): Promise<AiModelSummary[]> {
  if (isTauriEnvironment()) {
    return invoke<AiModelSummary[]>("list_ai_models");
  }
  // Mock browser/test implementation
  return MOCK_CATALOG.map((manifest) => ({
    manifest,
    status: mockInstalledModels.has(manifest.id) ? "installed" : "notInstalled",
    installation: mockInstalledModels.has(manifest.id)
      ? {
          modelId: manifest.id,
          installedAt: "2026-10-15T00:00:00Z",
          installedVersion: manifest.version,
          artifactFilename: manifest.artifactFilename,
          actualSizeBytes: manifest.expectedSizeBytes,
          verifiedSha256: manifest.expectedSha256 ?? "mockhash",
          inUse: false,
        }
      : undefined,
    runtimeAvailable: false,
  }));
}

export async function getAiModelStatus(modelId: string): Promise<AiModelSummary> {
  if (isTauriEnvironment()) {
    return invoke<AiModelSummary>("get_ai_model_status", { modelId });
  }
  const manifest = MOCK_CATALOG.find((m) => m.id === modelId);
  if (!manifest) throw new Error(`Model not found: ${modelId}`);
  return {
    manifest,
    status: mockInstalledModels.has(manifest.id) ? "installed" : "notInstalled",
    runtimeAvailable: false,
  };
}

export async function getAiHardwareInfo(): Promise<AiHardwareInfo> {
  if (isTauriEnvironment()) {
    return invoke<AiHardwareInfo>("get_ai_hardware_info");
  }
  return mockHardware;
}

export async function startAiModelDownload(modelId: string): Promise<AiModelDownloadProgress> {
  if (isTauriEnvironment()) {
    return invoke<AiModelDownloadProgress>("start_ai_model_download", { modelId });
  }
  mockInstalledModels.add(modelId);
  return {
    modelId,
    bytesDownloaded: 100,
    totalBytes: 100,
    progressPct: 100,
    state: "complete",
  };
}

export async function getAiModelDownloadStatus(
  modelId: string
): Promise<AiModelDownloadProgress | null> {
  if (isTauriEnvironment()) {
    return invoke<AiModelDownloadProgress | null>("get_ai_model_download_status", { modelId });
  }
  return null;
}

export async function cancelAiModelDownload(modelId: string): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("cancel_ai_model_download", { modelId });
  }
  mockInstalledModels.delete(modelId);
}

export async function verifyAiModel(modelId: string): Promise<boolean> {
  if (isTauriEnvironment()) {
    return invoke<boolean>("verify_ai_model", { modelId });
  }
  return mockInstalledModels.has(modelId);
}

export async function removeAiModel(modelId: string): Promise<void> {
  if (isTauriEnvironment()) {
    return invoke<void>("remove_ai_model", { modelId });
  }
  mockInstalledModels.delete(modelId);
}

export async function getAiStorageUsage(): Promise<AiStorageUsage> {
  if (isTauriEnvironment()) {
    return invoke<AiStorageUsage>("get_ai_storage_usage");
  }
  let totalBytes = 0;
  for (const id of mockInstalledModels) {
    const m = MOCK_CATALOG.find((item) => item.id === id);
    if (m) totalBytes += m.expectedSizeBytes;
  }
  return {
    baseDir: "~/.local/share/open-media-workbench/ai-models",
    totalModelsSizeBytes: totalBytes,
    installedModelsCount: mockInstalledModels.size,
    stagingSizeBytes: 0,
    availableDiskBytes: mockHardware.storageAvailableBytes,
  };
}

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const val = bytes / Math.pow(1024, i);
  return `${val.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatCapabilityLabel(cap: AiModelCapability): string {
  switch (cap) {
    case "speechToText":
      return "Speech to Text";
    case "textGeneration":
      return "Local LLM / Text";
    case "translation":
      return "Neural Translation";
    case "textToSpeech":
      return "Text to Speech (TTS)";
    case "speechAlignment":
      return "Speech Alignment";
    case "speakerDetection":
      return "Speaker Diarization";
    case "visionAnalysis":
      return "Vision Analysis";
  }
}

export function formatStatusBadge(status: AiModelStatus): {
  label: string;
  bg: string;
  text: string;
  border: string;
  description: string;
} {
  switch (status) {
    case "installed":
      return {
        label: "Installed (Runtime Pending)",
        bg: "bg-emerald-500/10",
        text: "text-emerald-400",
        border: "border-emerald-500/30",
        description: "Model weights verified on disk. Execution engine planned for future step.",
      };
    case "ready":
      return {
        label: "Ready for Inference",
        bg: "bg-teal-500/10",
        text: "text-teal-400",
        border: "border-teal-500/30",
        description: "Installed and inference engine operational.",
      };
    case "installing":
      return {
        label: "Downloading",
        bg: "bg-indigo-500/10",
        text: "text-indigo-400",
        border: "border-indigo-500/30",
        description: "Streaming artifact from verified upstream repository.",
      };
    case "verifying":
      return {
        label: "Verifying SHA-256",
        bg: "bg-amber-500/10",
        text: "text-amber-400",
        border: "border-amber-500/30",
        description: "Performing cryptographic integrity checksum.",
      };
    case "corrupted":
      return {
        label: "Corrupted / Missing",
        bg: "bg-rose-500/10",
        text: "text-rose-400",
        border: "border-rose-500/30",
        description: "Artifact file is missing or checksum failed.",
      };
    case "failed":
      return {
        label: "Download Failed",
        bg: "bg-red-500/10",
        text: "text-red-400",
        border: "border-red-500/30",
        description: "Network or storage failure.",
      };
    case "notInstalled":
    default:
      return {
        label: "Available to Download",
        bg: "bg-zinc-800/40",
        text: "text-zinc-400",
        border: "border-zinc-700/50",
        description: "Not installed. User consent required to download.",
      };
  }
}
