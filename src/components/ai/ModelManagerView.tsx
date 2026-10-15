import React, { useEffect, useState, useCallback } from "react";
import type {
  AiHardwareInfo,
  AiModelSummary,
  AiStorageUsage,
} from "../../types/ai";
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
} from "../../lib/aiManager";
import {
  CheckIcon,
  CloseIcon,
  CpuIcon,
  DownloadIcon,
  ResetIcon,
  ShieldCheckIcon,
  TrashIcon,
} from "../ui/Icons";
import { Button } from "../ui/Button";

interface ModelManagerViewProps {
  onClose?: () => void;
}

export function ModelManagerView({ onClose }: ModelManagerViewProps): React.ReactElement {
  const [models, setModels] = useState<AiModelSummary[]>([]);
  const [hardware, setHardware] = useState<AiHardwareInfo | null>(null);
  const [storage, setStorage] = useState<AiStorageUsage | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [confirmInstallModel, setConfirmInstallModel] = useState<AiModelSummary | null>(null);
  const [confirmRemoveModel, setConfirmRemoveModel] = useState<AiModelSummary | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [verifyingModelId, setVerifyingModelId] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setActionError(null);
      const [modelList, hw, stor] = await Promise.all([
        listAiModels(),
        getAiHardwareInfo(),
        getAiStorageUsage(),
      ]);
      setModels(modelList);
      setHardware(hw);
      setStorage(stor);
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Polling for active downloads
  useEffect(() => {
    const hasActiveDownloads = models.some(
      (m) => m.status === "installing" || m.status === "verifying"
    );
    if (!hasActiveDownloads) return;

    const interval = setInterval(async () => {
      try {
        const [updatedModels, stor] = await Promise.all([
          listAiModels(),
          getAiStorageUsage(),
        ]);
        setModels(updatedModels);
        setStorage(stor);
      } catch {
        // Ignore polling errors
      }
    }, 1500);

    return () => clearInterval(interval);
  }, [models]);

  const handleStartDownload = async (model: AiModelSummary): Promise<void> => {
    try {
      setActionError(null);
      setConfirmInstallModel(null);
      await startAiModelDownload(model.manifest.id);
      await loadData();
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCancelDownload = async (modelId: string): Promise<void> => {
    try {
      setActionError(null);
      await cancelAiModelDownload(modelId);
      await loadData();
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleVerify = async (modelId: string): Promise<void> => {
    try {
      setActionError(null);
      setVerifyingModelId(modelId);
      const ok = await verifyAiModel(modelId);
      if (!ok) {
        setActionError(`Integrity check failed for ${modelId}: Checksum mismatch!`);
      }
      await loadData();
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    } finally {
      setVerifyingModelId(null);
    }
  };

  const handleRemove = async (modelId: string): Promise<void> => {
    try {
      setActionError(null);
      setConfirmRemoveModel(null);
      await removeAiModel(modelId);
      await loadData();
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    }
  };

  const filteredModels = models.filter((m) => {
    if (selectedCategory === "all") return true;
    return m.manifest.capability === selectedCategory;
  });

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-100 overflow-hidden select-none">
      {/* Top Banner / System Hardware & Storage */}
      <div className="shrink-0 p-4 border-b border-zinc-800/80 bg-zinc-900/40">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center space-x-2.5">
            <div className="p-1.5 rounded-lg bg-indigo-500/10 border border-indigo-500/30 text-indigo-400">
              <CpuIcon size={18} />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-zinc-100 flex items-center space-x-2">
                <span>AI Model Manager</span>
                <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400">
                  100% Offline & Shared
                </span>
              </h2>
              <p className="text-xs text-zinc-400 mt-0.5">
                Shared local AI weights for Localization and AI Shorts workspaces. Zero telemetry or external inference.
              </p>
            </div>
          </div>
          <div className="flex items-center space-x-2">
            <Button
              variant="subtle"
              size="sm"
              onClick={loadData}
              disabled={loading}
              title="Refresh models and storage"
            >
              <ResetIcon size={14} className={loading ? "animate-spin" : ""} />
            </Button>
            {onClose && (
              <Button variant="subtle" size="sm" onClick={onClose}>
                <CloseIcon size={14} />
              </Button>
            )}
          </div>
        </div>

        {/* System Stats Bar */}
        <div className="grid grid-cols-4 gap-2.5 pt-2 border-t border-zinc-800/50 text-xs">
          <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800/80">
            <div className="text-[11px] text-zinc-500">Managed AI Storage</div>
            <div className="font-semibold text-zinc-200 mt-0.5">
              {formatBytes(storage?.totalModelsSizeBytes ?? 0)}
              <span className="text-zinc-500 font-normal ml-1">
                ({storage?.installedModelsCount ?? 0} installed)
              </span>
            </div>
          </div>

          <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800/80">
            <div className="text-[11px] text-zinc-500">Available Disk Space</div>
            <div className="font-semibold text-zinc-200 mt-0.5">
              {formatBytes(storage?.availableDiskBytes ?? 0)}
            </div>
          </div>

          <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800/80">
            <div className="text-[11px] text-zinc-500">System Memory</div>
            <div className="font-semibold text-zinc-200 mt-0.5">
              {hardware?.totalRamBytes ? formatBytes(hardware.totalRamBytes) : "Unknown"}
              <span className="text-zinc-500 font-normal ml-1">
                ({hardware?.logicalCpus ?? 1} CPUs)
              </span>
            </div>
          </div>

          <div className="p-2 rounded-lg bg-zinc-900/80 border border-zinc-800/80">
            <div className="text-[11px] text-zinc-500">GPU Acceleration</div>
            <div className="font-semibold text-zinc-200 mt-0.5 truncate" title={hardware?.gpuDeviceName ?? "None"}>
              {hardware?.gpuDetected ? hardware.gpuDeviceName ?? "Detected" : "CPU Fallback"}
            </div>
          </div>
        </div>
      </div>

      {/* Global Notice */}
      <div className="px-4 py-2 bg-indigo-950/20 border-b border-indigo-900/30 flex items-center justify-between text-xs text-indigo-300">
        <div className="flex items-center space-x-2">
          <ShieldCheckIcon size={14} className="text-indigo-400 shrink-0" />
          <span>
            AI models are <strong>optional</strong>. Model downloads require explicit consent. No models are required for normal media repair or subtitle editing.
          </span>
        </div>
      </div>

      {/* Error alert if any */}
      {actionError && (
        <div className="mx-4 mt-3 p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/30 text-rose-300 text-xs flex items-center justify-between">
          <span>{actionError}</span>
          <button
            type="button"
            onClick={() => setActionError(null)}
            className="text-rose-400 hover:text-rose-200 ml-2"
          >
            <CloseIcon size={14} />
          </button>
        </div>
      )}

      {/* Category Filter Tabs */}
      <div className="px-4 py-2.5 border-b border-zinc-800/60 bg-zinc-900/20 flex items-center space-x-1.5 overflow-x-auto">
        {[
          { id: "all", label: "All Models" },
          { id: "speechToText", label: "Speech to Text" },
          { id: "textGeneration", label: "Local LLM / Text" },
          { id: "translation", label: "Translation" },
          { id: "textToSpeech", label: "Voice / TTS" },
        ].map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setSelectedCategory(tab.id)}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors whitespace-nowrap ${
              selectedCategory === tab.id
                ? "bg-zinc-800 text-zinc-100 border border-zinc-700 shadow-xs"
                : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Models Grid / List */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {filteredModels.length === 0 ? (
          <div className="text-center py-12 text-zinc-500 text-xs">
            No models found in this category.
          </div>
        ) : (
          filteredModels.map((item) => {
            const statusBadge = formatStatusBadge(item.status);
            const isInstalled = item.status === "installed" || item.status === "ready";
            const isDownloading = item.status === "installing";
            const isVerifying = item.status === "verifying" || verifyingModelId === item.manifest.id;

            return (
              <div
                key={item.manifest.id}
                className="p-3.5 rounded-xl border border-zinc-800 bg-zinc-900/50 hover:border-zinc-700/80 transition-all flex flex-col space-y-3"
              >
                {/* Header row */}
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <div className="flex items-center space-x-2">
                      <h3 className="text-sm font-semibold text-zinc-100 tracking-tight">
                        {item.manifest.name}
                      </h3>
                      <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700">
                        {item.manifest.family}
                      </span>
                      <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-zinc-800 text-zinc-300">
                        {formatCapabilityLabel(item.manifest.capability)}
                      </span>
                    </div>
                    <p className="text-xs text-zinc-400 leading-relaxed">
                      {item.manifest.description}
                    </p>
                  </div>

                  {/* Status Badge */}
                  <div className="flex items-center space-x-2 shrink-0">
                    <span
                      className={`text-xs px-2.5 py-1 rounded-full font-medium border ${statusBadge.bg} ${statusBadge.text} ${statusBadge.border}`}
                    >
                      {isVerifying ? "Verifying..." : statusBadge.label}
                    </span>
                  </div>
                </div>

                {/* Metadata details row */}
                <div className="grid grid-cols-4 gap-2 text-[11px] text-zinc-400 py-1.5 border-t border-b border-zinc-800/40">
                  <div>
                    <span className="text-zinc-500">Artifact Size:</span>{" "}
                    <span className="text-zinc-200 font-medium">
                      {formatBytes(item.manifest.expectedSizeBytes)}
                    </span>
                  </div>
                  <div>
                    <span className="text-zinc-500">License:</span>{" "}
                    <span className="text-zinc-200 font-medium">
                      {item.manifest.licenseName}
                    </span>
                  </div>
                  <div>
                    <span className="text-zinc-500">Backend:</span>{" "}
                    <span className="text-zinc-200 font-medium">
                      {item.manifest.runtimeBackend} ({item.manifest.runtimeFormat.toUpperCase()})
                    </span>
                  </div>
                  <div>
                    <span className="text-zinc-500">RAM Rec:</span>{" "}
                    <span className="text-zinc-200 font-medium">
                      {formatBytes(item.manifest.minRamBytes)}
                    </span>
                  </div>
                </div>

                {/* Progress bar if downloading */}
                {isDownloading && item.downloadProgress && (
                  <div className="space-y-1.5 p-2 rounded-lg bg-zinc-950 border border-zinc-800">
                    <div className="flex justify-between text-xs text-zinc-400">
                      <span>Downloading artifact...</span>
                      <span>
                        {formatBytes(item.downloadProgress.bytesDownloaded)} /{" "}
                        {formatBytes(item.downloadProgress.totalBytes)} (
                        {item.downloadProgress.progressPct.toFixed(1)}%)
                      </span>
                    </div>
                    <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-indigo-500 transition-all duration-300"
                        style={{ width: `${item.downloadProgress.progressPct}%` }}
                      />
                    </div>
                  </div>
                )}

                {/* Actions row */}
                <div className="flex items-center justify-between pt-1">
                  <div className="text-[11px] text-zinc-500 flex items-center space-x-1.5">
                    <span>Source:</span>
                    <span className="text-zinc-400 truncate max-w-xs" title={item.manifest.sourceRepository}>
                      {item.manifest.sourceRepository.replace("https://", "")}
                    </span>
                  </div>

                  <div className="flex items-center space-x-2">
                    {/* Catalog-only / unavailable */}
                    {!item.manifest.installable && (
                      <Button variant="subtle" size="sm" disabled>
                        Catalog Only (Runtime Pending)
                      </Button>
                    )}

                    {/* Installable & Not Installed */}
                    {item.manifest.installable && !isInstalled && !isDownloading && !isVerifying && (
                      <Button
                        variant="primary"
                        size="sm"
                        onClick={() => setConfirmInstallModel(item)}
                      >
                        <DownloadIcon size={14} className="mr-1.5" />
                        Install Model
                      </Button>
                    )}

                    {/* Downloading -> Cancel */}
                    {isDownloading && (
                      <Button
                        variant="danger"
                        size="sm"
                        onClick={() => handleCancelDownload(item.manifest.id)}
                      >
                        Cancel Download
                      </Button>
                    )}

                    {/* Installed actions: Verify & Remove */}
                    {isInstalled && (
                      <>
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => handleVerify(item.manifest.id)}
                          disabled={isVerifying}
                          title="Verify cryptographic SHA-256 against trusted catalog"
                        >
                          <ShieldCheckIcon size={14} className="mr-1.5 text-emerald-400" />
                          Verify Integrity
                        </Button>
                        <Button
                          variant="danger"
                          size="sm"
                          onClick={() => setConfirmRemoveModel(item)}
                          title="Remove model file from shared disk storage"
                        >
                          <TrashIcon size={14} className="mr-1.5" />
                          Remove
                        </Button>
                      </>
                    )}
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Install Confirmation Modal */}
      {confirmInstallModel && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-xs select-none">
          <div className="w-full max-w-md rounded-xl border border-zinc-800 bg-zinc-950 p-5 shadow-2xl flex flex-col space-y-4">
            <div className="flex items-center justify-between border-b border-zinc-800 pb-3">
              <h3 className="text-sm font-semibold text-zinc-100 flex items-center space-x-2">
                <DownloadIcon size={16} className="text-indigo-400" />
                <span>Confirm Model Download</span>
              </h3>
              <button
                type="button"
                onClick={() => setConfirmInstallModel(null)}
                className="text-zinc-400 hover:text-zinc-200"
              >
                <CloseIcon size={14} />
              </button>
            </div>

            <div className="text-xs text-zinc-300 space-y-2.5">
              <p>
                You are about to download <strong>{confirmInstallModel.manifest.name}</strong> to local shared storage.
              </p>
              <div className="p-3 rounded-lg bg-zinc-900 border border-zinc-800 space-y-1.5 text-[11px]">
                <div className="flex justify-between">
                  <span className="text-zinc-500">Download Size:</span>
                  <span className="text-zinc-200 font-semibold">
                    {formatBytes(confirmInstallModel.manifest.expectedSizeBytes)}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-zinc-500">Available Disk Space:</span>
                  <span className="text-zinc-200 font-semibold">
                    {formatBytes(storage?.availableDiskBytes ?? 0)}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-zinc-500">Model License:</span>
                  <span className="text-zinc-200 font-semibold">
                    {confirmInstallModel.manifest.licenseName}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span className="text-zinc-500">Execution Backend:</span>
                  <span className="text-zinc-200 font-semibold">
                    {confirmInstallModel.manifest.runtimeBackend} (Offline)
                  </span>
                </div>
              </div>
              <p className="text-[11px] text-zinc-400">
                Notice: The model artifact is downloaded over HTTPS directly from verified upstream repository ({confirmInstallModel.manifest.sourceRepository}). SHA-256 integrity check will be performed immediately upon completion.
              </p>
            </div>

            <div className="flex items-center justify-end space-x-2 pt-2 border-t border-zinc-800">
              <Button
                variant="subtle"
                size="sm"
                onClick={() => setConfirmInstallModel(null)}
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={() => handleStartDownload(confirmInstallModel)}
              >
                <CheckIcon size={14} className="mr-1.5" />
                Confirm & Download
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Remove Confirmation Modal */}
      {confirmRemoveModel && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-xs select-none">
          <div className="w-full max-w-sm rounded-xl border border-zinc-800 bg-zinc-950 p-5 shadow-2xl flex flex-col space-y-4">
            <div className="flex items-center justify-between border-b border-zinc-800 pb-3">
              <h3 className="text-sm font-semibold text-rose-400 flex items-center space-x-2">
                <TrashIcon size={16} />
                <span>Remove Model</span>
              </h3>
              <button
                type="button"
                onClick={() => setConfirmRemoveModel(null)}
                className="text-zinc-400 hover:text-zinc-200"
              >
                <CloseIcon size={14} />
              </button>
            </div>

            <p className="text-xs text-zinc-300">
              Are you sure you want to remove <strong>{confirmRemoveModel.manifest.name}</strong> from shared storage? This will delete the downloaded weight files and reclaim disk space.
            </p>

            <div className="flex items-center justify-end space-x-2 pt-2 border-t border-zinc-800">
              <Button
                variant="subtle"
                size="sm"
                onClick={() => setConfirmRemoveModel(null)}
              >
                Cancel
              </Button>
              <Button
                variant="danger"
                size="sm"
                onClick={() => handleRemove(confirmRemoveModel.manifest.id)}
              >
                Delete Files
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
