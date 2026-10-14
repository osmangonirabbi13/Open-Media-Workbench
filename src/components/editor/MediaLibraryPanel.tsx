import React, { useEffect, useState, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  FilmIcon,
  PlusIcon,
  TrashIcon,
  UploadIcon,
  InfoIcon,
  CloseIcon,
} from "../ui/Icons";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import {
  pickAndImportFiles,
  importFilesByPath,
  getMediaLibraryAssets,
  removeMediaLibraryAsset,
} from "../../lib/mediaImport";
import { isTauriEnvironment } from "../../lib/tauri";
import type { MediaAsset, ImportBatchResult } from "../../types/media";
import {
  formatDurationMicros,
  formatFileSize,
  formatFps,
} from "../../types/media";
import { MediaDetailsModal } from "./MediaDetailsModal";

export function MediaLibraryPanel(): React.ReactElement {
  const [assets, setAssets] = useState<MediaAsset[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [isDragging, setIsDragging] = useState<boolean>(false);
  const [selectedAsset, setSelectedAsset] = useState<MediaAsset | null>(null);
  const [detailsAsset, setDetailsAsset] = useState<MediaAsset | null>(null);
  const [errorBanner, setErrorBanner] = useState<string | null>(null);
  const [noticeBanner, setNoticeBanner] = useState<string | null>(null);

  // Load existing session assets on initial mount
  useEffect(() => {
    let isMounted = true;
    async function loadInitialAssets() {
      try {
        const loaded = await getMediaLibraryAssets();
        if (isMounted) {
          setAssets(loaded);
        }
      } catch (err) {
        console.error("Failed to load initial media assets:", err);
      }
    }
    loadInitialAssets();
    return () => {
      isMounted = false;
    };
  }, []);

  const handleImportResult = useCallback((result: ImportBatchResult) => {
    if (result.succeeded.length > 0) {
      setAssets((prev) => {
        const map = new Map(prev.map((a) => [a.id, a]));
        for (const asset of result.succeeded) {
          map.set(asset.id, asset);
        }
        return Array.from(map.values());
      });
      setSelectedAsset(result.succeeded[0]);
    }

    const notices: string[] = [];
    if (result.duplicates.length > 0) {
      notices.push(
        `${result.duplicates.length} duplicate file(s) already in library: ${result.duplicates.join(", ")}`
      );
    }
    if (result.failed.length > 0) {
      const failedMsg = result.failed
        .map((f) => `${f.fileName}: ${f.errorMessage}`)
        .join("; ");
      setErrorBanner(`Failed to import: ${failedMsg}`);
    } else {
      setErrorBanner(null);
    }

    if (notices.length > 0) {
      setNoticeBanner(notices.join(" | "));
    } else {
      setNoticeBanner(null);
    }
  }, []);

  // Handle native file selection dialog
  const handleOpenPicker = async () => {
    setErrorBanner(null);
    setNoticeBanner(null);
    setIsLoading(true);
    try {
      const result = await pickAndImportFiles();
      handleImportResult(result);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorBanner(msg);
    } finally {
      setIsLoading(false);
    }
  };

  // Handle file path ingestion (from native drop)
  const handleImportPaths = useCallback(
    async (paths: string[]) => {
      setErrorBanner(null);
      setNoticeBanner(null);
      setIsLoading(true);
      try {
        const result = await importFilesByPath(paths);
        handleImportResult(result);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setErrorBanner(msg);
      } finally {
        setIsLoading(false);
      }
    },
    [handleImportResult]
  );

  // Setup Tauri native window drag & drop event listener
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    if (isTauriEnvironment()) {
      getCurrentWindow()
        .onDragDropEvent((event) => {
          if (event.payload.type === "enter" || event.payload.type === "over") {
            setIsDragging(true);
          } else if (event.payload.type === "leave") {
            setIsDragging(false);
          } else if (event.payload.type === "drop") {
            setIsDragging(false);
            const droppedPaths = event.payload.paths;
            if (droppedPaths && droppedPaths.length > 0) {
              handleImportPaths(droppedPaths);
            }
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch((err) => {
          console.warn("Could not register window drag drop listener:", err);
        });
    }

    return () => {
      if (unlisten) unlisten();
    };
  }, [handleImportPaths]);

  // Handle item removal
  const handleRemove = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      const removed = await removeMediaLibraryAsset(id);
      if (removed) {
        setAssets((prev) => prev.filter((a) => a.id !== id));
        if (selectedAsset?.id === id) {
          setSelectedAsset(null);
        }
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorBanner(`Failed to remove asset: ${msg}`);
    }
  };

  return (
    <section
      aria-label="Media Library"
      className="relative flex flex-col h-full bg-zinc-950/60 border-r border-zinc-800/80 overflow-hidden select-none"
      onDragOver={(e) => {
        e.preventDefault();
        setIsDragging(true);
      }}
      onDragLeave={() => setIsDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setIsDragging(false);
      }}
    >
      {/* Panel Header */}
      <div className="h-9 shrink-0 flex items-center justify-between px-3 border-b border-zinc-800/70 bg-zinc-900/40">
        <div className="flex items-center space-x-2">
          <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
            Media Library
          </span>
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-zinc-800 text-zinc-400">
            {assets.length} {assets.length === 1 ? "item" : "items"}
          </span>
        </div>

        <button
          type="button"
          onClick={handleOpenPicker}
          disabled={isLoading}
          title="Import Media Files (MP4, MOV, MKV, WebM)"
          className="p-1 text-zinc-300 hover:text-white bg-zinc-800/80 hover:bg-zinc-700/80 rounded transition-colors"
        >
          <PlusIcon size={14} />
        </button>
      </div>

      {/* Warning/Notice Banners */}
      {errorBanner && (
        <div className="p-2.5 bg-rose-950/70 border-b border-rose-800/70 text-rose-200 text-xs flex items-start justify-between">
          <div className="flex items-start space-x-2 truncate">
            <span className="font-semibold shrink-0">Error:</span>
            <span className="truncate">{errorBanner}</span>
          </div>
          <button
            type="button"
            onClick={() => setErrorBanner(null)}
            className="text-rose-400 hover:text-rose-100 ml-2"
          >
            <CloseIcon size={14} />
          </button>
        </div>
      )}

      {noticeBanner && (
        <div className="p-2.5 bg-amber-950/60 border-b border-amber-800/60 text-amber-200 text-xs flex items-start justify-between">
          <div className="flex items-start space-x-2 truncate">
            <span className="font-semibold shrink-0">Notice:</span>
            <span className="truncate">{noticeBanner}</span>
          </div>
          <button
            type="button"
            onClick={() => setNoticeBanner(null)}
            className="text-amber-400 hover:text-amber-100 ml-2"
          >
            <CloseIcon size={14} />
          </button>
        </div>
      )}

      {/* Loading Bar */}
      {isLoading && (
        <div className="flex items-center justify-center space-x-2 py-2 px-3 bg-indigo-950/50 border-b border-indigo-800/60 text-indigo-300 text-xs animate-pulse">
          <div className="w-3 h-3 border-2 border-indigo-400 border-t-transparent rounded-full animate-spin" />
          <span>Probing media metadata with FFprobe...</span>
        </div>
      )}

      {/* Drag & Drop Visual Overlay */}
      {isDragging && (
        <div className="absolute inset-0 z-20 flex flex-col items-center justify-center bg-indigo-950/80 backdrop-blur-xs border-2 border-dashed border-indigo-400 m-2 rounded-xl text-center pointer-events-none animate-in fade-in duration-150">
          <div className="p-3 bg-indigo-500/20 text-indigo-300 rounded-full mb-2">
            <UploadIcon size={28} />
          </div>
          <p className="text-sm font-semibold text-white">Drop Media Files to Ingest</p>
          <p className="text-xs text-indigo-200 mt-1">MP4, MOV, MKV, WebM</p>
        </div>
      )}

      {/* Content Area */}
      <div className="flex-1 overflow-y-auto p-3">
        {assets.length === 0 && !isLoading ? (
          <div className="h-full flex items-center justify-center">
            <EmptyState
              icon={<FilmIcon size={24} />}
              badge="Milestone 2 &bull; Step 6"
              title="No Media Imported"
              description="Select or drag local video files to inspect container format, video/audio stream codecs, and editing compatibility."
              action={
                <Button
                  variant="primary"
                  size="sm"
                  onClick={handleOpenPicker}
                  icon={<PlusIcon size={14} />}
                >
                  Import Media
                </Button>
              }
            />
          </div>
        ) : (
          <div className="space-y-2">
            {assets.map((asset) => {
              const isSelected = selectedAsset?.id === asset.id;
              const primaryVideo = asset.metadata.videoStreams[0];
              const primaryAudio = asset.metadata.audioStreams[0];
              const isSupported = asset.compatibility.overall === "supported";
              const isLimited = asset.compatibility.overall === "supportedWithLimitations";

              return (
                <div
                  key={asset.id}
                  onClick={() => setSelectedAsset(asset)}
                  className={`group relative p-2.5 rounded-lg border transition-all cursor-pointer ${
                    isSelected
                      ? "bg-zinc-800/80 border-indigo-500/80 shadow-md shadow-indigo-950/30"
                      : "bg-zinc-900/60 hover:bg-zinc-900 border-zinc-800/80 hover:border-zinc-700"
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex items-start space-x-2.5 min-w-0">
                      <div className="p-2 rounded bg-zinc-800 text-zinc-400 shrink-0 mt-0.5 group-hover:text-zinc-200 transition-colors">
                        <FilmIcon size={16} />
                      </div>
                      <div className="min-w-0 truncate">
                        <p className="text-xs font-medium text-zinc-100 truncate" title={asset.fileName}>
                          {asset.fileName}
                        </p>
                        <p className="text-[11px] text-zinc-400 mt-0.5">
                          {formatDurationMicros(asset.metadata.container.durationMicros)} &bull;{" "}
                          {formatFileSize(asset.source.fileSize)}
                        </p>
                        <div className="flex items-center space-x-1.5 mt-1.5 text-[10px]">
                          {primaryVideo && (
                            <span className="px-1.5 py-0.2 rounded bg-zinc-800 text-zinc-300 font-mono">
                              {primaryVideo.width}&times;{primaryVideo.height} &bull;{" "}
                              {primaryVideo.codecName.toUpperCase()} &bull;{" "}
                              {formatFps(primaryVideo.averageFrameRate)}
                            </span>
                          )}
                          {primaryAudio && (
                            <span className="px-1.5 py-0.2 rounded bg-zinc-800 text-zinc-400 font-mono">
                              {primaryAudio.codecName.toUpperCase()}
                            </span>
                          )}
                        </div>
                      </div>
                    </div>

                    {/* Actions and Status Badge */}
                    <div className="flex flex-col items-end space-y-1.5 shrink-0 ml-2">
                      <div className="flex items-center space-x-1">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            setDetailsAsset(asset);
                          }}
                          title="View Technical Metadata Details"
                          className="p-1 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 rounded transition-colors"
                        >
                          <InfoIcon size={14} />
                        </button>
                        <button
                          type="button"
                          onClick={(e) => handleRemove(asset.id, e)}
                          title="Remove from project library (original file is NOT deleted)"
                          className="p-1 text-zinc-500 hover:text-rose-400 hover:bg-rose-950/30 rounded transition-colors"
                        >
                          <TrashIcon size={14} />
                        </button>
                      </div>

                      {/* Compatibility Indicator */}
                      <span
                        className={`px-1.5 py-0.2 rounded text-[10px] font-semibold border ${
                          isSupported
                            ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/30"
                            : isLimited
                              ? "bg-amber-500/10 text-amber-400 border-amber-500/30"
                              : "bg-rose-500/10 text-rose-400 border-rose-500/30"
                        }`}
                      >
                        {isSupported ? "Ready" : isLimited ? "Limitation" : "Unsupported"}
                      </span>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Metadata Details Modal */}
      <MediaDetailsModal
        asset={detailsAsset}
        onClose={() => setDetailsAsset(null)}
      />
    </section>
  );
}
