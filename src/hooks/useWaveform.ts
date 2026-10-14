import { useState, useEffect, useRef, useCallback } from "react";
import type { MediaAsset } from "../types/media";
import type {
  WaveformSegment,
  WaveformStatus,
} from "../types/waveform";
import {
  cancelWaveformAnalysis,
  getWaveformSegment,
  getWaveformStatus,
  requestWaveformAnalysis,
} from "../lib/waveform";

export interface UseWaveformOptions {
  asset: MediaAsset | null;
  viewportStartMicros: number;
  viewportEndMicros: number;
  pixelWidth: number;
}

export interface UseWaveformReturn {
  status: WaveformStatus;
  segment: WaveformSegment | null;
  isLoading: boolean;
  error: string | null;
  retryAnalysis: () => Promise<void>;
  cancelAnalysis: () => Promise<void>;
}

export function useWaveform({
  asset,
  viewportStartMicros,
  viewportEndMicros,
  pixelWidth,
}: UseWaveformOptions): UseWaveformReturn {
  const [status, setStatus] = useState<WaveformStatus>({ state: "none" });
  const [segment, setSegment] = useState<WaveformSegment | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const activeAssetIdRef = useRef<string | null>(null);
  const pollTimerRef = useRef<number | null>(null);
  const fetchDebounceRef = useRef<number | null>(null);

  const stopPolling = useCallback(() => {
    if (pollTimerRef.current !== null) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
  }, []);

  const fetchSegment = useCallback(
    async (assetId: string, start: number, end: number, width: number) => {
      try {
        const res = await getWaveformSegment(assetId, start, end, width);
        if (activeAssetIdRef.current === assetId) {
          if (res.segment) {
            setSegment(res.segment);
          }
          setStatus(res.status);
        }
      } catch (err: unknown) {
        if (activeAssetIdRef.current === assetId) {
          console.warn("Failed to fetch waveform segment:", err);
        }
      }
    },
    []
  );

  const startPolling = useCallback(
    (assetId: string) => {
      stopPolling();
      pollTimerRef.current = window.setInterval(async () => {
        if (activeAssetIdRef.current !== assetId) {
          stopPolling();
          return;
        }
        try {
          const res = await getWaveformStatus(assetId);
          setStatus(res.status);

          if (res.status.state === "ready") {
            stopPolling();
            setIsLoading(false);
            fetchSegment(assetId, viewportStartMicros, viewportEndMicros, pixelWidth);
          } else if (
            res.status.state === "failed" ||
            res.status.state === "cancelled" ||
            res.status.state === "noAudio"
          ) {
            stopPolling();
            setIsLoading(false);
            if (res.status.state === "failed") {
              setError(res.status.error);
            }
          }
        } catch {
          stopPolling();
          setIsLoading(false);
        }
      }, 1000);
    },
    [stopPolling, fetchSegment, viewportStartMicros, viewportEndMicros, pixelWidth]
  );

  // Asset change / initialization effect
  useEffect(() => {
    stopPolling();

    if (!asset) {
      activeAssetIdRef.current = null;
      setStatus({ state: "none" });
      setSegment(null);
      setIsLoading(false);
      setError(null);
      return;
    }

    const currentId = asset.id;
    activeAssetIdRef.current = currentId;
    setError(null);
    setSegment(null);

    // If media has no audio tracks, transition immediately
    if (asset.metadata.audioStreams.length === 0) {
      setStatus({ state: "noAudio" });
      setIsLoading(false);
      return;
    }

    setIsLoading(true);

    async function initWaveform() {
      try {
        const statusRes = await getWaveformStatus(currentId);
        if (activeAssetIdRef.current !== currentId) return;

        setStatus(statusRes.status);

        if (statusRes.status.state === "ready") {
          setIsLoading(false);
          fetchSegment(currentId, viewportStartMicros, viewportEndMicros, pixelWidth);
        } else if (statusRes.status.state === "extracting") {
          startPolling(currentId);
        } else if (statusRes.status.state === "none") {
          // Auto-trigger extraction
          const reqRes = await requestWaveformAnalysis(currentId);
          if (activeAssetIdRef.current !== currentId) return;
          setStatus(reqRes.status);
          if (reqRes.status.state === "ready") {
            setIsLoading(false);
            fetchSegment(currentId, viewportStartMicros, viewportEndMicros, pixelWidth);
          } else {
            startPolling(currentId);
          }
        } else {
          setIsLoading(false);
          if (statusRes.status.state === "failed") {
            setError(statusRes.status.error);
          }
        }
      } catch (err: unknown) {
        if (activeAssetIdRef.current !== currentId) return;
        setIsLoading(false);
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
      }
    }

    initWaveform();

    return () => {
      stopPolling();
    };
  }, [asset, startPolling, stopPolling, fetchSegment, viewportStartMicros, viewportEndMicros, pixelWidth]);

  // Viewport / pixelWidth update effect
  useEffect(() => {
    if (!asset || status.state !== "ready") return;

    if (fetchDebounceRef.current !== null) {
      clearTimeout(fetchDebounceRef.current);
    }

    fetchDebounceRef.current = window.setTimeout(() => {
      fetchSegment(asset.id, viewportStartMicros, viewportEndMicros, pixelWidth);
    }, 50);

    return () => {
      if (fetchDebounceRef.current !== null) {
        clearTimeout(fetchDebounceRef.current);
      }
    };
  }, [asset, status.state, viewportStartMicros, viewportEndMicros, pixelWidth, fetchSegment]);

  const retryAnalysis = useCallback(async () => {
    if (!asset) return;
    setError(null);
    setIsLoading(true);
    try {
      const res = await requestWaveformAnalysis(asset.id);
      setStatus(res.status);
      if (res.status.state === "ready") {
        setIsLoading(false);
        fetchSegment(asset.id, viewportStartMicros, viewportEndMicros, pixelWidth);
      } else {
        startPolling(asset.id);
      }
    } catch (err: unknown) {
      setIsLoading(false);
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
    }
  }, [asset, startPolling, fetchSegment, viewportStartMicros, viewportEndMicros, pixelWidth]);

  const cancelAnalysis = useCallback(async () => {
    if (!asset) return;
    stopPolling();
    try {
      await cancelWaveformAnalysis(asset.id);
      setStatus({ state: "cancelled" });
      setIsLoading(false);
    } catch (err: unknown) {
      console.warn("Failed to cancel waveform analysis:", err);
    }
  }, [asset, stopPolling]);

  return {
    status,
    segment,
    isLoading,
    error,
    retryAnalysis,
    cancelAnalysis,
  };
}
