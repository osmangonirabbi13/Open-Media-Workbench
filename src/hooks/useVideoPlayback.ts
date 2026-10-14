import { useState, useEffect, useRef, useCallback } from "react";
import type { MediaAsset } from "../types/media";
import type { PlaybackInfo, ProxyState } from "../types/playback";
import {
  getMediaPlaybackUrl,
  requestPreviewProxy,
  cancelPreviewProxy,
  getProxyStatus,
} from "../lib/playback";

export interface UseVideoPlaybackOptions {
  videoRef: React.RefObject<HTMLVideoElement | null>;
  asset: MediaAsset | null;
}

export interface UseVideoPlaybackReturn {
  playbackInfo: PlaybackInfo | null;
  isLoading: boolean;
  isPlaying: boolean;
  isBuffering: boolean;
  isSeeking: boolean;
  currentTime: number;
  duration: number;
  volume: number;
  isMuted: boolean;
  error: string | null;
  isEnded: boolean;
  canPlay: boolean;
  fps: number;
  aspectRatio: { width: number; height: number } | null;
  proxyState: ProxyState;
  togglePlay: () => void;
  play: () => Promise<void>;
  pause: () => void;
  seek: (seconds: number) => void;
  stepFrame: (frames: number) => void;
  jumpToStart: () => void;
  jumpToEnd: () => void;
  setVolume: (vol: number) => void;
  toggleMute: () => void;
  triggerProxyGeneration: () => Promise<void>;
  cancelProxyGeneration: () => Promise<void>;
}

export function useVideoPlayback({
  videoRef,
  asset,
}: UseVideoPlaybackOptions): UseVideoPlaybackReturn {
  const [playbackInfo, setPlaybackInfo] = useState<PlaybackInfo | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [isBuffering, setIsBuffering] = useState<boolean>(false);
  const [isSeeking, setIsSeeking] = useState<boolean>(false);
  const [currentTime, setCurrentTime] = useState<number>(0);
  const [duration, setDuration] = useState<number>(0);
  const [volume, setVolumeState] = useState<number>(1.0);
  const [isMuted, setIsMuted] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [isEnded, setIsEnded] = useState<boolean>(false);
  const [canPlay, setCanPlay] = useState<boolean>(false);
  const [proxyState, setProxyState] = useState<ProxyState>({ state: "none" });

  const activeAssetIdRef = useRef<string | null>(null);
  const proxyPollIntervalRef = useRef<number | null>(null);

  const fps =
    playbackInfo?.fps ||
    asset?.metadata.videoStreams[0]?.averageFrameRate?.num
      ? (asset?.metadata.videoStreams[0]?.averageFrameRate?.num ?? 30) /
        (asset?.metadata.videoStreams[0]?.averageFrameRate?.den ?? 1)
      : 30;

  const aspectRatio =
    playbackInfo?.width && playbackInfo?.height
      ? { width: playbackInfo.width, height: playbackInfo.height }
      : asset?.metadata.videoStreams[0]
        ? {
            width: asset.metadata.videoStreams[0].width,
            height: asset.metadata.videoStreams[0].height,
          }
        : null;

  // Cleanup proxy polling helper
  const stopProxyPolling = useCallback(() => {
    if (proxyPollIntervalRef.current !== null) {
      clearInterval(proxyPollIntervalRef.current);
      proxyPollIntervalRef.current = null;
    }
  }, []);

  // Poll for proxy completion
  const startProxyPolling = useCallback(
    (assetId: string) => {
      stopProxyPolling();
      proxyPollIntervalRef.current = window.setInterval(async () => {
        if (activeAssetIdRef.current !== assetId) {
          stopProxyPolling();
          return;
        }
        try {
          const res = await getProxyStatus(assetId);
          setProxyState(res.status);

          if (res.status.state === "ready") {
            stopProxyPolling();
            // Refresh playback URL to use the newly ready proxy
            const freshInfo = await getMediaPlaybackUrl(assetId);
            if (activeAssetIdRef.current === assetId) {
              setPlaybackInfo(freshInfo);
              setError(null);
            }
          } else if (res.status.state === "failed" || res.status.state === "cancelled") {
            stopProxyPolling();
          }
        } catch {
          stopProxyPolling();
        }
      }, 1000);
    },
    [stopProxyPolling]
  );

  // 1. Fetch playback info whenever selected asset changes
  useEffect(() => {
    let isCancelled = false;
    stopProxyPolling();

    if (!asset) {
      activeAssetIdRef.current = null;
      setPlaybackInfo(null);
      setIsLoading(false);
      setIsPlaying(false);
      setCurrentTime(0);
      setDuration(0);
      setError(null);
      setCanPlay(false);
      setProxyState({ state: "none" });
      if (videoRef.current) {
        videoRef.current.pause();
        videoRef.current.removeAttribute("src");
        videoRef.current.load();
      }
      return;
    }

    const currentId = asset.id;
    activeAssetIdRef.current = currentId;
    setIsLoading(true);
    setError(null);
    setCanPlay(false);
    setCurrentTime(0);
    setDuration(
      asset.metadata.container.durationMicros
        ? asset.metadata.container.durationMicros / 1_000_000
        : 0
    );

    async function loadPlayback() {
      try {
        const info = await getMediaPlaybackUrl(currentId);
        if (isCancelled || activeAssetIdRef.current !== currentId) return;

        setPlaybackInfo(info);
        setProxyState(info.proxyState);

        if (info.proxyState.state === "generating") {
          startProxyPolling(currentId);
        }
      } catch (err: unknown) {
        if (isCancelled || activeAssetIdRef.current !== currentId) return;
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
      } finally {
        if (!isCancelled && activeAssetIdRef.current === currentId) {
          setIsLoading(false);
        }
      }
    }

    loadPlayback();

    return () => {
      isCancelled = true;
      stopProxyPolling();
    };
  }, [asset, videoRef, startProxyPolling, stopProxyPolling]);

  // 2. Attach real HTMLMediaElement event listeners
  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    const handleLoadedMetadata = () => {
      if (Number.isFinite(video.duration) && video.duration > 0) {
        setDuration(video.duration);
      }
      setIsLoading(false);
    };

    const handleCanPlay = () => {
      setCanPlay(true);
      setIsBuffering(false);
      setIsLoading(false);
    };

    const handleTimeUpdate = () => {
      setCurrentTime(video.currentTime);
      setIsEnded(video.ended);
    };

    const handlePlay = () => {
      setIsPlaying(true);
      setIsEnded(false);
    };

    const handlePause = () => {
      setIsPlaying(false);
    };

    const handleWaiting = () => {
      setIsBuffering(true);
    };

    const handlePlaying = () => {
      setIsBuffering(false);
      setIsPlaying(true);
    };

    const handleSeeking = () => {
      setIsSeeking(true);
    };

    const handleSeeked = () => {
      setIsSeeking(false);
      setCurrentTime(video.currentTime);
    };

    const handleEnded = () => {
      setIsPlaying(false);
      setIsEnded(true);
    };

    const handleError = () => {
      setIsLoading(false);
      setIsBuffering(false);
      setIsPlaying(false);
      const mediaError = video.error;
      let message = "HTML5 video playback error";
      if (mediaError) {
        switch (mediaError.code) {
          case MediaError.MEDIA_ERR_ABORTED:
            message = "Playback aborted by user or browser";
            break;
          case MediaError.MEDIA_ERR_NETWORK:
            message = "Network / protocol error during media streaming";
            break;
          case MediaError.MEDIA_ERR_DECODE:
            message =
              "Video decoding failed: Codec or profile may not be natively supported by WebKitGTK. Try generating a preview proxy.";
            break;
          case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
            message =
              "Format or codec not natively supported by WebKitGTK. A preview proxy can make this media playable.";
            break;
          default:
            message = mediaError.message || message;
        }
      }
      setError(message);
    };

    video.addEventListener("loadedmetadata", handleLoadedMetadata);
    video.addEventListener("canplay", handleCanPlay);
    video.addEventListener("timeupdate", handleTimeUpdate);
    video.addEventListener("play", handlePlay);
    video.addEventListener("pause", handlePause);
    video.addEventListener("waiting", handleWaiting);
    video.addEventListener("playing", handlePlaying);
    video.addEventListener("seeking", handleSeeking);
    video.addEventListener("seeked", handleSeeked);
    video.addEventListener("ended", handleEnded);
    video.addEventListener("error", handleError);

    return () => {
      video.removeEventListener("loadedmetadata", handleLoadedMetadata);
      video.removeEventListener("canplay", handleCanPlay);
      video.removeEventListener("timeupdate", handleTimeUpdate);
      video.removeEventListener("play", handlePlay);
      video.removeEventListener("pause", handlePause);
      video.removeEventListener("waiting", handleWaiting);
      video.removeEventListener("playing", handlePlaying);
      video.removeEventListener("seeking", handleSeeking);
      video.removeEventListener("seeked", handleSeeked);
      video.removeEventListener("ended", handleEnded);
      video.removeEventListener("error", handleError);
    };
  }, [videoRef, playbackInfo?.playbackUrl]);

  // Transport methods
  const play = useCallback(async () => {
    const video = videoRef.current;
    if (!video) return;
    try {
      await video.play();
    } catch (err: unknown) {
      console.warn("Autoplay / play prevented:", err);
    }
  }, [videoRef]);

  const pause = useCallback(() => {
    const video = videoRef.current;
    if (video && !video.paused) {
      video.pause();
    }
  }, [videoRef]);

  const togglePlay = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused || video.ended) {
      play();
    } else {
      pause();
    }
  }, [videoRef, play, pause]);

  const seek = useCallback(
    (seconds: number) => {
      const video = videoRef.current;
      if (!video) return;
      const targetDuration = duration > 0 ? duration : (video.duration || 0);
      const clamped = Math.max(0, Math.min(seconds, targetDuration));
      video.currentTime = clamped;
      setCurrentTime(clamped);
    },
    [videoRef, duration]
  );

  const stepFrame = useCallback(
    (deltaFrames: number) => {
      const frameDuration = 1 / (fps > 0 ? fps : 30);
      seek(currentTime + deltaFrames * frameDuration);
    },
    [currentTime, fps, seek]
  );

  const jumpToStart = useCallback(() => {
    seek(0);
  }, [seek]);

  const jumpToEnd = useCallback(() => {
    const targetDuration = duration > 0 ? duration : (videoRef.current?.duration || 0);
    seek(Math.max(0, targetDuration - 0.05));
  }, [duration, videoRef, seek]);

  const setVolume = useCallback(
    (vol: number) => {
      const video = videoRef.current;
      const clamped = Math.max(0, Math.min(1, vol));
      setVolumeState(clamped);
      if (video) {
        video.volume = clamped;
        if (clamped > 0 && video.muted) {
          video.muted = false;
          setIsMuted(false);
        }
      }
    },
    [videoRef]
  );

  const toggleMute = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    const nextMuted = !video.muted;
    video.muted = nextMuted;
    setIsMuted(nextMuted);
  }, [videoRef]);

  const triggerProxyGeneration = useCallback(async () => {
    if (!asset) return;
    try {
      const res = await requestPreviewProxy(asset.id);
      setProxyState(res.status);
      if (res.status.state === "generating") {
        startProxyPolling(asset.id);
      } else if (res.status.state === "ready") {
        const freshInfo = await getMediaPlaybackUrl(asset.id);
        setPlaybackInfo(freshInfo);
        setError(null);
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(`Proxy generation request failed: ${msg}`);
    }
  }, [asset, startProxyPolling]);

  const cancelProxyGeneration = useCallback(async () => {
    if (!asset) return;
    try {
      await cancelPreviewProxy(asset.id);
      stopProxyPolling();
      setProxyState({ state: "cancelled" });
    } catch (err: unknown) {
      console.error("Failed to cancel proxy generation:", err);
    }
  }, [asset, stopProxyPolling]);

  return {
    playbackInfo,
    isLoading,
    isPlaying,
    isBuffering,
    isSeeking,
    currentTime,
    duration,
    volume,
    isMuted,
    error,
    isEnded,
    canPlay,
    fps,
    aspectRatio,
    proxyState,
    togglePlay,
    play,
    pause,
    seek,
    stepFrame,
    jumpToStart,
    jumpToEnd,
    setVolume,
    toggleMute,
    triggerProxyGeneration,
    cancelProxyGeneration,
  };
}
