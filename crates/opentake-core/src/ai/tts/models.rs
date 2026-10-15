//! Data contracts for Text-to-Speech synthesis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Settings controlling the voice synthesis output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoiceSettings {
    /// Optional speaker index for multi-speaker models.
    pub speaker_id: Option<u32>,
    /// Speed scale factor (0.5 to 2.0, default: 1.0).
    pub speed_scale: f32,
    /// Noise scale factor (phoneme variation, default: 0.667).
    pub noise_scale: f32,
    /// Noise width scale factor (phoneme width variation, default: 0.8).
    pub noise_w: f32,
}

impl Default for TtsVoiceSettings {
    fn default() -> Self {
        Self {
            speaker_id: None,
            speed_scale: 1.0,
            noise_scale: 0.667,
            noise_w: 0.8,
        }
    }
}

impl TtsVoiceSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.speed_scale < 0.25 || self.speed_scale > 4.0 {
            return Err("Speed scale must be between 0.25 and 4.0".into());
        }
        if self.noise_scale < 0.0 || self.noise_scale > 2.0 {
            return Err("Noise scale must be between 0.0 and 2.0".into());
        }
        if self.noise_w < 0.0 || self.noise_w > 2.0 {
            return Err("Noise width must be between 0.0 and 2.0".into());
        }
        Ok(())
    }
}

/// Source type for speech synthesis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TtsSource {
    Script {
        text: String,
    },
    SubtitleTrack {
        track_id: String,
        cue_ids: Option<Vec<String>>,
    },
}

/// Incoming client request for offline speech synthesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsRequest {
    pub source: TtsSource,
    pub voice_id: String,
    pub language: String,
    #[serde(default)]
    pub settings: TtsVoiceSettings,
}

/// Current stage in the synthesis job pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TtsProgressStage {
    Queued,
    CheckingVoice,
    PreparingText,
    LoadingModel,
    Synthesizing,
    Validating,
    Completed,
    Failed,
    Cancelled,
}

/// Status report for an active or completed synthesis job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsStatus {
    pub job_id: String,
    pub stage: TtsProgressStage,
    pub completed_segments: usize,
    pub total_segments: usize,
    pub progress_pct: f64,
    pub current_message: String,
    pub error: Option<String>,
}

/// Audio metadata and timing information for a synthesized segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsAudioSegment {
    /// Unique stable identifier for this segment.
    pub id: String,
    /// Associated source subtitle cue ID, if synthesized from a track.
    pub source_cue_id: Option<String>,
    /// The text that was synthesized.
    pub original_text: String,
    /// The voice ID used for synthesis.
    pub voice_id: String,
    /// Audio sample rate in Hz (e.g. 22050).
    pub sample_rate: u32,
    /// Number of audio channels (e.g. 1 for mono).
    pub channel_count: u16,
    /// Total audio samples count.
    pub sample_count: u64,
    /// Measured duration in microseconds.
    pub duration_us: u64,
    /// Start timestamp of source cue in microseconds, if applicable.
    pub cue_start_us: Option<u64>,
    /// End timestamp of source cue in microseconds, if applicable.
    pub cue_end_us: Option<u64>,
    /// Difference between generated audio duration and subtitle cue duration (in microseconds).
    /// Positive = audio is longer than subtitle display interval.
    /// Negative = audio is shorter than subtitle display interval.
    pub timing_difference_us: Option<i64>,
    /// Path to the staged WAV file inside application storage.
    pub staged_audio_path: String,
    /// Non-fatal diagnostics or warnings (e.g. duration mismatch).
    pub warnings: Vec<String>,
}

/// Complete synthesis result for a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsResult {
    pub schema_version: u32,
    pub job_id: String,
    pub source_type: String,
    pub voice_id: String,
    pub runtime: String,
    pub segments: Vec<TtsAudioSegment>,
    pub total_duration_us: u64,
    pub created_at_epoch_ms: u64,
    pub warnings: Vec<String>,
}

/// Hardware and runtime capabilities report for the local TTS backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsRuntimeCapabilities {
    pub discovered: bool,
    pub binary_path: Option<String>,
    pub version: Option<String>,
    pub supported_formats: Vec<String>,
    pub error: Option<String>,
}

/// Catalog voice manifest describing a verified voice model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiVoiceManifest {
    pub id: String,
    pub display_name: String,
    pub language: String,
    pub locale: String,
    pub voice_family: String,
    pub model_id: String,
    pub sample_rate: u32,
    pub speaker_count: u32,
    pub is_installed: bool,
    pub runtime_ready: bool,
    pub license: String,
    pub description: String,
}

/// Structured input passed to the low-level TTS engine adapter.
#[derive(Debug, Clone)]
pub struct TtsSynthesisInput {
    pub text: String,
    pub voice_id: String,
    pub model_path: PathBuf,
    pub config_path: PathBuf,
    pub settings: TtsVoiceSettings,
}

/// Validated audio metadata parsed from a generated WAV file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavAudioMetadata {
    pub sample_rate: u32,
    pub channel_count: u16,
    pub bits_per_sample: u16,
    pub sample_count: u64,
    pub duration_us: u64,
    pub is_silent: bool,
}
