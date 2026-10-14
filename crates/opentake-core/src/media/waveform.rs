use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::discovery::{discover_tool, DiscoveryConfig, ToolType};
use super::error::MediaEngineError;
use super::identity::MediaAsset;
use super::process::{run_streaming_process, ProcessSpec};
use super::waveform_cache::{
    cleanup_temp_waveform_file, compute_waveform_cache_key, load_waveform_from_cache,
    save_waveform_to_cache,
};

/// Analysis sample rate for speech and waveform energy extraction (16 kHz).
pub const WAVEFORM_SAMPLE_RATE: u32 = 16_000;

/// Base resolution for LOD 0: 100 bins per second (10 ms per bin = 160 samples at 16 kHz).
pub const BASE_BINS_PER_SECOND: f64 = 100.0;
pub const SAMPLES_PER_BASE_BIN: u32 = 160;

/// Aggregation factor between consecutive LOD pyramid levels.
pub const LOD_AGGREGATION_FACTOR: usize = 10;

/// Maximum bins returned in a single IPC segment response to strictly bound memory and payload size.
pub const MAX_SEGMENT_BINS: usize = 4_000;

/// Current algorithm version to support cache invalidation when DSP changes.
pub const WAVEFORM_ALGORITHM_VERSION: u32 = 1;

/// A single aggregated audio amplitude bin containing peak envelope and RMS energy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformBin {
    /// Normalized minimum sample in this time interval [-1.0, 1.0].
    pub min: f32,
    /// Normalized maximum sample in this time interval [-1.0, 1.0].
    pub max: f32,
    /// Normalized Root Mean Square (RMS) amplitude [0.0, 1.0].
    pub rms: f32,
    /// Number of raw audio samples represented in this bin.
    pub sample_count: u32,
}

impl Default for WaveformBin {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 0.0,
            rms: 0.0,
            sample_count: 0,
        }
    }
}

/// A single Level of Detail (LOD) tier in the multi-resolution waveform pyramid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformLodLevel {
    /// Level index: 0 = Fine (100 bins/s), 1 = Medium (10 bins/s), 2 = Overview (1 bin/s).
    pub level: u32,
    /// Bins per second of audio for this resolution tier.
    pub bins_per_second: f64,
    /// Raw audio samples aggregated into each bin at this tier.
    pub samples_per_bin: u32,
    /// Pre-calculated amplitude bins for the full audio stream.
    pub bins: Vec<WaveformBin>,
}

/// Complete multi-LOD waveform dataset for an analyzed media asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    pub asset_id: String,
    pub stream_index: usize,
    pub sample_rate: u32,
    pub channels: u32,
    pub duration_micros: u64,
    pub start_time_micros: i64,
    pub total_samples: u64,
    pub levels: Vec<WaveformLodLevel>,
    pub algorithm_version: u32,
}

/// A slice of waveform bins covering a requested time window at the optimal LOD tier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformSegment {
    pub asset_id: String,
    pub lod_level: u32,
    pub bins_per_second: f64,
    pub start_micros: i64,
    pub end_micros: i64,
    pub bins: Vec<WaveformBin>,
}

/// Accumulator that consumes streaming raw 16-bit signed PCM chunks (`s16le`)
/// and computes LOD 0 waveform bins with $O(1)$ memory overhead.
pub struct PcmSampleAccumulator {
    samples_per_bin: u32,
    current_min: f32,
    current_max: f32,
    current_sum_sq: f64,
    current_count: u32,
    total_samples: u64,
    leftover_byte: Option<u8>,
    bins: Vec<WaveformBin>,
}

impl PcmSampleAccumulator {
    pub fn new(samples_per_bin: u32) -> Self {
        Self {
            samples_per_bin,
            current_min: 1.0,
            current_max: -1.0,
            current_sum_sq: 0.0,
            current_count: 0,
            total_samples: 0,
            leftover_byte: None,
            bins: Vec::new(),
        }
    }

    /// Ingests a slice of raw little-endian 16-bit PCM bytes.
    pub fn ingest_pcm_chunk(&mut self, chunk: &[u8]) -> Result<(), MediaEngineError> {
        let mut idx = 0;

        // If there was a trailing odd byte from previous chunk, combine it with first byte
        if let Some(prev_byte) = self.leftover_byte.take() {
            if !chunk.is_empty() {
                let sample_bytes = [prev_byte, chunk[0]];
                let raw_sample = i16::from_le_bytes(sample_bytes);
                self.process_single_sample(raw_sample);
                idx = 1;
            } else {
                self.leftover_byte = Some(prev_byte);
                return Ok(());
            }
        }

        // Process 2-byte sample pairs
        while idx + 1 < chunk.len() {
            let sample_bytes = [chunk[idx], chunk[idx + 1]];
            let raw_sample = i16::from_le_bytes(sample_bytes);
            self.process_single_sample(raw_sample);
            idx += 2;
        }

        // Store any trailing odd byte
        if idx < chunk.len() {
            self.leftover_byte = Some(chunk[idx]);
        }

        Ok(())
    }

    #[inline]
    fn process_single_sample(&mut self, raw: i16) {
        // Normalize to [-1.0, 1.0]
        let s = (raw as f32) / 32768.0;
        let s_clamped = s.clamp(-1.0, 1.0);

        if s_clamped < self.current_min {
            self.current_min = s_clamped;
        }
        if s_clamped > self.current_max {
            self.current_max = s_clamped;
        }

        self.current_sum_sq += (s_clamped as f64).powi(2);
        self.current_count += 1;
        self.total_samples += 1;

        if self.current_count >= self.samples_per_bin {
            self.finalize_current_bin();
        }
    }

    fn finalize_current_bin(&mut self) {
        if self.current_count == 0 {
            return;
        }

        let rms = (self.current_sum_sq / (self.current_count as f64))
            .sqrt()
            .clamp(0.0, 1.0) as f32;

        self.bins.push(WaveformBin {
            min: self.current_min,
            max: self.current_max,
            rms,
            sample_count: self.current_count,
        });

        self.current_min = 1.0;
        self.current_max = -1.0;
        self.current_sum_sq = 0.0;
        self.current_count = 0;
    }

    /// Finalizes processing, flushing any trailing partial bin, and builds the multi-LOD pyramid.
    pub fn finish(
        mut self,
        asset_id: String,
        stream_index: usize,
        sample_rate: u32,
        channels: u32,
        duration_micros: u64,
        start_time_micros: i64,
    ) -> WaveformData {
        // Flush remaining samples in active bin
        if self.current_count > 0 {
            self.finalize_current_bin();
        }

        // Handle case where media is completely empty
        if self.bins.is_empty() {
            self.bins.push(WaveformBin::default());
        }

        let lod0 = WaveformLodLevel {
            level: 0,
            bins_per_second: BASE_BINS_PER_SECOND,
            samples_per_bin: self.samples_per_bin,
            bins: self.bins,
        };

        // Build LOD 1 (10x aggregation: 10 bins/sec)
        let lod1 = aggregate_lod_level(&lod0, 1, LOD_AGGREGATION_FACTOR);

        // Build LOD 2 (10x aggregation from LOD 1: 1 bin/sec)
        let lod2 = aggregate_lod_level(&lod1, 2, LOD_AGGREGATION_FACTOR);

        WaveformData {
            asset_id,
            stream_index,
            sample_rate,
            channels,
            duration_micros,
            start_time_micros,
            total_samples: self.total_samples,
            levels: vec![lod0, lod1, lod2],
            algorithm_version: WAVEFORM_ALGORITHM_VERSION,
        }
    }
}

/// Aggregates a base LOD level into a coarser resolution tier using sample-weighted squared energy.
pub fn aggregate_lod_level(
    source_level: &WaveformLodLevel,
    target_level_idx: u32,
    factor: usize,
) -> WaveformLodLevel {
    let mut aggregated_bins = Vec::with_capacity(source_level.bins.len().div_ceil(factor));

    for chunk in source_level.bins.chunks(factor) {
        let mut min_val = 1.0f32;
        let mut max_val = -1.0f32;
        let mut total_sample_count = 0u32;
        let mut total_weighted_energy = 0.0f64;

        for bin in chunk {
            if bin.min < min_val {
                min_val = bin.min;
            }
            if bin.max > max_val {
                max_val = bin.max;
            }
            total_sample_count += bin.sample_count;
            total_weighted_energy += (bin.sample_count as f64) * (bin.rms as f64).powi(2);
        }

        let rms = if total_sample_count > 0 {
            (total_weighted_energy / (total_sample_count as f64))
                .sqrt()
                .clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        aggregated_bins.push(WaveformBin {
            min: min_val,
            max: max_val,
            rms,
            sample_count: total_sample_count,
        });
    }

    if aggregated_bins.is_empty() {
        aggregated_bins.push(WaveformBin::default());
    }

    WaveformLodLevel {
        level: target_level_idx,
        bins_per_second: source_level.bins_per_second / (factor as f64),
        samples_per_bin: source_level.samples_per_bin * (factor as u32),
        bins: aggregated_bins,
    }
}

impl WaveformData {
    /// Extracts a bounded slice of bins for the requested source time range and target pixel width.
    pub fn get_segment(
        &self,
        start_micros: i64,
        end_micros: i64,
        target_pixel_width: u32,
    ) -> WaveformSegment {
        let max_end = self.start_time_micros + (self.duration_micros as i64);
        let clamped_start = start_micros.clamp(self.start_time_micros, max_end);
        let clamped_end = end_micros.clamp(clamped_start, max_end);

        let span_micros = (clamped_end - clamped_start).max(1);
        let span_seconds = (span_micros as f64) / 1_000_000.0;
        let target_pixels = (target_pixel_width as f64).max(10.0);

        let required_bins_per_sec = target_pixels / span_seconds;

        // Select optimal LOD
        let lod = if required_bins_per_sec > 25.0 {
            &self.levels[0] // LOD 0: 100 bins/sec
        } else if required_bins_per_sec > 2.5 && self.levels.len() > 1 {
            &self.levels[1] // LOD 1: 10 bins/sec
        } else if self.levels.len() > 2 {
            &self.levels[2] // LOD 2: 1 bin/sec
        } else {
            self.levels.last().unwrap()
        };

        let start_offset_sec =
            ((clamped_start - self.start_time_micros) as f64 / 1_000_000.0).max(0.0);
        let end_offset_sec = ((clamped_end - self.start_time_micros) as f64 / 1_000_000.0).max(0.0);

        let start_idx =
            ((start_offset_sec * lod.bins_per_second).floor() as usize).min(lod.bins.len());
        let end_idx =
            ((end_offset_sec * lod.bins_per_second).ceil() as usize).min(lod.bins.len());

        let raw_slice = if start_idx < end_idx {
            &lod.bins[start_idx..end_idx]
        } else {
            &[]
        };

        // Strictly bound response size to MAX_SEGMENT_BINS
        let bins = if raw_slice.len() <= MAX_SEGMENT_BINS {
            raw_slice.to_vec()
        } else {
            let step = raw_slice.len().div_ceil(MAX_SEGMENT_BINS);
            raw_slice
                .chunks(step)
                .map(|chunk| {
                    let mut min_val = 1.0f32;
                    let mut max_val = -1.0f32;
                    let mut total_samples = 0u32;
                    let mut energy = 0.0f64;
                    for b in chunk {
                        if b.min < min_val {
                            min_val = b.min;
                        }
                        if b.max > max_val {
                            max_val = b.max;
                        }
                        total_samples += b.sample_count;
                        energy += (b.sample_count as f64) * (b.rms as f64).powi(2);
                    }
                    let rms = if total_samples > 0 {
                        (energy / total_samples as f64).sqrt() as f32
                    } else {
                        0.0
                    };
                    WaveformBin {
                        min: min_val,
                        max: max_val,
                        rms,
                        sample_count: total_samples,
                    }
                })
                .collect()
        };

        WaveformSegment {
            asset_id: self.asset_id.clone(),
            lod_level: lod.level,
            bins_per_second: lod.bins_per_second,
            start_micros: clamped_start,
            end_micros: clamped_end,
            bins,
        }
    }
}

/// Headless engine managing streaming PCM extraction, peak/RMS analysis, and local caching.
#[derive(Debug, Clone)]
pub struct WaveformEngine {
    pub cache_dir: PathBuf,
    pub ffmpeg_path: Option<PathBuf>,
}

impl WaveformEngine {
    pub fn new(cache_dir: PathBuf, ffmpeg_path: Option<PathBuf>) -> Self {
        Self {
            cache_dir,
            ffmpeg_path,
        }
    }

    /// Computes the deterministic cache key identifying this asset's waveform analysis.
    pub fn compute_cache_key(&self, asset: &MediaAsset, stream_index: usize) -> String {
        compute_waveform_cache_key(asset, stream_index)
    }

    /// Checks if a valid, existing waveform is already available in the disk cache.
    pub fn find_cached_waveform(
        &self,
        asset: &MediaAsset,
        stream_index: usize,
    ) -> Result<Option<WaveformData>, MediaEngineError> {
        let cache_key = self.compute_cache_key(asset, stream_index);
        load_waveform_from_cache(&self.cache_dir, &cache_key, &asset.id)
    }

    /// Extracts PCM audio via FFmpeg and aggregates into a multi-LOD waveform dataset.
    ///
    /// # Bounded Streaming Guarantee
    /// Consumes raw 16 kHz mono signed 16-bit PCM over stdout in bounded 64 KiB chunks
    /// without ever collecting raw audio into memory.
    pub async fn extract_waveform(
        &self,
        asset: &MediaAsset,
        stream_index: usize,
        cancellation: Option<&CancellationToken>,
    ) -> Result<WaveformData, MediaEngineError> {
        // 1. Guard against audio-free media
        if asset.metadata.audio_streams.is_empty() {
            return Err(MediaEngineError::NoAudioTrack {
                id: asset.id.clone(),
            });
        }

        // 2. Check existing cache first
        let cache_key = self.compute_cache_key(asset, stream_index);
        if let Ok(Some(cached)) = load_waveform_from_cache(&self.cache_dir, &cache_key, &asset.id) {
            return Ok(cached);
        }

        // 3. Resolve ffmpeg executable
        let ffmpeg_binary = match &self.ffmpeg_path {
            Some(p) => p.clone(),
            None => {
                let discovered = discover_tool(ToolType::Ffmpeg, &DiscoveryConfig::default())?;
                discovered.canonical_path
            }
        };

        // 4. Construct FFmpeg argument vector with mandatory `--` guard
        let audio_map = format!("0:a:{stream_index}?");
        let args = vec![
            "-y".to_string(),
            "-v".to_string(),
            "error".to_string(),
            "-i".to_string(),
            asset.source.canonical_path.clone(),
            "-map".to_string(),
            audio_map,
            "-vn".to_string(),
            "-sn".to_string(),
            "-dn".to_string(),
            "-ac".to_string(),
            "1".to_string(),
            "-ar".to_string(),
            WAVEFORM_SAMPLE_RATE.to_string(),
            "-f".to_string(),
            "s16le".to_string(),
            "-".to_string(),
        ];

        let mut spec = ProcessSpec::new(ffmpeg_binary, args, Duration::from_secs(600));
        spec.max_stderr_bytes = 256 * 1024;

        let mut accumulator = PcmSampleAccumulator::new(SAMPLES_PER_BASE_BIN);

        // 5. Run streaming extraction
        let stream_res = run_streaming_process(spec, cancellation, |chunk| {
            accumulator.ingest_pcm_chunk(chunk)
        })
        .await;

        match stream_res {
            Ok(_) => {
                let duration_micros = asset.metadata.duration_micros().unwrap_or(0);
                let start_time_micros = asset.metadata.container.start_time_micros.unwrap_or(0);

                let waveform_data = accumulator.finish(
                    asset.id.clone(),
                    stream_index,
                    WAVEFORM_SAMPLE_RATE,
                    1,
                    duration_micros,
                    start_time_micros,
                );

                // Persist to disk cache
                let _ = save_waveform_to_cache(&self.cache_dir, &cache_key, &waveform_data);

                Ok(waveform_data)
            }
            Err(MediaEngineError::Cancelled { .. }) => {
                cleanup_temp_waveform_file(&self.cache_dir, &cache_key);
                Err(MediaEngineError::WaveformCancelled {
                    id: asset.id.clone(),
                })
            }
            Err(e) => {
                cleanup_temp_waveform_file(&self.cache_dir, &cache_key);
                Err(MediaEngineError::WaveformExtractionFailed {
                    id: asset.id.clone(),
                    reason: e.user_message(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_accumulator_sine_wave() {
        let mut acc = PcmSampleAccumulator::new(160);
        let mut pcm_bytes = Vec::new();

        // Generate 1 second of 440Hz sine wave at 16kHz
        for i in 0..16_000 {
            let phase = 2.0 * std::f64::consts::PI * 440.0 * (i as f64) / 16_000.0;
            let sample = (phase.sin() * 16384.0) as i16;
            pcm_bytes.extend_from_slice(&sample.to_le_bytes());
        }

        acc.ingest_pcm_chunk(&pcm_bytes).unwrap();
        let waveform = acc.finish("test_asset".to_string(), 0, 16_000, 1, 1_000_000, 0);

        assert_eq!(waveform.levels.len(), 3);
        assert_eq!(waveform.levels[0].bins.len(), 100); // 100 bins for 1 sec
        assert_eq!(waveform.levels[1].bins.len(), 10); // 10 bins for 1 sec
        assert_eq!(waveform.levels[2].bins.len(), 1); // 1 bin for 1 sec

        // Amplitude of half-scale sine should be ~0.5 peak and ~0.353 RMS
        let bin0 = waveform.levels[0].bins[0];
        assert!(bin0.max > 0.45 && bin0.max <= 0.55);
        assert!(bin0.min < -0.45 && bin0.min >= -0.55);
        assert!(bin0.rms > 0.30 && bin0.rms < 0.40);
    }

    #[test]
    fn test_pcm_accumulator_silence() {
        let mut acc = PcmSampleAccumulator::new(160);
        let pcm_bytes = vec![0u8; 3200]; // 1600 zero samples = 10 bins

        acc.ingest_pcm_chunk(&pcm_bytes).unwrap();
        let waveform = acc.finish("silence".to_string(), 0, 16_000, 1, 100_000, 0);

        for bin in &waveform.levels[0].bins {
            assert_eq!(bin.min, 0.0);
            assert_eq!(bin.max, 0.0);
            assert_eq!(bin.rms, 0.0);
        }
    }

    #[test]
    fn test_partial_sample_chunk_split() {
        let mut acc = PcmSampleAccumulator::new(10);
        // Odd chunk split: sample is 2 bytes (e.g. 0x01, 0x02)
        // Send first byte in chunk 1, second byte in chunk 2
        acc.ingest_pcm_chunk(&[0x01]).unwrap();
        acc.ingest_pcm_chunk(&[0x02]).unwrap();
        let waveform = acc.finish("split".to_string(), 0, 16_000, 1, 1000, 0);
        assert_eq!(waveform.total_samples, 1);
    }

    #[test]
    fn test_pcm_clipping_and_extreme_values() {
        let mut acc = PcmSampleAccumulator::new(2);
        let mut pcm_bytes = Vec::new();
        // Positive clip (32767) and negative clip (-32768)
        pcm_bytes.extend_from_slice(&i16::MAX.to_le_bytes());
        pcm_bytes.extend_from_slice(&i16::MIN.to_le_bytes());

        acc.ingest_pcm_chunk(&pcm_bytes).unwrap();
        let waveform = acc.finish("clip_asset".to_string(), 0, 16_000, 1, 125, 0);

        assert_eq!(waveform.levels[0].bins.len(), 1);
        let bin = waveform.levels[0].bins[0];
        assert!((bin.max - 1.0).abs() < 0.001);
        assert_eq!(bin.min, -1.0);
        assert!((bin.rms - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_short_trailing_window_preserved() {
        let mut acc = PcmSampleAccumulator::new(160);
        // Feed 165 samples (1 full bin of 160 + 5 trailing samples)
        let mut pcm_bytes = Vec::new();
        for _ in 0..165 {
            pcm_bytes.extend_from_slice(&8000i16.to_le_bytes());
        }

        acc.ingest_pcm_chunk(&pcm_bytes).unwrap();
        let waveform = acc.finish("trailing".to_string(), 0, 16_000, 1, 10_312, 0);

        assert_eq!(waveform.levels[0].bins.len(), 2);
        assert_eq!(waveform.levels[0].bins[0].sample_count, 160);
        assert_eq!(waveform.levels[0].bins[1].sample_count, 5);
        assert_eq!(waveform.total_samples, 165);
    }

    #[test]
    fn test_lod_weighted_rms_aggregation() {
        // Test energy preservation when aggregating bins with different sample counts
        let bin1 = WaveformBin {
            min: -0.2,
            max: 0.5,
            rms: 0.4,
            sample_count: 100,
        };
        let bin2 = WaveformBin {
            min: -0.8,
            max: 0.3,
            rms: 0.2,
            sample_count: 300,
        };

        let lod0 = WaveformLodLevel {
            level: 0,
            bins_per_second: 100.0,
            samples_per_bin: 160,
            bins: vec![bin1, bin2],
        };

        let lod1 = aggregate_lod_level(&lod0, 1, 10);
        assert_eq!(lod1.bins.len(), 1);
        let merged = lod1.bins[0];

        assert_eq!(merged.min, -0.8);
        assert_eq!(merged.max, 0.5);
        assert_eq!(merged.sample_count, 400);

        // Expected RMS: sqrt((100 * 0.4^2 + 300 * 0.2^2) / 400) = sqrt((16 + 12) / 400) = sqrt(28/400) = sqrt(0.07) ≈ 0.264575
        let expected_rms = (0.07f32).sqrt();
        assert!((merged.rms - expected_rms).abs() < 1e-5);
    }

    #[test]
    fn test_waveform_get_segment_lod_selection() {
        // Construct a waveform of 100 seconds
        let bin = WaveformBin {
            min: -0.5,
            max: 0.5,
            rms: 0.3,
            sample_count: 160,
        };

        let lod0 = WaveformLodLevel {
            level: 0,
            bins_per_second: 100.0,
            samples_per_bin: 160,
            bins: vec![bin; 10_000], // 100 sec at 100 bins/s
        };

        let lod1 = aggregate_lod_level(&lod0, 1, 10); // 1,000 bins
        let lod2 = aggregate_lod_level(&lod1, 2, 10); // 100 bins

        let waveform = WaveformData {
            asset_id: "test_seg".to_string(),
            stream_index: 0,
            sample_rate: 16_000,
            channels: 1,
            duration_micros: 100_000_000,
            start_time_micros: 0,
            total_samples: 1_600_000,
            levels: vec![lod0, lod1, lod2],
            algorithm_version: 1,
        };

        // 1. High zoom: viewing 1 second with 800 target pixels -> should pick LOD 0
        let seg_zoomed = waveform.get_segment(0, 1_000_000, 800);
        assert_eq!(seg_zoomed.lod_level, 0);
        assert_eq!(seg_zoomed.bins.len(), 100);

        // 2. Overview: viewing all 100 seconds with 200 target pixels -> should pick LOD 2
        let seg_overview = waveform.get_segment(0, 100_000_000, 200);
        assert_eq!(seg_overview.lod_level, 2);
        assert_eq!(seg_overview.bins.len(), 100);

        // 3. Out-of-bounds start/end clamping
        let seg_clamped = waveform.get_segment(90_000_000, 150_000_000, 500);
        assert_eq!(seg_clamped.end_micros, 100_000_000);
    }
}
