use serde::{Deserialize, Serialize};

use super::metadata::MediaMetadata;

/// Explicit media compatibility classification level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilityStatus {
    /// Fully supported format for inspection, future preview, and editing.
    Supported,
    /// Inspectable and editable, but may require proxy generation or have preview caveats.
    SupportedWithLimitations,
    /// Format can be inspected, but editing or playback is unsupported in OpenTake.
    Unsupported,
    /// Invalid container, missing streams, or corrupted media structure.
    Invalid,
}

/// Comprehensive compatibility assessment for an imported media asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCompatibility {
    pub overall: CompatibilityStatus,
    pub container_status: CompatibilityStatus,
    pub video_status: CompatibilityStatus,
    pub audio_status: CompatibilityStatus,
    pub can_inspect: bool,
    pub can_edit_future: bool,
    pub warnings: Vec<String>,
}

impl MediaCompatibility {
    /// Evaluates media metadata and returns a structured compatibility assessment.
    pub fn evaluate(metadata: &MediaMetadata) -> Self {
        let mut warnings = Vec::new();

        // 1. Check if media has any content
        if metadata.video_streams.is_empty() && metadata.audio_streams.is_empty() {
            return Self {
                overall: CompatibilityStatus::Invalid,
                container_status: CompatibilityStatus::Invalid,
                video_status: CompatibilityStatus::Invalid,
                audio_status: CompatibilityStatus::Invalid,
                can_inspect: true,
                can_edit_future: false,
                warnings: vec![
                    "Media file contains no recognizable video or audio streams.".to_string(),
                ],
            };
        }

        // 2. Container evaluation
        let format_lower = metadata.container.format_name.to_lowercase();
        let container_status = if format_lower.contains("mp4")
            || format_lower.contains("mov")
            || format_lower.contains("quicktime")
            || format_lower.contains("m4a")
            || format_lower.contains("matroska")
            || format_lower.contains("webm")
            || format_lower.contains("mkv")
        {
            CompatibilityStatus::Supported
        } else if format_lower.contains("avi")
            || format_lower.contains("mpegts")
            || format_lower.contains("flv")
            || format_lower.contains("ogg")
            || format_lower.contains("wav")
        {
            warnings.push(format!(
                "Container format '{}' may require remuxing for smooth playback.",
                metadata.container.format_name
            ));
            CompatibilityStatus::SupportedWithLimitations
        } else {
            warnings.push(format!(
                "Uncommon container format '{}' detected.",
                metadata.container.format_name
            ));
            CompatibilityStatus::Unsupported
        };

        // 3. Video evaluation
        let video_status = if let Some(video) = metadata.primary_video() {
            let codec = video.codec_name.to_lowercase();
            if video.is_vfr_suspect {
                warnings.push(
                    "Variable frame rate (VFR) detected. Timeline seek sync may require transcoding."
                        .to_string(),
                );
            }
            if video.width > 3840 || video.height > 2160 {
                warnings.push(
                    "High-resolution media (>4K) detected. Proxy preview is recommended."
                        .to_string(),
                );
            }

            match codec.as_str() {
                "h264" | "avc1" => CompatibilityStatus::Supported,
                "vp9" | "av1" => CompatibilityStatus::Supported,
                "hevc" | "h265" => {
                    warnings.push(
                        "H.265 / HEVC video codec may require hardware acceleration or platform decoders."
                            .to_string(),
                    );
                    CompatibilityStatus::SupportedWithLimitations
                }
                "prores" | "dnxhr" => {
                    warnings.push(
                        "Professional intermediate codec detected. Playback may be computationally intensive."
                            .to_string(),
                    );
                    CompatibilityStatus::SupportedWithLimitations
                }
                "vp8" | "mpeg4" | "mjpeg" => {
                    warnings.push(format!("Legacy video codec '{codec}' detected."));
                    CompatibilityStatus::SupportedWithLimitations
                }
                _ => {
                    warnings.push(format!(
                        "Unsupported or unrecognized video codec '{codec}'."
                    ));
                    CompatibilityStatus::Unsupported
                }
            }
        } else {
            // Audio-only media is supported
            warnings.push("Media file contains no video stream (audio-only).".to_string());
            CompatibilityStatus::Supported
        };

        // 4. Audio evaluation
        let audio_status = if let Some(audio) = metadata.primary_audio() {
            let codec = audio.codec_name.to_lowercase();
            match codec.as_str() {
                "aac" | "mp3" | "opus" | "flac" => CompatibilityStatus::Supported,
                s if s.starts_with("pcm_") => CompatibilityStatus::Supported,
                "vorbis" | "alac" => CompatibilityStatus::Supported,
                "ac3" | "eac3" => {
                    warnings.push(
                        "Dolby Digital audio (AC-3) may require surround downmixing.".to_string(),
                    );
                    CompatibilityStatus::SupportedWithLimitations
                }
                _ => {
                    warnings.push(format!(
                        "Unsupported or unrecognized audio codec '{codec}'."
                    ));
                    CompatibilityStatus::Unsupported
                }
            }
        } else {
            // Video-only media is supported
            warnings.push("Media file contains no audio stream.".to_string());
            CompatibilityStatus::Supported
        };

        // 5. Overall status determination
        let overall = if container_status == CompatibilityStatus::Unsupported
            || video_status == CompatibilityStatus::Unsupported
            || audio_status == CompatibilityStatus::Unsupported
        {
            CompatibilityStatus::Unsupported
        } else if container_status == CompatibilityStatus::SupportedWithLimitations
            || video_status == CompatibilityStatus::SupportedWithLimitations
            || audio_status == CompatibilityStatus::SupportedWithLimitations
        {
            CompatibilityStatus::SupportedWithLimitations
        } else {
            CompatibilityStatus::Supported
        };

        let can_edit_future = overall == CompatibilityStatus::Supported
            || overall == CompatibilityStatus::SupportedWithLimitations;

        Self {
            overall,
            container_status,
            video_status,
            audio_status,
            can_inspect: true,
            can_edit_future,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::metadata::{AudioStreamMetadata, ContainerMetadata, VideoStreamMetadata};

    #[test]
    fn test_compatibility_h264_aac_mp4() {
        let meta = MediaMetadata {
            container: ContainerMetadata {
                format_name: "mov,mp4,m4a,3gp,3g2,mj2".to_string(),
                format_long_name: None,
                duration_micros: Some(10_000_000),
                start_time_micros: Some(0),
                bitrate_bps: Some(5_000_000),
                size_bytes: 1024 * 1024,
                stream_count: 2,
            },
            video_streams: vec![VideoStreamMetadata {
                stream_index: 0,
                codec_name: "h264".to_string(),
                codec_long_name: None,
                width: 1920,
                height: 1080,
                pixel_format: Some("yuv420p".to_string()),
                sample_aspect_ratio: None,
                display_aspect_ratio: None,
                rotation_degrees: None,
                average_frame_rate: None,
                nominal_frame_rate: None,
                timebase: None,
                duration_micros: Some(10_000_000),
                profile: None,
                is_vfr_suspect: false,
            }],
            audio_streams: vec![AudioStreamMetadata {
                stream_index: 1,
                codec_name: "aac".to_string(),
                codec_long_name: None,
                sample_rate: 48000,
                channels: 2,
                channel_layout: Some("stereo".to_string()),
                bitrate_bps: Some(192000),
                duration_micros: Some(10_000_000),
                timebase: None,
            }],
            other_streams: vec![],
        };

        let compat = MediaCompatibility::evaluate(&meta);
        assert_eq!(compat.overall, CompatibilityStatus::Supported);
        assert_eq!(compat.container_status, CompatibilityStatus::Supported);
        assert_eq!(compat.video_status, CompatibilityStatus::Supported);
        assert_eq!(compat.audio_status, CompatibilityStatus::Supported);
        assert!(compat.can_edit_future);
        assert!(compat.warnings.is_empty());
    }

    #[test]
    fn test_compatibility_hevc_limitations() {
        let meta = MediaMetadata {
            container: ContainerMetadata {
                format_name: "matroska,webm".to_string(),
                format_long_name: None,
                duration_micros: Some(5_000_000),
                start_time_micros: None,
                bitrate_bps: None,
                size_bytes: 2048,
                stream_count: 1,
            },
            video_streams: vec![VideoStreamMetadata {
                stream_index: 0,
                codec_name: "hevc".to_string(),
                codec_long_name: None,
                width: 3840,
                height: 2160,
                pixel_format: None,
                sample_aspect_ratio: None,
                display_aspect_ratio: None,
                rotation_degrees: None,
                average_frame_rate: None,
                nominal_frame_rate: None,
                timebase: None,
                duration_micros: None,
                profile: None,
                is_vfr_suspect: false,
            }],
            audio_streams: vec![],
            other_streams: vec![],
        };

        let compat = MediaCompatibility::evaluate(&meta);
        assert_eq!(
            compat.overall,
            CompatibilityStatus::SupportedWithLimitations
        );
        assert_eq!(
            compat.video_status,
            CompatibilityStatus::SupportedWithLimitations
        );
        assert!(compat.can_inspect);
        assert!(compat.can_edit_future);
        assert!(!compat.warnings.is_empty());
    }
}
