use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::error::MediaEngineError;
use super::identity::MediaAsset;
use super::waveform::{
    WaveformBin, WaveformData, WaveformLodLevel, WAVEFORM_ALGORITHM_VERSION, WAVEFORM_SAMPLE_RATE,
};

/// File header magic for OpenTake Waveform binary format.
pub const WAVEFORM_CACHE_MAGIC: &[u8; 4] = b"OTWF";

/// Schema version for binary waveform cache layout.
pub const WAVEFORM_CACHE_SCHEMA_VERSION: u32 = 1;

/// Computes a deterministic SHA-256 cache key identifying the audio content and extraction parameters.
pub fn compute_waveform_cache_key(asset: &MediaAsset, stream_index: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"opentake-waveform-v1:");
    hasher.update(asset.source.canonical_path.as_bytes());
    hasher.update(asset.source.file_size.to_le_bytes());
    if let Some(mtime) = asset.source.modified_epoch_ms {
        hasher.update(mtime.to_le_bytes());
    }
    hasher.update((stream_index as u64).to_le_bytes());
    hasher.update(WAVEFORM_SAMPLE_RATE.to_le_bytes());
    hasher.update(WAVEFORM_ALGORITHM_VERSION.to_le_bytes());
    hasher.update(WAVEFORM_CACHE_SCHEMA_VERSION.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

/// Returns the file path for a cached waveform file.
pub fn get_waveform_cache_path(cache_dir: &Path, cache_key: &str) -> PathBuf {
    cache_dir.join(format!("{cache_key}.otwf"))
}

/// Atomically persists waveform data into the local disk cache.
pub fn save_waveform_to_cache(
    cache_dir: &Path,
    cache_key: &str,
    data: &WaveformData,
) -> Result<PathBuf, MediaEngineError> {
    fs::create_dir_all(cache_dir).map_err(|e| {
        MediaEngineError::Internal(format!("Failed to create waveform cache directory: {e}"))
    })?;

    let temp_path = cache_dir.join(format!("{cache_key}.tmp.otwf"));
    let final_path = get_waveform_cache_path(cache_dir, cache_key);

    let file = File::create(&temp_path).map_err(|e| {
        MediaEngineError::Internal(format!(
            "Failed to create temporary waveform cache file: {e}"
        ))
    })?;

    let mut writer = BufWriter::new(file);

    // 1. Magic & Versions
    writer
        .write_all(WAVEFORM_CACHE_MAGIC)
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    writer
        .write_all(&WAVEFORM_CACHE_SCHEMA_VERSION.to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    writer
        .write_all(&data.algorithm_version.to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;

    // 2. Stream Metadata
    writer
        .write_all(&(data.stream_index as u32).to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    writer
        .write_all(&data.sample_rate.to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    writer
        .write_all(&data.channels.to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    writer
        .write_all(&data.duration_micros.to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    writer
        .write_all(&data.start_time_micros.to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    writer
        .write_all(&data.total_samples.to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;

    // 3. LOD Levels
    writer
        .write_all(&(data.levels.len() as u32).to_le_bytes())
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;

    for level in &data.levels {
        writer
            .write_all(&level.level.to_le_bytes())
            .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
        writer
            .write_all(&level.bins_per_second.to_le_bytes())
            .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
        writer
            .write_all(&level.samples_per_bin.to_le_bytes())
            .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
        writer
            .write_all(&(level.bins.len() as u32).to_le_bytes())
            .map_err(|e| MediaEngineError::Internal(e.to_string()))?;

        for bin in &level.bins {
            writer
                .write_all(&bin.min.to_le_bytes())
                .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
            writer
                .write_all(&bin.max.to_le_bytes())
                .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
            writer
                .write_all(&bin.rms.to_le_bytes())
                .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
            writer
                .write_all(&bin.sample_count.to_le_bytes())
                .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
        }
    }

    writer
        .flush()
        .map_err(|e| MediaEngineError::Internal(e.to_string()))?;
    drop(writer);

    // Atomically commit temporary file to final cache destination
    fs::rename(&temp_path, &final_path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        MediaEngineError::Internal(format!("Failed to finalize waveform cache file: {e}"))
    })?;

    Ok(final_path)
}

/// Loads and validates cached waveform data from disk. Returns `Ok(None)` if cache is missing or corrupt.
pub fn load_waveform_from_cache(
    cache_dir: &Path,
    cache_key: &str,
    asset_id: &str,
) -> Result<Option<WaveformData>, MediaEngineError> {
    let final_path = get_waveform_cache_path(cache_dir, cache_key);
    if !final_path.exists() {
        return Ok(None);
    }

    let file = match File::open(&final_path) {
        Ok(f) => f,
        Err(_) => return Ok(None),
    };

    let mut reader = BufReader::new(file);

    // Helper macro to read fixed byte array
    let mut magic = [0u8; 4];
    if reader.read_exact(&mut magic).is_err() || &magic != WAVEFORM_CACHE_MAGIC {
        let _ = fs::remove_file(&final_path);
        return Ok(None);
    }

    let mut u32_buf = [0u8; 4];
    let mut u64_buf = [0u8; 8];

    // Schema version
    if reader.read_exact(&mut u32_buf).is_err()
        || u32::from_le_bytes(u32_buf) != WAVEFORM_CACHE_SCHEMA_VERSION
    {
        let _ = fs::remove_file(&final_path);
        return Ok(None);
    }

    // Algorithm version
    if reader.read_exact(&mut u32_buf).is_err()
        || u32::from_le_bytes(u32_buf) != WAVEFORM_ALGORITHM_VERSION
    {
        let _ = fs::remove_file(&final_path);
        return Ok(None);
    }
    let algorithm_version = u32::from_le_bytes(u32_buf);

    // Stream metadata
    if reader.read_exact(&mut u32_buf).is_err() {
        return Ok(None);
    }
    let stream_index = u32::from_le_bytes(u32_buf) as usize;

    if reader.read_exact(&mut u32_buf).is_err() {
        return Ok(None);
    }
    let sample_rate = u32::from_le_bytes(u32_buf);

    if reader.read_exact(&mut u32_buf).is_err() {
        return Ok(None);
    }
    let channels = u32::from_le_bytes(u32_buf);

    if reader.read_exact(&mut u64_buf).is_err() {
        return Ok(None);
    }
    let duration_micros = u64::from_le_bytes(u64_buf);

    if reader.read_exact(&mut u64_buf).is_err() {
        return Ok(None);
    }
    let start_time_micros = i64::from_le_bytes(u64_buf);

    if reader.read_exact(&mut u64_buf).is_err() {
        return Ok(None);
    }
    let total_samples = u64::from_le_bytes(u64_buf);

    // Level count
    if reader.read_exact(&mut u32_buf).is_err() {
        return Ok(None);
    }
    let level_count = u32::from_le_bytes(u32_buf);

    let mut levels = Vec::with_capacity(level_count as usize);

    for _ in 0..level_count {
        if reader.read_exact(&mut u32_buf).is_err() {
            return Ok(None);
        }
        let level = u32::from_le_bytes(u32_buf);

        if reader.read_exact(&mut u64_buf).is_err() {
            return Ok(None);
        }
        let bins_per_second = f64::from_le_bytes(u64_buf);

        if reader.read_exact(&mut u32_buf).is_err() {
            return Ok(None);
        }
        let samples_per_bin = u32::from_le_bytes(u32_buf);

        if reader.read_exact(&mut u32_buf).is_err() {
            return Ok(None);
        }
        let bin_count = u32::from_le_bytes(u32_buf) as usize;

        let mut bins = Vec::with_capacity(bin_count);
        for _ in 0..bin_count {
            if reader.read_exact(&mut u32_buf).is_err() {
                return Ok(None);
            }
            let min = f32::from_le_bytes(u32_buf);

            if reader.read_exact(&mut u32_buf).is_err() {
                return Ok(None);
            }
            let max = f32::from_le_bytes(u32_buf);

            if reader.read_exact(&mut u32_buf).is_err() {
                return Ok(None);
            }
            let rms = f32::from_le_bytes(u32_buf);

            if reader.read_exact(&mut u32_buf).is_err() {
                return Ok(None);
            }
            let sample_count = u32::from_le_bytes(u32_buf);

            bins.push(WaveformBin {
                min,
                max,
                rms,
                sample_count,
            });
        }

        levels.push(WaveformLodLevel {
            level,
            bins_per_second,
            samples_per_bin,
            bins,
        });
    }

    Ok(Some(WaveformData {
        asset_id: asset_id.to_string(),
        stream_index,
        sample_rate,
        channels,
        duration_micros,
        start_time_micros,
        total_samples,
        levels,
        algorithm_version,
    }))
}

/// Cleans up any leftover temporary `.tmp.otwf` file from an interrupted analysis.
pub fn cleanup_temp_waveform_file(cache_dir: &Path, cache_key: &str) {
    let temp_path = cache_dir.join(format!("{cache_key}.tmp.otwf"));
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_waveform_cache_roundtrip() {
        let temp_dir = tempdir().unwrap();
        let cache_key = "abc123def456";

        let sample_bin = WaveformBin {
            min: -0.85,
            max: 0.92,
            rms: 0.44,
            sample_count: 160,
        };

        let lod0 = WaveformLodLevel {
            level: 0,
            bins_per_second: 100.0,
            samples_per_bin: 160,
            bins: vec![sample_bin; 50],
        };

        let original_data = WaveformData {
            asset_id: "asset_test_1".to_string(),
            stream_index: 0,
            sample_rate: 16_000,
            channels: 1,
            duration_micros: 500_000,
            start_time_micros: 0,
            total_samples: 8_000,
            levels: vec![lod0],
            algorithm_version: 1,
        };

        // Save
        let saved_path =
            save_waveform_to_cache(temp_dir.path(), cache_key, &original_data).unwrap();
        assert!(saved_path.exists());

        // Load
        let loaded = load_waveform_from_cache(temp_dir.path(), cache_key, "asset_test_1")
            .unwrap()
            .expect("Should load successfully");

        assert_eq!(loaded.asset_id, "asset_test_1");
        assert_eq!(loaded.total_samples, 8_000);
        assert_eq!(loaded.levels.len(), 1);
        assert_eq!(loaded.levels[0].bins.len(), 50);
        assert_eq!(loaded.levels[0].bins[0].min, -0.85);
        assert_eq!(loaded.levels[0].bins[0].max, 0.92);
    }

    #[test]
    fn test_corrupt_cache_file_recovery() {
        let temp_dir = tempdir().unwrap();
        let cache_key = "corrupt_key";
        let cache_path = get_waveform_cache_path(temp_dir.path(), cache_key);

        // Write corrupt bytes
        fs::write(&cache_path, b"INVALID_GARBAGE_BYTES").unwrap();

        let loaded = load_waveform_from_cache(temp_dir.path(), cache_key, "test_asset").unwrap();
        assert!(loaded.is_none());
        assert!(
            !cache_path.exists(),
            "Corrupt cache file should have been deleted"
        );
    }
}
