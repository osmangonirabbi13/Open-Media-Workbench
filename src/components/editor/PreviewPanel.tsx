import React, { useRef, useEffect, useCallback } from "react";
import {
  MonitorIcon,
  PlayIcon,
  PauseIcon,
  SkipBackIcon,
  SkipForwardIcon,
  StepBackIcon,
  StepForwardIcon,
  VolumeIcon,
  VolumeMuteIcon,
  ResetIcon,
  SubtitlesIcon,
} from "../ui/Icons";
import { IconButton } from "../ui/IconButton";
import { EmptyState } from "../ui/EmptyState";
import type { MediaAsset } from "../../types/media";
import type { SubtitleCue } from "../../types/subtitles";
import { SubtitleOverlay } from "./SubtitleOverlay";
import { useVideoPlayback } from "../../hooks/useVideoPlayback";
import { formatTimecode, formatFrames } from "../../lib/playback";

export interface PreviewPanelProps {
  timecodeFormat: "timecode" | "frames";
  asset: MediaAsset | null;
  onPlaybackTimeChange?: (currentTime: number, duration: number, isPlaying: boolean) => void;
  onRegisterSeek?: (seekFn: (seconds: number) => void) => void;
  activeSubtitleCues?: SubtitleCue[];
  subtitlesVisible?: boolean;
  onToggleSubtitlesVisible?: () => void;
  enableKaraoke?: boolean;
  subtitleStyle?: import("../../types/styling").SubtitleStyle;
}

export function PreviewPanel({
  timecodeFormat,
  asset,
  onPlaybackTimeChange,
  onRegisterSeek,
  activeSubtitleCues,
  subtitlesVisible = true,
  onToggleSubtitlesVisible,
  enableKaraoke = true,
  subtitleStyle,
}: PreviewPanelProps): React.ReactElement {
  const videoRef = useRef<HTMLVideoElement>(null);
  const scrubberTrackRef = useRef<HTMLDivElement>(null);

  const {
    playbackInfo,
    isLoading,
    isPlaying,
    isBuffering,
    currentTime,
    duration,
    volume,
    isMuted,
    error,
    fps,
    aspectRatio,
    proxyState,
    togglePlay,
    seek,
    stepFrame,
    jumpToStart,
    jumpToEnd,
    setVolume,
    toggleMute,
    triggerProxyGeneration,
    cancelProxyGeneration,
  } = useVideoPlayback({ videoRef, asset });

  // Register seek handler to parent for timeline playhead scrubbing
  useEffect(() => {
    if (onRegisterSeek) {
      onRegisterSeek(seek);
    }
  }, [onRegisterSeek, seek]);

  // Synchronize playback timing to parent component (e.g. for timeline playhead synchronization)
  useEffect(() => {
    if (onPlaybackTimeChange) {
      onPlaybackTimeChange(currentTime, duration, isPlaying);
    }
  }, [onPlaybackTimeChange, currentTime, duration, isPlaying]);

  // Format timecodes according to preference
  const currentTimecode =
    timecodeFormat === "frames"
      ? formatFrames(currentTime, fps)
      : formatTimecode(currentTime, fps);

  const durationTimecode =
    timecodeFormat === "frames"
      ? formatFrames(duration, fps)
      : formatTimecode(duration, fps);

  // Scrubber progress fraction [0, 1]
  const progressFraction = duration > 0 ? Math.min(1, Math.max(0, currentTime / duration)) : 0;

  // Handle scrubber pointer click & drag
  const handleScrubberPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!asset || duration <= 0) return;
    const track = scrubberTrackRef.current;
    if (!track) return;

    e.preventDefault();
    track.setPointerCapture(e.pointerId);

    const updateSeekFromEvent = (clientX: number) => {
      const rect = track.getBoundingClientRect();
      if (rect.width <= 0) return;
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
      seek(ratio * duration);
    };

    updateSeekFromEvent(e.clientX);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      updateSeekFromEvent(moveEvent.clientX);
    };

    const handlePointerUp = (upEvent: PointerEvent) => {
      try {
        track.releasePointerCapture(upEvent.pointerId);
      } catch {
        // Ignored
      }
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  };

  // Keyboard shortcut listener
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // Do not intercept if focus is inside text input, textarea, select, or modal
      const activeEl = document.activeElement;
      if (
        activeEl &&
        (activeEl.tagName === "INPUT" ||
          activeEl.tagName === "TEXTAREA" ||
          activeEl.tagName === "SELECT" ||
          activeEl.getAttribute("role") === "slider")
      ) {
        return;
      }

      if (!asset) return;

      switch (e.code) {
        case "Space":
          e.preventDefault();
          togglePlay();
          break;
        case "ArrowLeft":
          e.preventDefault();
          // Step 1 frame if Shift held, else step 1 second
          if (e.shiftKey) {
            stepFrame(-1);
          } else {
            seek(currentTime - 1);
          }
          break;
        case "ArrowRight":
          e.preventDefault();
          if (e.shiftKey) {
            stepFrame(1);
          } else {
            seek(currentTime + 1);
          }
          break;
        case "Home":
          e.preventDefault();
          jumpToStart();
          break;
        case "End":
          e.preventDefault();
          jumpToEnd();
          break;
        default:
          break;
      }
    },
    [asset, togglePlay, stepFrame, seek, currentTime, jumpToStart, jumpToEnd]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleKeyDown]);

  return (
    <section
      aria-label="Preview Monitor"
      className="flex flex-col h-full bg-zinc-950 overflow-hidden select-none"
    >
      {/* Panel Header */}
      <div className="h-8 shrink-0 flex items-center justify-between px-3 border-b border-zinc-800/70 bg-zinc-900/40">
        <div className="flex items-center space-x-2">
          <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
            Preview Monitor
          </span>
          {aspectRatio && (
            <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-zinc-800 text-zinc-300">
              {aspectRatio.width}&times;{aspectRatio.height} &bull; {fps.toFixed(2)} fps
            </span>
          )}
          {playbackInfo?.isProxy && (
            <span className="px-1.5 py-0.2 rounded text-[10px] font-semibold bg-amber-500/20 text-amber-300 border border-amber-500/40">
              PROXY
            </span>
          )}
        </div>

        <div className="flex items-center space-x-2">
          {onToggleSubtitlesVisible && (
            <button
              type="button"
              onClick={onToggleSubtitlesVisible}
              title={subtitlesVisible ? "Disable Subtitle Overlay" : "Enable Subtitle Overlay"}
              className={`flex items-center space-x-1 px-1.5 py-0.5 rounded text-[11px] font-medium transition-colors border ${
                subtitlesVisible
                  ? "bg-indigo-950/80 text-indigo-300 border-indigo-700/60 shadow-xs"
                  : "bg-zinc-900 text-zinc-500 border-zinc-800 hover:text-zinc-300"
              }`}
            >
              <SubtitlesIcon size={12} className={subtitlesVisible ? "text-indigo-400" : ""} />
              <span>CC</span>
            </button>
          )}

          <div className="font-mono text-xs text-zinc-300 px-2 py-0.5 rounded bg-zinc-900 border border-zinc-800/60">
            {currentTimecode}
          </div>
        </div>
      </div>

      {/* Main Preview Surface (Viewport) */}
      <div className="flex-1 flex items-center justify-center p-3 bg-zinc-950/90 overflow-hidden relative">
        <div className="w-full h-full max-w-5xl max-h-full rounded-lg border border-zinc-800/70 bg-zinc-900/30 flex items-center justify-center relative shadow-2xl overflow-hidden">
          {/* Subtle Grid Background */}
          <div
            className="absolute inset-0 opacity-5 pointer-events-none"
            style={{
              backgroundImage:
                "linear-gradient(to right, #ffffff 1px, transparent 1px), linear-gradient(to bottom, #ffffff 1px, transparent 1px)",
              backgroundSize: "24px 24px",
            }}
          />

          {!asset ? (
            <EmptyState
              icon={<MonitorIcon size={28} />}
              badge="Preview Monitor"
              title="No Active Clip Selected"
              description="Select or import a media asset from the Media Library on the left to watch real decoded video and seek frames."
            />
          ) : (
            <>
              {/* HTML5 Video Element */}
              <video
                ref={videoRef}
                src={playbackInfo?.playbackUrl}
                playsInline
                preload="metadata"
                className={`w-full h-full object-contain relative z-10 ${
                  error ? "hidden" : "block"
                }`}
                onClick={togglePlay}
              />

              {/* Subtitle Caption Overlay */}
              <SubtitleOverlay
                activeCues={activeSubtitleCues || []}
                isVisible={subtitlesVisible}
                currentTimeUs={Math.round(currentTime * 1_000_000)}
                enableKaraoke={enableKaraoke}
                style={subtitleStyle}
              />

              {/* Buffering / Loading Overlay */}
              {(isLoading || isBuffering) && !error && (
                <div className="absolute inset-0 z-20 flex flex-col items-center justify-center bg-black/40 backdrop-blur-[1px] pointer-events-none">
                  <div className="w-8 h-8 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin mb-2" />
                  <span className="text-xs font-medium text-zinc-200">
                    {isLoading ? "Loading media stream..." : "Buffering..."}
                  </span>
                </div>
              )}

              {/* Proxy Generation In-Flight Banner */}
              {proxyState.state === "generating" && (
                <div className="absolute top-3 left-3 right-3 z-30 p-2.5 rounded bg-indigo-950/90 border border-indigo-700/60 shadow-lg flex items-center justify-between text-xs text-indigo-200 backdrop-blur-sm">
                  <div className="flex items-center space-x-2">
                    <div className="w-4 h-4 rounded-full border-2 border-indigo-400 border-t-transparent animate-spin shrink-0" />
                    <span>Generating 720p H.264 preview proxy in background...</span>
                  </div>
                  <button
                    type="button"
                    onClick={cancelProxyGeneration}
                    className="px-2 py-1 rounded bg-indigo-800/80 hover:bg-indigo-700 text-white font-medium transition-colors"
                  >
                    Cancel
                  </button>
                </div>
              )}

              {/* Decoding Error Banner & Proxy Fallback Button */}
              {error && (
                <div className="absolute inset-0 z-30 flex flex-col items-center justify-center p-6 bg-zinc-950/90 text-center max-w-md mx-auto">
                  <div className="w-10 h-10 rounded-full bg-rose-500/20 text-rose-400 flex items-center justify-center mb-3">
                    <ResetIcon size={20} />
                  </div>
                  <h3 className="text-sm font-semibold text-zinc-100 mb-1">
                    Native Playback Limitation
                  </h3>
                  <p className="text-xs text-zinc-400 mb-4 leading-relaxed">{error}</p>
                  <div className="flex items-center space-x-2">
                    <button
                      type="button"
                      onClick={triggerProxyGeneration}
                      disabled={proxyState.state === "generating"}
                      className="px-3 py-1.5 rounded text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white shadow transition-colors disabled:opacity-50"
                    >
                      {proxyState.state === "generating"
                        ? "Generating Proxy..."
                        : "Generate Compatible Preview Proxy (720p)"}
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* Interactive Seek Bar */}
      <div className="px-4 py-1.5 bg-zinc-900/70 border-t border-zinc-800/60">
        <div
          ref={scrubberTrackRef}
          role="slider"
          tabIndex={0}
          aria-label="Playback seek bar"
          aria-valuemin={0}
          aria-valuemax={duration}
          aria-valuenow={currentTime}
          onPointerDown={handleScrubberPointerDown}
          className={`h-2.5 rounded-full bg-zinc-800 relative cursor-pointer flex items-center group ${
            !asset || duration <= 0 ? "opacity-40 pointer-events-none" : ""
          }`}
        >
          {/* Progress fill */}
          <div
            className="h-full rounded-full bg-indigo-500 transition-all duration-75"
            style={{ width: `${progressFraction * 100}%` }}
          />
          {/* Scrubber Thumb */}
          <div
            className="absolute w-3.5 h-3.5 rounded-full bg-white shadow-md border border-indigo-600 pointer-events-none -ml-1.5 opacity-90 group-hover:scale-125 transition-transform"
            style={{ left: `${progressFraction * 100}%` }}
          />
        </div>
      </div>

      {/* Transport Controls Bar */}
      <div className="h-10 shrink-0 flex items-center justify-between px-4 border-t border-zinc-800/70 bg-zinc-900/60">
        {/* Left: Timecode Readout */}
        <div className="flex items-center space-x-2 text-xs font-mono text-zinc-400">
          <span className="text-zinc-200 font-medium">{currentTimecode}</span>
          <span className="text-zinc-600">/</span>
          <span>{durationTimecode}</span>
        </div>

        {/* Center: Transport Action Buttons */}
        <div className="flex items-center space-x-1">
          <IconButton
            icon={<SkipBackIcon size={14} />}
            label="Jump to Start"
            shortcut="Home"
            size="sm"
            disabled={!asset}
            onClick={jumpToStart}
          />
          <IconButton
            icon={<StepBackIcon size={14} />}
            label="Step Backward (Shift+Left = 1 frame, Left = 1 sec)"
            shortcut="Left"
            size="sm"
            disabled={!asset}
            onClick={() => stepFrame(-1)}
          />
          <IconButton
            icon={isPlaying ? <PauseIcon size={14} /> : <PlayIcon size={14} />}
            label={isPlaying ? "Pause" : "Play"}
            shortcut="Space"
            size="md"
            variant="default"
            disabled={!asset}
            onClick={togglePlay}
            className={
              isPlaying
                ? "bg-indigo-600 text-white hover:bg-indigo-500"
                : "bg-zinc-800 text-zinc-100 hover:bg-zinc-700"
            }
          />
          <IconButton
            icon={<StepForwardIcon size={14} />}
            label="Step Forward (Shift+Right = 1 frame, Right = 1 sec)"
            shortcut="Right"
            size="sm"
            disabled={!asset}
            onClick={() => stepFrame(1)}
          />
          <IconButton
            icon={<SkipForwardIcon size={14} />}
            label="Jump to End"
            shortcut="End"
            size="sm"
            disabled={!asset}
            onClick={jumpToEnd}
          />
        </div>

        {/* Right: Volume & Format Info */}
        <div className="flex items-center space-x-3">
          {/* Volume Control */}
          <div className="flex items-center space-x-1.5">
            <button
              type="button"
              onClick={toggleMute}
              disabled={!asset}
              className="p-1 text-zinc-400 hover:text-zinc-200 transition-colors disabled:opacity-40"
              title={isMuted || volume === 0 ? "Unmute" : "Mute"}
            >
              {isMuted || volume === 0 ? (
                <VolumeMuteIcon size={14} />
              ) : (
                <VolumeIcon size={14} />
              )}
            </button>
            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={isMuted ? 0 : volume}
              disabled={!asset}
              onChange={(e) => setVolume(parseFloat(e.target.value))}
              className="w-16 h-1.5 accent-indigo-500 rounded bg-zinc-800 cursor-pointer disabled:opacity-40"
              aria-label="Volume level"
            />
          </div>

          {/* Proxy Fallback Trigger button if not already proxy and asset is present */}
          {asset && !playbackInfo?.isProxy && proxyState.state !== "generating" && (
            <button
              type="button"
              onClick={triggerProxyGeneration}
              title="Transcode a 720p H.264 preview proxy for smoother scrubbing or unsupported codecs"
              className="px-2 py-0.5 rounded text-[10px] font-medium bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition-colors border border-zinc-700/60"
            >
              Proxy
            </button>
          )}
        </div>
      </div>
    </section>
  );
}
