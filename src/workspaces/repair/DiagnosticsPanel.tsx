import React from "react";
import type { MediaAsset } from "../../types/media";
import { formatFileSize } from "../../types/media";
import { ActivityIcon, FileSearchIcon, ShieldCheckIcon } from "../../components/ui/Icons";

export interface DiagnosticsPanelProps {
  asset: MediaAsset | null;
}

export function DiagnosticsPanel({ asset }: DiagnosticsPanelProps): React.ReactElement {
  if (!asset) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-6 text-center text-zinc-500">
        <FileSearchIcon size={32} className="mb-2 text-zinc-600" />
        <h4 className="text-xs font-semibold text-zinc-400">No Asset Selected</h4>
        <p className="text-[11px] mt-1 max-w-xs">
          Select or import a media file from the library to run container and stream integrity diagnostics.
        </p>
      </div>
    );
  }

  const isPlayable =
    asset.compatibility.overall === "supported" ||
    asset.compatibility.overall === "supportedWithLimitations";

  return (
    <div className="h-full flex flex-col p-4 overflow-y-auto space-y-4 select-text">
      {/* File Overview Summary Card */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-semibold text-zinc-200 truncate max-w-[220px]" title={asset.fileName}>
            {asset.fileName}
          </span>
          <span
            className={`px-2 py-0.5 rounded text-[10px] font-medium border ${
              isPlayable
                ? "bg-emerald-950/60 text-emerald-300 border-emerald-800/40"
                : "bg-amber-950/60 text-amber-300 border-amber-800/40"
            }`}
          >
            {isPlayable ? "Container Playable" : "Diagnostic Target"}
          </span>
        </div>

        <div className="grid grid-cols-2 gap-2 text-[11px] font-mono text-zinc-400">
          <div>
            <span className="text-zinc-500">File Size: </span>
            <span className="text-zinc-200">{formatFileSize(asset.source.fileSize)}</span>
          </div>
          <div>
            <span className="text-zinc-500">Format: </span>
            <span className="text-zinc-200 uppercase">{asset.metadata.container.formatName || "Unknown"}</span>
          </div>
          <div>
            <span className="text-zinc-500">Video Streams: </span>
            <span className="text-zinc-200">{asset.metadata.videoStreams.length}</span>
          </div>
          <div>
            <span className="text-zinc-500">Audio Streams: </span>
            <span className="text-zinc-200">{asset.metadata.audioStreams.length}</span>
          </div>
        </div>
      </div>

      {/* Diagnostics Category Checklist */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-zinc-400 flex items-center space-x-1.5">
            <ActivityIcon size={13} className="text-indigo-400" />
            <span>Inspection Categories</span>
          </h4>
          <span className="px-1.5 py-0.2 rounded text-[10px] bg-zinc-800 text-zinc-400 border border-zinc-700/50">
            Engine: STEP 12
          </span>
        </div>

        {/* 1. File Integrity & Header Check */}
        <div className="p-3 rounded-lg border border-zinc-800/80 bg-zinc-900/40 space-y-1">
          <div className="flex items-center justify-between text-xs font-medium text-zinc-300">
            <span>1. Container Header & Atom Hierarchy</span>
            <span className="text-[10px] font-mono text-amber-400/90">Pending Deep Probe</span>
          </div>
          <p className="text-[11px] text-zinc-500">
            Evaluates ISO BMFF atom tree (`ftyp`, `moov`, `mdat`), verifies unfinalized file headers, and checks atom offsets.
          </p>
        </div>

        {/* 2. Bitstream & Packet Continuity */}
        <div className="p-3 rounded-lg border border-zinc-800/80 bg-zinc-900/40 space-y-1">
          <div className="flex items-center justify-between text-xs font-medium text-zinc-300">
            <span>2. Packet & Bitstream Continuity</span>
            <span className="text-[10px] font-mono text-amber-400/90">Pending Deep Probe</span>
          </div>
          <p className="text-[11px] text-zinc-500">
            Scans for corrupted NAL units, truncated audio packets, and invalid slice headers.
          </p>
        </div>

        {/* 3. Timestamp & Index Synchronization */}
        <div className="p-3 rounded-lg border border-zinc-800/80 bg-zinc-900/40 space-y-1">
          <div className="flex items-center justify-between text-xs font-medium text-zinc-300">
            <span>3. Timestamp & Index Jitter</span>
            <span className="text-[10px] font-mono text-amber-400/90">Pending Deep Probe</span>
          </div>
          <p className="text-[11px] text-zinc-500">
            Detects non-monotonic PTS/DTS timestamps, missing keyframe indices, and audio/video desync.
          </p>
        </div>

        {/* 4. Repair Recommendation Assessment */}
        <div className="p-3 rounded-lg border border-zinc-800/80 bg-zinc-900/40 space-y-1">
          <div className="flex items-center justify-between text-xs font-medium text-zinc-300">
            <span>4. Salvage Strategy Recommendation</span>
            <span className="text-[10px] font-mono text-zinc-500">Requires STEP 12</span>
          </div>
          <p className="text-[11px] text-zinc-500">
            Recommends reference-based moov reconstruction, container remuxing, or raw stream salvage.
          </p>
        </div>
      </div>

      {/* Honest Limitation / Scheduled Notice */}
      <div className="p-3 rounded-lg border border-indigo-900/40 bg-indigo-950/20 text-indigo-300/90 text-xs flex items-start space-x-2.5">
        <ShieldCheckIcon size={16} className="shrink-0 mt-0.5 text-indigo-400" />
        <div className="space-y-1 leading-relaxed">
          <span className="font-semibold block">STEP 12 Diagnostic Engine Notice</span>
          <p className="text-[11px] text-indigo-200/80">
            Deep bitstream inspection, atom carving, and corruption fault isolation are part of the upcoming STEP 12 implementation. No simulated or fabricated health scores are displayed.
          </p>
        </div>
      </div>
    </div>
  );
}
