import React from "react";
import type { MediaAsset } from "../../types/media";
import { formatDurationMicros, formatFileSize, formatFps } from "../../types/media";
import { CloseIcon, FilmIcon, InfoIcon } from "../ui/Icons";

interface MediaDetailsModalProps {
  asset: MediaAsset | null;
  onClose: () => void;
}

export function MediaDetailsModal({
  asset,
  onClose,
}: MediaDetailsModalProps): React.ReactElement | null {
  if (!asset) return null;

  const { metadata, compatibility, source } = asset;
  const primaryVideo = metadata.videoStreams[0];
  const primaryAudio = metadata.audioStreams[0];

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "supported":
        return (
          <span className="px-2 py-0.5 rounded text-xs font-semibold bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
            Supported
          </span>
        );
      case "supportedWithLimitations":
        return (
          <span className="px-2 py-0.5 rounded text-xs font-semibold bg-amber-500/15 text-amber-400 border border-amber-500/30">
            Supported With Limitations
          </span>
        );
      case "unsupported":
        return (
          <span className="px-2 py-0.5 rounded text-xs font-semibold bg-rose-500/15 text-rose-400 border border-rose-500/30">
            Unsupported
          </span>
        );
      default:
        return (
          <span className="px-2 py-0.5 rounded text-xs font-semibold bg-zinc-700/50 text-zinc-300 border border-zinc-600">
            Unknown
          </span>
        );
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="media-details-title"
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-xs"
    >
      <div className="bg-zinc-900 border border-zinc-700/80 rounded-xl shadow-2xl max-w-2xl w-full max-h-[85vh] flex flex-col overflow-hidden text-zinc-100">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-zinc-800 bg-zinc-950/40">
          <div className="flex items-center space-x-3 truncate">
            <div className="p-2 rounded-lg bg-indigo-500/15 text-indigo-400 border border-indigo-500/30 shrink-0">
              <FilmIcon size={20} />
            </div>
            <div className="truncate">
              <h2 id="media-details-title" className="text-sm font-semibold truncate text-zinc-100">
                {asset.fileName}
              </h2>
              <p className="text-xs text-zinc-400">
                {metadata.container.formatName.toUpperCase()} &bull; {formatFileSize(source.fileSize)} &bull;{" "}
                {formatDurationMicros(metadata.container.durationMicros)}
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close media details"
            className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors"
          >
            <CloseIcon size={18} />
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6 text-xs">
          {/* Compatibility Banner */}
          <div className="p-3.5 rounded-lg bg-zinc-950/60 border border-zinc-800/80 flex flex-col space-y-2">
            <div className="flex items-center justify-between">
              <span className="font-medium text-zinc-300">Format Compatibility</span>
              {getStatusBadge(compatibility.overall)}
            </div>
            {compatibility.warnings.length > 0 && (
              <div className="mt-2 space-y-1">
                {compatibility.warnings.map((warn, i) => (
                  <p key={i} className="text-amber-300/90 flex items-start space-x-1.5 text-[11px]">
                    <span className="text-amber-400 shrink-0">&bull;</span>
                    <span>{warn}</span>
                  </p>
                ))}
              </div>
            )}
          </div>

          {/* Video Stream Info */}
          {primaryVideo ? (
            <div className="space-y-2">
              <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Video Stream (Index {primaryVideo.streamIndex})
              </h3>
              <div className="grid grid-cols-2 gap-2 p-3 bg-zinc-950/40 rounded-lg border border-zinc-800/60">
                <div>
                  <span className="text-zinc-500">Codec:</span>{" "}
                  <span className="font-mono text-zinc-200 uppercase">{primaryVideo.codecName}</span>
                </div>
                <div>
                  <span className="text-zinc-500">Resolution:</span>{" "}
                  <span className="font-mono text-zinc-200">
                    {primaryVideo.width} &times; {primaryVideo.height}
                  </span>
                </div>
                <div>
                  <span className="text-zinc-500">Frame Rate:</span>{" "}
                  <span className="font-mono text-zinc-200">
                    {formatFps(primaryVideo.averageFrameRate)}{" "}
                    {primaryVideo.averageFrameRate && (
                      <span className="text-zinc-500 text-[10px]">
                        ({primaryVideo.averageFrameRate.num}/{primaryVideo.averageFrameRate.den})
                      </span>
                    )}
                  </span>
                </div>
                <div>
                  <span className="text-zinc-500">Pixel Format:</span>{" "}
                  <span className="font-mono text-zinc-200">{primaryVideo.pixelFormat || "Unknown"}</span>
                </div>
                <div>
                  <span className="text-zinc-500">Timebase:</span>{" "}
                  <span className="font-mono text-zinc-200">
                    {primaryVideo.timebase
                      ? `${primaryVideo.timebase.num}/${primaryVideo.timebase.den}`
                      : "Unknown"}
                  </span>
                </div>
                <div>
                  <span className="text-zinc-500">Orientation:</span>{" "}
                  <span className="font-mono text-zinc-200">
                    {primaryVideo.rotationDegrees !== undefined && primaryVideo.rotationDegrees !== null
                      ? `${primaryVideo.rotationDegrees}&deg;`
                      : "0&deg; (Normal)"}
                  </span>
                </div>
              </div>
            </div>
          ) : (
            <div className="p-3 bg-zinc-950/40 rounded-lg border border-zinc-800 text-zinc-500 italic">
              No video stream detected (audio-only asset).
            </div>
          )}

          {/* Audio Stream Info */}
          {primaryAudio ? (
            <div className="space-y-2">
              <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
                Audio Stream (Index {primaryAudio.streamIndex})
              </h3>
              <div className="grid grid-cols-2 gap-2 p-3 bg-zinc-950/40 rounded-lg border border-zinc-800/60">
                <div>
                  <span className="text-zinc-500">Codec:</span>{" "}
                  <span className="font-mono text-zinc-200 uppercase">{primaryAudio.codecName}</span>
                </div>
                <div>
                  <span className="text-zinc-500">Sample Rate:</span>{" "}
                  <span className="font-mono text-zinc-200">{primaryAudio.sampleRate} Hz</span>
                </div>
                <div>
                  <span className="text-zinc-500">Channels:</span>{" "}
                  <span className="font-mono text-zinc-200">
                    {primaryAudio.channels} ({primaryAudio.channelLayout || "Channels"})
                  </span>
                </div>
                <div>
                  <span className="text-zinc-500">Bitrate:</span>{" "}
                  <span className="font-mono text-zinc-200">
                    {primaryAudio.bitrateBps
                      ? `${Math.round(primaryAudio.bitrateBps / 1000)} kbps`
                      : "Unknown"}
                  </span>
                </div>
              </div>
            </div>
          ) : (
            <div className="p-3 bg-zinc-950/40 rounded-lg border border-zinc-800 text-zinc-500 italic">
              No audio stream detected (silent video asset).
            </div>
          )}

          {/* Technical Provenance */}
          <div className="space-y-2">
            <h3 className="text-xs font-semibold text-zinc-400 uppercase tracking-wider">
              File Provenance
            </h3>
            <div className="p-3 bg-zinc-950/40 rounded-lg border border-zinc-800/60 space-y-1.5 font-mono text-[11px]">
              <div className="truncate">
                <span className="text-zinc-500">Path:</span>{" "}
                <span className="text-zinc-300 select-all" title={source.canonicalPath}>
                  {source.canonicalPath}
                </span>
              </div>
              <div>
                <span className="text-zinc-500">Fingerprint:</span>{" "}
                <span className="text-zinc-400 select-all text-[10px]">
                  {source.identityHint.substring(0, 24)}...
                </span>
              </div>
            </div>
          </div>

          {/* Roadmap Scope Notice */}
          <div className="p-3 rounded-lg bg-indigo-950/30 border border-indigo-800/50 flex items-start space-x-2 text-[11px] text-indigo-300">
            <InfoIcon size={14} className="shrink-0 mt-0.5 text-indigo-400" />
            <div>
              <p className="font-medium text-indigo-200">Milestone 2 &bull; Step 6 Scope Lock</p>
              <p className="text-indigo-300/80 mt-0.5">
                This media file has been securely validated and probed with FFprobe. Real video preview seeking will be enabled in Step 7. Timeline clip editing begins in Step 9.
              </p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end px-6 py-3 border-t border-zinc-800 bg-zinc-950/40">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-medium rounded-lg transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
