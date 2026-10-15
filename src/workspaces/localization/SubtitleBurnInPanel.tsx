import React, { useEffect, useState } from "react";
import type { MediaAsset } from "../../types/media";
import type { SubtitleTrack } from "../../types/subtitles";
import type {
  ExportResolution,
  SubtitleBurnInRequest,
  SubtitleBurnInStatus,
  SubtitleExportProfile,
  SubtitleStyle,
} from "../../types/styling";
import {
  cancelSubtitleBurnInExport,
  finalizeBurnInExport,
  getSubtitleBurnInStatus,
  startSubtitleBurnInExport,
} from "../../lib/styling";
import { Button } from "../../components/ui/Button";
import {
  CheckIcon,
  DownloadIcon,
  ShieldCheckIcon,
} from "../../components/ui/Icons";

export interface SubtitleBurnInPanelProps {
  asset: MediaAsset | null;
  activeTrack: SubtitleTrack | null;
  currentStyle: SubtitleStyle;
  onNavigateToStyling?: () => void;
}

export function SubtitleBurnInPanel({
  asset,
  activeTrack,
  currentStyle,
  onNavigateToStyling,
}: SubtitleBurnInPanelProps): React.ReactElement {
  const [resolutionType, setResolutionType] = useState<"original" | "res1080p" | "res720p">("original");
  const [crf, setCrf] = useState<number>(23);
  const [audioPolicy, setAudioPolicy] = useState<"copy" | "aac">("copy");
  const [status, setStatus] = useState<SubtitleBurnInStatus>({ stage: "queued" });
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [destinationPath, setDestinationPath] = useState<string>("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Poll status while job is active
  useEffect(() => {
    let interval: NodeJS.Timeout | null = null;
    if (isExporting) {
      interval = setInterval(async () => {
        try {
          const cur = await getSubtitleBurnInStatus();
          setStatus(cur);
          if (
            cur.stage === "readyToSave" ||
            cur.stage === "completed" ||
            cur.stage === "failed" ||
            cur.stage === "cancelled"
          ) {
            setIsExporting(false);
            if (cur.stage === "failed") {
              setErrorMessage(cur.error);
            }
          }
        } catch {
          // keep polling or silence
        }
      }, 500);
    }
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [isExporting]);

  const handleStartExport = async () => {
    if (!asset || !activeTrack) return;
    setErrorMessage(null);

    const resolution: ExportResolution =
      resolutionType === "res1080p"
        ? { type: "res1080p" }
        : resolutionType === "res720p"
        ? { type: "res720p" }
        : { type: "original" };

    const profile: SubtitleExportProfile = {
      container: "mp4",
      resolution,
      videoCodec: "libx264",
      crf,
      preset: "medium",
      audioPolicy,
      selectedTrackId: activeTrack.id,
      style: currentStyle,
    };

    const jobId = `burnin-${Date.now()}`;
    const stagingDir = "/tmp/open-media-workbench/staging";

    const request: SubtitleBurnInRequest = {
      jobId,
      sourceMediaPath: asset.source.canonicalPath,
      track: activeTrack,
      style: currentStyle,
      profile,
      stagingDir,
    };

    try {
      setIsExporting(true);
      setStatus({ stage: "preparing" });
      await startSubtitleBurnInExport(request);
    } catch (e) {
      setIsExporting(false);
      setErrorMessage(String(e));
      setStatus({ stage: "failed", error: String(e) });
    }
  };

  const handleCancel = async () => {
    await cancelSubtitleBurnInExport();
    setIsExporting(false);
    setStatus({ stage: "cancelled" });
  };

  const handleFinalizeSave = async () => {
    if (!destinationPath.trim() || !asset) return;
    try {
      await finalizeBurnInExport(destinationPath.trim(), asset.source.canonicalPath);
      setStatus({ stage: "completed", finalDestinationPath: destinationPath.trim() });
    } catch (e) {
      setErrorMessage(String(e));
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="space-y-5 text-xs text-zinc-300 pb-8">
      {/* Header & Context */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-2.5">
        <div className="flex items-center justify-between">
          <span className="text-xs font-semibold text-zinc-100 uppercase tracking-wider flex items-center space-x-1.5">
            <DownloadIcon size={14} className="text-amber-400" />
            <span>Subtitle Burn-In Video Export</span>
          </span>
          <span className="text-[10px] text-zinc-400 font-mono">
            libass + x264
          </span>
        </div>

        <p className="text-[11px] text-zinc-400 leading-relaxed">
          Burn selected subtitles directly into video frames. Produces a standard, universal MP4 playable on any screen, social platform, or hardware player without requiring external caption support.
        </p>
      </div>

      {/* Asset & Subtitle Summary */}
      <div className="grid grid-cols-2 gap-2.5">
        <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/40 space-y-1">
          <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Source Media</span>
          <p className="text-xs font-medium text-zinc-200 truncate">
            {asset ? asset.fileName : "No Media Selected"}
          </p>
          <span className="text-[10px] text-zinc-400 font-mono block">
            {asset && asset.metadata.container.durationMicros
              ? `${(asset.metadata.container.durationMicros / 1_000_000).toFixed(1)}s`
              : "-"}
          </span>
        </div>

        <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/40 space-y-1">
          <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Subtitle Track</span>
          <p className="text-xs font-medium text-zinc-200 truncate">
            {activeTrack ? activeTrack.displayName : "No Track Selected"}
          </p>
          <span className="text-[10px] text-zinc-400 font-mono block">
            {activeTrack ? `${activeTrack.cues.length} cues (${activeTrack.language})` : "-"}
          </span>
        </div>
      </div>

      {/* Selected Style Badge */}
      <div className="p-3 rounded-lg border border-zinc-800/80 bg-zinc-900/30 flex items-center justify-between">
        <div>
          <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Applied Style</span>
          <span className="text-xs font-medium text-amber-300">
            {currentStyle.displayName} ({currentStyle.fontFamily}, {currentStyle.fontSize}px)
          </span>
        </div>
        {onNavigateToStyling && (
          <button
            onClick={onNavigateToStyling}
            className="text-[11px] text-amber-400 hover:text-amber-300 underline font-medium"
          >
            Edit Style
          </button>
        )}
      </div>

      {/* Export Configuration Section */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/40 space-y-3.5">
        <span className="text-xs font-semibold text-zinc-200 block border-b border-zinc-800 pb-1.5">
          Encoding & Quality Settings
        </span>

        {/* Resolution */}
        <div className="space-y-1.5">
          <label className="text-[11px] text-zinc-400 block">Video Resolution</label>
          <div className="grid grid-cols-3 gap-2">
            {[
              { id: "original", label: "Original" },
              { id: "res1080p", label: "1080p FHD" },
              { id: "res720p", label: "720p HD" },
            ].map((r) => (
              <button
                key={r.id}
                onClick={() => setResolutionType(r.id as typeof resolutionType)}
                className={`py-1.5 px-2 rounded border text-[11px] font-medium ${
                  resolutionType === r.id
                    ? "bg-amber-500/20 border-amber-500/60 text-amber-300"
                    : "bg-zinc-950 border-zinc-800 text-zinc-400 hover:border-zinc-700"
                }`}
              >
                {r.label}
              </button>
            ))}
          </div>
        </div>

        {/* Quality (CRF) */}
        <div className="space-y-1">
          <div className="flex justify-between items-center">
            <label className="text-[11px] text-zinc-400">Quality (CRF)</label>
            <span className="text-[11px] font-mono text-zinc-300">
              CRF {crf} {crf <= 20 ? "(High Quality)" : crf <= 24 ? "(Balanced)" : "(Smaller File)"}
            </span>
          </div>
          <input
            type="range"
            min={18}
            max={28}
            value={crf}
            onChange={(e) => setCrf(parseInt(e.target.value, 10))}
            className="w-full accent-amber-500"
          />
        </div>

        {/* Audio Policy */}
        <div className="space-y-1.5">
          <label className="text-[11px] text-zinc-400 block">Audio Stream Handling</label>
          <div className="flex space-x-3">
            <label className="flex items-center space-x-1.5 text-xs text-zinc-300 cursor-pointer">
              <input
                type="radio"
                name="audioPolicy"
                checked={audioPolicy === "copy"}
                onChange={() => setAudioPolicy("copy")}
                className="text-amber-500 focus:ring-0"
              />
              <span>Copy Audio Stream (Lossless)</span>
            </label>
            <label className="flex items-center space-x-1.5 text-xs text-zinc-300 cursor-pointer">
              <input
                type="radio"
                name="audioPolicy"
                checked={audioPolicy === "aac"}
                onChange={() => setAudioPolicy("aac")}
                className="text-amber-500 focus:ring-0"
              />
              <span>Re-encode AAC (192 kbps)</span>
            </label>
          </div>
        </div>

        {/* Technical Notice */}
        <div className="p-2.5 rounded bg-zinc-950/80 border border-zinc-800/80 flex items-start space-x-2 text-[10px] text-zinc-400 leading-snug">
          <ShieldCheckIcon size={14} className="text-amber-400 shrink-0 mt-0.5" />
          <span>
            Burning subtitles permanently draws pixels into video frames and requires video re-encoding. Source media files remain 100% untouched and immutable.
          </span>
        </div>
      </div>

      {/* Error Message */}
      {errorMessage && (
        <div className="p-2.5 rounded bg-red-950/40 border border-red-800 text-red-300 text-xs">
          {errorMessage}
        </div>
      )}

      {/* Progress & Status Card */}
      {isExporting && (
        <div className="p-3.5 rounded-lg border border-amber-500/40 bg-amber-950/20 space-y-2.5">
          <div className="flex items-center justify-between">
            <span className="font-semibold text-xs text-amber-300 flex items-center space-x-1.5">
              <div className="w-3.5 h-3.5 rounded-full border-2 border-amber-400 border-t-transparent animate-spin" />
              <span>
                {status.stage === "encoding"
                  ? status.message
                  : status.stage === "validating"
                  ? "Validating rendered video streams..."
                  : "Preparing subtitle burn-in..."}
              </span>
            </span>
            <Button size="xs" variant="subtle" onClick={handleCancel} className="text-zinc-400 hover:text-red-300">
              Cancel
            </Button>
          </div>
          <div className="w-full h-1.5 bg-zinc-800 rounded-full overflow-hidden">
            <div className="h-full bg-amber-500 animate-pulse w-full" />
          </div>
        </div>
      )}

      {/* Staged Output Ready Card */}
      {status.stage === "readyToSave" && (
        <div className="p-3.5 rounded-lg border border-emerald-500/40 bg-emerald-950/20 space-y-3">
          <div className="flex items-center space-x-1.5 text-emerald-300 font-semibold text-xs">
            <CheckIcon size={14} />
            <span>Subtitles Successfully Burned Into Video!</span>
          </div>

          <div className="grid grid-cols-3 gap-2 text-[11px] font-mono text-zinc-300 bg-zinc-950 p-2.5 rounded border border-zinc-800">
            <div>
              <span className="text-[10px] text-zinc-400 block font-sans">Resolution</span>
              {status.width}x{status.height}
            </div>
            <div>
              <span className="text-[10px] text-zinc-400 block font-sans">File Size</span>
              {formatFileSize(status.sizeBytes)}
            </div>
            <div>
              <span className="text-[10px] text-zinc-400 block font-sans">Duration</span>
              {status.durationSecs.toFixed(1)}s
            </div>
          </div>

          <div className="space-y-1.5 pt-1">
            <label className="text-[11px] text-zinc-400 block">Save Destination Path</label>
            <div className="flex space-x-2">
              <input
                type="text"
                placeholder={asset ? `${asset.source.canonicalPath.replace(/\.[^/.]+$/, "")}_subtitled.mp4` : "/path/to/output.mp4"}
                value={destinationPath}
                onChange={(e) => setDestinationPath(e.target.value)}
                className="flex-1 bg-zinc-950 border border-zinc-700 rounded px-2.5 py-1.5 text-xs text-zinc-200"
              />
              <Button
                size="xs"
                variant="primary"
                onClick={handleFinalizeSave}
                disabled={!destinationPath.trim()}
                className="bg-emerald-600 hover:bg-emerald-500 text-white font-medium"
              >
                Save Video
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Completed State */}
      {status.stage === "completed" && (
        <div className="p-3.5 rounded-lg border border-emerald-500/40 bg-emerald-950/20 space-y-2">
          <div className="flex items-center space-x-1.5 text-emerald-300 font-semibold text-xs">
            <CheckIcon size={14} />
            <span>Video Finalized & Saved</span>
          </div>
          <p className="text-[11px] font-mono text-zinc-300 truncate">
            {status.finalDestinationPath}
          </p>
        </div>
      )}

      {/* Main Action Button */}
      {status.stage !== "readyToSave" && status.stage !== "completed" && (
        <Button
          size="sm"
          variant="primary"
          onClick={handleStartExport}
          disabled={!asset || !activeTrack || isExporting}
          className="w-full bg-amber-600 hover:bg-amber-500 text-black font-semibold flex items-center justify-center space-x-2 py-2"
        >
          <DownloadIcon size={14} />
          <span>
            {isExporting ? "Rendering Video..." : "Start Video Burn-In Export"}
          </span>
        </Button>
      )}
    </div>
  );
}
