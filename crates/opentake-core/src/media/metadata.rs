use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::error::MediaEngineError;

/// An exact rational fraction representing frame rates, aspect ratios, and timebases.
/// Denominator must never be zero for a valid fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rational {
    pub num: u32,
    pub den: u32,
}

impl Rational {
    pub fn new(num: u32, den: u32) -> Option<Self> {
        if den == 0 {
            None
        } else {
            Some(Self { num, den })
        }
    }

    /// Converts to floating-point value for UI presentation purposes only.
    /// Never use this float as the canonical timing source in the editing engine.
    pub fn to_f64_lossy(&self) -> f64 {
        if self.den == 0 {
            0.0
        } else {
            self.num as f64 / self.den as f64
        }
    }

    /// Parses FFprobe string representations like "30000/1001", "24/1", or "0/0".
    pub fn parse_fraction(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        match parts.as_slice() {
            [num_str, den_str] => {
                let num = num_str.trim().parse::<u32>().ok()?;
                let den = den_str.trim().parse::<u32>().ok()?;
                if den == 0 || (num == 0 && den <= 1) {
                    // "0/0" or "0/1" indicates unknown or invalid rate
                    None
                } else {
                    Some(Self { num, den })
                }
            }
            [num_str] => {
                let num = num_str.trim().parse::<u32>().ok()?;
                if num == 0 {
                    None
                } else {
                    Some(Self { num, den: 1 })
                }
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

/// Normalized container format metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerMetadata {
    pub format_name: String,
    pub format_long_name: Option<String>,
    pub duration_micros: Option<u64>,
    pub start_time_micros: Option<i64>,
    pub bitrate_bps: Option<u64>,
    pub size_bytes: u64,
    pub stream_count: usize,
}

/// Normalized video stream metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStreamMetadata {
    pub stream_index: usize,
    pub codec_name: String,
    pub codec_long_name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub pixel_format: Option<String>,
    pub sample_aspect_ratio: Option<String>,
    pub display_aspect_ratio: Option<String>,
    pub rotation_degrees: Option<i32>,
    pub average_frame_rate: Option<Rational>,
    pub nominal_frame_rate: Option<Rational>,
    pub timebase: Option<Rational>,
    pub duration_micros: Option<u64>,
    pub profile: Option<String>,
    pub is_vfr_suspect: bool,
}

/// Normalized audio stream metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStreamMetadata {
    pub stream_index: usize,
    pub codec_name: String,
    pub codec_long_name: Option<String>,
    pub sample_rate: u32,
    pub channels: u32,
    pub channel_layout: Option<String>,
    pub bitrate_bps: Option<u64>,
    pub duration_micros: Option<u64>,
    pub timebase: Option<Rational>,
}

/// Metadata for secondary streams (subtitles, data, attachments).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherStreamMetadata {
    pub stream_index: usize,
    pub codec_type: String,
    pub codec_name: Option<String>,
}

/// Full normalized media file inspection metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub container: ContainerMetadata,
    pub video_streams: Vec<VideoStreamMetadata>,
    pub audio_streams: Vec<AudioStreamMetadata>,
    pub other_streams: Vec<OtherStreamMetadata>,
}

impl MediaMetadata {
    /// Returns the primary video stream if present.
    pub fn primary_video(&self) -> Option<&VideoStreamMetadata> {
        self.video_streams.first()
    }

    /// Returns the primary audio stream if present.
    pub fn primary_audio(&self) -> Option<&AudioStreamMetadata> {
        self.audio_streams.first()
    }

    /// Computes overall media duration in microseconds, prioritizing container duration
    /// then falling back to video stream or audio stream duration.
    pub fn duration_micros(&self) -> Option<u64> {
        self.container
            .duration_micros
            .or_else(|| self.video_streams.iter().find_map(|v| v.duration_micros))
            .or_else(|| self.audio_streams.iter().find_map(|a| a.duration_micros))
    }
}

// ---------------------------------------------------------------------------
// Internal Raw FFprobe JSON deserialization structures
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RawFfprobeOutput {
    pub format: Option<RawFormat>,
    #[serde(default)]
    pub streams: Vec<RawStream>,
}

#[derive(Debug, Deserialize)]
pub struct RawFormat {
    pub format_name: Option<String>,
    pub format_long_name: Option<String>,
    pub duration: Option<String>,
    pub start_time: Option<String>,
    pub size: Option<String>,
    pub bit_rate: Option<String>,
    pub nb_streams: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RawStream {
    pub index: Option<usize>,
    pub codec_name: Option<String>,
    pub codec_long_name: Option<String>,
    pub codec_type: Option<String>,
    pub profile: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub pix_fmt: Option<String>,
    pub sample_aspect_ratio: Option<String>,
    pub display_aspect_ratio: Option<String>,
    pub r_frame_rate: Option<String>,
    pub avg_frame_rate: Option<String>,
    pub time_base: Option<String>,
    pub duration: Option<String>,
    pub bit_rate: Option<String>,
    pub sample_rate: Option<String>,
    pub channels: Option<u32>,
    pub channel_layout: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub side_data_list: Option<Vec<serde_json::Value>>,
}

/// Converts seconds string (e.g. "12.345678") to integer microseconds.
fn parse_seconds_to_micros(s: &str) -> Option<u64> {
    let secs = f64::from_str(s.trim()).ok()?;
    if secs < 0.0 || secs.is_nan() || secs.is_infinite() {
        None
    } else {
        Some((secs * 1_000_000.0).round() as u64)
    }
}

/// Converts seconds string to signed integer microseconds.
fn parse_seconds_to_signed_micros(s: &str) -> Option<i64> {
    let secs = f64::from_str(s.trim()).ok()?;
    if secs.is_nan() || secs.is_infinite() {
        None
    } else {
        Some((secs * 1_000_000.0).round() as i64)
    }
}

/// Extracts rotation angle in degrees from tags or side data.
fn extract_rotation(stream: &RawStream) -> Option<i32> {
    // 1. Check stream tags
    if let Some(tags) = &stream.tags {
        if let Some(rotate_val) = tags.get("rotate") {
            if let Some(num) = rotate_val.as_i64() {
                return Some(num as i32);
            }
            if let Some(s) = rotate_val.as_str() {
                if let Ok(num) = s.parse::<i32>() {
                    return Some(num);
                }
            }
        }
    }

    // 2. Check side_data_list (Display Matrix)
    if let Some(side_data) = &stream.side_data_list {
        for item in side_data {
            if item.get("side_data_type").and_then(|v| v.as_str()) == Some("Display Matrix") {
                if let Some(rotation) = item.get("rotation").and_then(|r| r.as_i64()) {
                    return Some(rotation as i32);
                }
            }
        }
    }

    None
}

/// Normalizes raw FFprobe output into safe, domain-typed `MediaMetadata`.
pub fn normalize_ffprobe_json(
    raw_json: &str,
    fallback_file_size: u64,
) -> Result<MediaMetadata, MediaEngineError> {
    let parsed: RawFfprobeOutput =
        serde_json::from_str(raw_json).map_err(|e| MediaEngineError::OutputParseFailed {
            field: "ffprobe_json".to_string(),
            raw: format!("Invalid FFprobe JSON: {}", e),
        })?;

    let raw_format = parsed.format.unwrap_or(RawFormat {
        format_name: None,
        format_long_name: None,
        duration: None,
        start_time: None,
        size: None,
        bit_rate: None,
        nb_streams: None,
    });

    let format_name = raw_format
        .format_name
        .unwrap_or_else(|| "unknown".to_string());
    let format_long_name = raw_format.format_long_name;
    let duration_micros = raw_format
        .duration
        .as_deref()
        .and_then(parse_seconds_to_micros);
    let start_time_micros = raw_format
        .start_time
        .as_deref()
        .and_then(parse_seconds_to_signed_micros);
    let bitrate_bps = raw_format
        .bit_rate
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok());
    let size_bytes = raw_format
        .size
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(fallback_file_size);
    let stream_count = raw_format.nb_streams.unwrap_or(parsed.streams.len());

    let container = ContainerMetadata {
        format_name,
        format_long_name,
        duration_micros,
        start_time_micros,
        bitrate_bps,
        size_bytes,
        stream_count,
    };

    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();
    let mut other_streams = Vec::new();

    for (idx, stream) in parsed.streams.into_iter().enumerate() {
        let stream_index = stream.index.unwrap_or(idx);
        let rotation_degrees = extract_rotation(&stream);
        let codec_type = stream
            .codec_type
            .as_deref()
            .unwrap_or("unknown")
            .to_lowercase();
        let codec_name = stream
            .codec_name
            .as_deref()
            .unwrap_or("unknown")
            .to_lowercase();

        match codec_type.as_str() {
            "video" => {
                let width = stream.width.unwrap_or(0);
                let height = stream.height.unwrap_or(0);
                let average_frame_rate = stream
                    .avg_frame_rate
                    .as_deref()
                    .and_then(Rational::parse_fraction);
                let nominal_frame_rate = stream
                    .r_frame_rate
                    .as_deref()
                    .and_then(Rational::parse_fraction);
                let timebase = stream
                    .time_base
                    .as_deref()
                    .and_then(Rational::parse_fraction);
                let duration_micros = stream.duration.as_deref().and_then(parse_seconds_to_micros);

                // Detect VFR heuristic: both frame rates exist and differ
                let is_vfr_suspect = match (average_frame_rate, nominal_frame_rate) {
                    (Some(avg), Some(nom)) => {
                        let diff = (avg.to_f64_lossy() - nom.to_f64_lossy()).abs();
                        diff > 0.05
                    }
                    _ => false,
                };

                video_streams.push(VideoStreamMetadata {
                    stream_index,
                    codec_name,
                    codec_long_name: stream.codec_long_name,
                    width,
                    height,
                    pixel_format: stream.pix_fmt,
                    sample_aspect_ratio: stream.sample_aspect_ratio,
                    display_aspect_ratio: stream.display_aspect_ratio,
                    rotation_degrees,
                    average_frame_rate,
                    nominal_frame_rate,
                    timebase,
                    duration_micros,
                    profile: stream.profile,
                    is_vfr_suspect,
                });
            }
            "audio" => {
                let sample_rate = stream
                    .sample_rate
                    .as_deref()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                let channels = stream.channels.unwrap_or(0);
                let bitrate_bps = stream
                    .bit_rate
                    .as_deref()
                    .and_then(|s| s.parse::<u64>().ok());
                let duration_micros = stream.duration.as_deref().and_then(parse_seconds_to_micros);
                let timebase = stream
                    .time_base
                    .as_deref()
                    .and_then(Rational::parse_fraction);

                audio_streams.push(AudioStreamMetadata {
                    stream_index,
                    codec_name,
                    codec_long_name: stream.codec_long_name,
                    sample_rate,
                    channels,
                    channel_layout: stream.channel_layout,
                    bitrate_bps,
                    duration_micros,
                    timebase,
                });
            }
            other => {
                other_streams.push(OtherStreamMetadata {
                    stream_index,
                    codec_type: other.to_string(),
                    codec_name: if codec_name == "unknown" {
                        None
                    } else {
                        Some(codec_name)
                    },
                });
            }
        }
    }

    Ok(MediaMetadata {
        container,
        video_streams,
        audio_streams,
        other_streams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rational_parsing() {
        let r = Rational::parse_fraction("30000/1001").unwrap();
        assert_eq!(r.num, 30000);
        assert_eq!(r.den, 1001);
        assert!((r.to_f64_lossy() - 29.97).abs() < 0.01);

        let r2 = Rational::parse_fraction("25/1").unwrap();
        assert_eq!(r2.num, 25);
        assert_eq!(r2.den, 1);
        assert_eq!(r2.to_f64_lossy(), 25.0);

        // Invalid cases
        assert!(Rational::parse_fraction("0/0").is_none());
        assert!(Rational::parse_fraction("0/1").is_none());
        assert!(Rational::parse_fraction("10/0").is_none());
        assert!(Rational::parse_fraction("invalid").is_none());
    }

    #[test]
    fn test_normalize_ffprobe_json_full() {
        let sample_json = r#"{
            "format": {
                "format_name": "mov,mp4,m4a,3gp,3g2,mj2",
                "format_long_name": "QuickTime / MOV",
                "duration": "14.240000",
                "start_time": "0.000000",
                "size": "5242880",
                "bit_rate": "2945438",
                "nb_streams": 2
            },
            "streams": [
                {
                    "index": 0,
                    "codec_name": "h264",
                    "codec_long_name": "H.264 / AVC / MPEG-4 AVC / MPEG-4 part 10",
                    "codec_type": "video",
                    "width": 1920,
                    "height": 1080,
                    "pix_fmt": "yuv420p",
                    "r_frame_rate": "30/1",
                    "avg_frame_rate": "30/1",
                    "time_base": "1/30000",
                    "duration": "14.233333",
                    "tags": {
                        "rotate": "90"
                    }
                },
                {
                    "index": 1,
                    "codec_name": "aac",
                    "codec_long_name": "AAC (Advanced Audio Coding)",
                    "codec_type": "audio",
                    "sample_rate": "48000",
                    "channels": 2,
                    "channel_layout": "stereo",
                    "bit_rate": "192000",
                    "time_base": "1/48000",
                    "duration": "14.240000"
                }
            ]
        }"#;

        let meta = normalize_ffprobe_json(sample_json, 5242880).unwrap();
        assert_eq!(meta.container.format_name, "mov,mp4,m4a,3gp,3g2,mj2");
        assert_eq!(meta.container.duration_micros, Some(14_240_000));
        assert_eq!(meta.container.size_bytes, 5242880);

        assert_eq!(meta.video_streams.len(), 1);
        let v = &meta.video_streams[0];
        assert_eq!(v.codec_name, "h264");
        assert_eq!(v.width, 1920);
        assert_eq!(v.height, 1080);
        assert_eq!(v.rotation_degrees, Some(90));
        assert_eq!(v.average_frame_rate.unwrap().num, 30);
        assert_eq!(v.nominal_frame_rate.unwrap().num, 30);
        assert!(!v.is_vfr_suspect);

        assert_eq!(meta.audio_streams.len(), 1);
        let a = &meta.audio_streams[0];
        assert_eq!(a.codec_name, "aac");
        assert_eq!(a.sample_rate, 48000);
        assert_eq!(a.channels, 2);
        assert_eq!(a.channel_layout.as_deref(), Some("stereo"));
    }

    #[test]
    fn test_normalize_empty_or_minimal_json() {
        let minimal_json = r#"{
            "format": {
                "format_name": "matroska,webm"
            },
            "streams": []
        }"#;

        let meta = normalize_ffprobe_json(minimal_json, 1024).unwrap();
        assert_eq!(meta.container.format_name, "matroska,webm");
        assert_eq!(meta.container.duration_micros, None);
        assert_eq!(meta.video_streams.len(), 0);
        assert_eq!(meta.audio_streams.len(), 0);
    }
}
