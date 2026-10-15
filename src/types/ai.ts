//! Type definitions for Shared AI Model Manager.

export type AiModelCapability =
  | "speechToText"
  | "textGeneration"
  | "translation"
  | "textToSpeech"
  | "speechAlignment"
  | "speakerDetection"
  | "visionAnalysis";

export type AiModelStatus =
  | "notInstalled"
  | "installing"
  | "installed"
  | "verifying"
  | "ready"
  | "corrupted"
  | "incompatible"
  | "failed"
  | "removing";

export type AiDownloadState =
  | "queued"
  | "downloading"
  | "verifying"
  | "complete"
  | "failed"
  | "cancelled";

export interface AiModelManifest {
  id: string;
  name: string;
  family: string;
  version: string;
  capability: AiModelCapability;
  supportedLanguages: string[];
  runtimeBackend: string;
  runtimeFormat: string;
  sourceRepository: string;
  artifactUrl?: string;
  artifactFilename: string;
  expectedSizeBytes: number;
  expectedSha256?: string;
  licenseName: string;
  licenseUrl: string;
  modelCardUrl: string;
  minRamBytes: number;
  cpuCompatible: boolean;
  gpuRecommended: boolean;
  installable: boolean;
  description: string;
  schemaVersion: number;
}

export interface AiModelInstallation {
  modelId: string;
  installedAt: string;
  installedVersion: string;
  artifactFilename: string;
  actualSizeBytes: number;
  verifiedSha256: string;
  inUse: boolean;
}

export interface AiModelDownloadProgress {
  modelId: string;
  bytesDownloaded: number;
  totalBytes: number;
  progressPct: number;
  state: AiDownloadState;
  error?: string;
}

export interface AiHardwareInfo {
  os: string;
  arch: string;
  logicalCpus: number;
  totalRamBytes?: number;
  availableRamBytes?: number;
  gpuDetected: boolean;
  gpuDeviceName?: string;
  gpuBackendInstalled: boolean;
  gpuRuntimeUsable: boolean;
  storageTotalBytes?: number;
  storageAvailableBytes?: number;
  supportedRuntimeBackends: string[];
}

export interface AiStorageUsage {
  baseDir: string;
  totalModelsSizeBytes: number;
  installedModelsCount: number;
  stagingSizeBytes: number;
  availableDiskBytes?: number;
}

export interface AiModelSummary {
  manifest: AiModelManifest;
  status: AiModelStatus;
  installation?: AiModelInstallation;
  downloadProgress?: AiModelDownloadProgress;
  runtimeAvailable: boolean;
}
