//! Data contracts and domain models for the AI Voice Library.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai::tts::models::TtsVoiceSettings;

pub type VoiceId = String;

/// Availability and readiness status of a voice in the local environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VoiceAvailabilityStatus {
    /// Present in catalog but model artifacts are not downloaded.
    AvailableForInstall,
    /// Model files installed, but required inference runtime binary is missing.
    InstalledRuntimeMissing,
    /// Model files installed, but artifact integrity verification failed.
    InstalledVerificationRequired,
    /// Model is verified, installed, and the TTS engine is fully operational.
    Ready,
    /// Model or runtime is incompatible with the host environment.
    Incompatible,
    /// The requested text/subtitles language is not supported by this voice.
    UnsupportedLanguage,
    /// Voice is currently unavailable or corrupt.
    Unavailable,
}

/// Catalog entry describing a verified offline voice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCatalogEntry {
    /// Unique stable identifier for the voice (e.g. "piper-en-lessac-medium").
    pub id: VoiceId,
    /// Human-friendly name (e.g. "Lessac (English - US)").
    pub display_name: String,
    /// Underlying AI Model ID in the shared model manager.
    pub model_id: String,
    /// Required inference runtime backend (e.g. "piper").
    pub runtime_backend: String,
    /// Primary language code (ISO 639-1, e.g. "en", "bn", "es").
    pub language: String,
    /// Full BCP 47 locale tag (e.g. "en-US", "bn-BD", "es-ES").
    pub locale: String,
    /// Model release version tag.
    pub version: String,
    /// Upstream defined quality tier ("low", "medium", "high").
    pub quality_tier: String,
    /// Combined total size of all required model artifacts in bytes.
    pub artifact_size_bytes: u64,
    /// Official upstream source repository URL.
    pub source_url: String,
    /// License expression (e.g. "Public Domain", "MIT", "Apache-2.0", "CC-BY-4.0").
    pub license: String,
    /// Attribution requirements and dataset provenance.
    pub attribution: String,
    /// Number of distinct speaker identities supported by this voice model.
    pub speaker_count: u32,
    /// List of valid speaker IDs (e.g. `[0]` for single-speaker, `[0, 1, ...]` for multi-speaker).
    pub supported_speaker_ids: Vec<u32>,
    /// Native audio sample rate in Hz (e.g. 22050).
    pub sample_rate: u32,
    /// Human-readable description.
    pub description: String,
    /// Upstream-documented accent, if known.
    pub accent: Option<String>,
}

/// Server-evaluated compatibility and readiness report for a voice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCompatibility {
    pub voice_id: VoiceId,
    pub status: VoiceAvailabilityStatus,
    pub is_ready: bool,
    pub is_installed: bool,
    pub runtime_ready: bool,
    pub supported_languages: Vec<String>,
    pub speaker_count: u32,
    pub warnings: Vec<String>,
}

/// User's persistent voice preferences and favorites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicePreferences {
    pub schema_version: u32,
    /// Set of favorited voice IDs.
    pub favorite_voice_ids: Vec<String>,
    /// User's preferred global default voice ID.
    pub preferred_voice_id: Option<String>,
    /// Chronological list of recently used voice IDs (most recent first).
    pub recent_voice_ids: Vec<String>,
    /// Last chosen speaker ID per voice ID.
    pub last_speaker_ids: HashMap<String, u32>,
    /// Last updated timestamp (epoch milliseconds).
    pub updated_at_epoch_ms: u64,
}

impl Default for VoicePreferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            favorite_voice_ids: Vec::new(),
            preferred_voice_id: None,
            recent_voice_ids: Vec::new(),
            last_speaker_ids: HashMap::new(),
            updated_at_epoch_ms: 0,
        }
    }
}

/// Project-level binding assigning a voice to a subtitle track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceAssignment {
    /// Subtitle track ID in the project.
    pub track_id: String,
    /// Assigned voice ID.
    pub voice_id: VoiceId,
    /// Target language of the assignment.
    pub language: String,
    /// Optional speaker ID variant for multi-speaker models.
    pub speaker_id: Option<u32>,
    /// Timestamp when assigned (epoch milliseconds).
    pub assigned_at_epoch_ms: u64,
}

/// Collection of voice assignments persisted inside SubtitleDocument metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVoiceAssignments {
    /// Project-level fallback/default voice.
    pub default_voice_id: Option<VoiceId>,
    /// Explicit per-track assignments.
    pub track_assignments: Vec<VoiceAssignment>,
}

/// Client request to generate or retrieve a synthesized voice preview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicePreviewRequest {
    pub voice_id: VoiceId,
    /// Optional custom text to speak. Defaults to "Welcome to Open Media Workbench."
    pub custom_text: Option<String>,
    /// Optional speaker ID variant. Defaults to 0.
    pub speaker_id: Option<u32>,
    /// Optional voice synthesis settings.
    pub settings: Option<TtsVoiceSettings>,
}

/// Synthesized voice preview result containing audio provenance and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoicePreviewResult {
    /// Unique preview hash / cache identifier.
    pub preview_id: String,
    /// Voice ID used.
    pub voice_id: VoiceId,
    /// Text synthesized.
    pub text: String,
    /// Staged WAV file path on disk.
    pub staged_audio_path: String,
    /// Audio sample rate in Hz.
    pub sample_rate: u32,
    /// Audio channel count.
    pub channel_count: u16,
    /// Audio duration in microseconds.
    pub duration_us: u64,
    /// Whether this result was served directly from the preview cache.
    pub cached: bool,
    /// Timestamp generated (epoch milliseconds).
    pub created_at_epoch_ms: u64,
}

/// Request to synthesize and compare two voices side-by-side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceComparisonRequest {
    pub voice_id_a: VoiceId,
    pub voice_id_b: VoiceId,
    pub text: Option<String>,
}

/// Result of comparing two voices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceComparisonResult {
    pub voice_a: VoicePreviewResult,
    pub voice_b: VoicePreviewResult,
    /// Duration difference in microseconds: `duration_a - duration_b`.
    pub duration_diff_us: i64,
}
