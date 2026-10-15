//! Data models for word-level alignment, provenance, quality ratings, and segmentation policies.

use serde::{Deserialize, Serialize};

use crate::editing::time::MediaTimeUs;

/// Provenance metadata indicating how a word's timestamp boundaries were established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WordTimingProvenance {
    /// Derived from native Whisper token/segment timing output.
    #[default]
    WhisperNativeEstimate,
    /// Derived from an explicit acoustic forced-alignment pass.
    ForcedAlignment,
    /// Manually modified or nudged by the user in the subtitle editor.
    UserAdjusted,
    /// Text token exists in transcript but lacks valid speech timing boundaries.
    Unaligned,
}

/// Overall classification of alignment accuracy and completeness for a result set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimingQuality {
    /// All words possess verified, non-zero timing boundaries.
    Aligned,
    /// Timestamps are estimated by model tokens or segment subdivisions.
    #[default]
    Estimated,
    /// Some words possess timing boundaries while others are unaligned.
    PartiallyAligned,
    /// No word-level timing boundaries could be established.
    Unavailable,
}

/// A canonical single spoken word with microsecond timing, confidence, and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignedWord {
    /// Stable identifier for this word instance (e.g. "w_0_3").
    pub id: String,
    /// Clean linguistic word text (Unicode preserved).
    pub text: String,
    /// Inclusive speech start timestamp in integer microseconds ($start\_us \ge 0$).
    pub start_us: MediaTimeUs,
    /// Exclusive speech end timestamp in integer microseconds ($end\_us > start\_us$).
    pub end_us: MediaTimeUs,
    /// Associated parent transcription segment identifier if originated from Whisper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_segment_id: Option<String>,
    /// Associated canonical subtitle cue identifier when partitioned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_cue_id: Option<String>,
    /// Acoustic decoder confidence score ($0.0 \le p \le 1.0$) if provided by runtime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Provenance origin of this word's timing.
    pub provenance: WordTimingProvenance,
    /// Optional punctuation prefix attached to word (e.g. `“`, `¿`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation_prefix: Option<String>,
    /// Optional punctuation suffix attached to word (e.g. `,`, `.`, `?`, `!`, `।`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation_suffix: Option<String>,
}

impl PartialEq for AlignedWord {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.text == other.text
            && self.start_us == other.start_us
            && self.end_us == other.end_us
            && self.source_segment_id == other.source_segment_id
            && self.source_cue_id == other.source_cue_id
            && self.provenance == other.provenance
            && self.punctuation_prefix == other.punctuation_prefix
            && self.punctuation_suffix == other.punctuation_suffix
            && match (self.confidence, other.confidence) {
                (Some(a), Some(b)) => a.to_bits() == b.to_bits(),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for AlignedWord {}

impl AlignedWord {
    /// Constructs a new AlignedWord with validated timestamp ordering.
    pub fn new(
        id: impl Into<String>,
        text: impl Into<String>,
        start_us: MediaTimeUs,
        end_us: MediaTimeUs,
        provenance: WordTimingProvenance,
    ) -> Result<Self, super::errors::AlignmentError> {
        let start = start_us.as_micros();
        let end = end_us.as_micros();

        if start < 0 {
            return Err(super::errors::AlignmentError::NegativeTime { time_us: start });
        }
        if end <= start {
            let text_str = text.into();
            return Err(super::errors::AlignmentError::InvalidWordTimestamp {
                word: text_str,
                start_us: start,
                end_us: end,
            });
        }

        Ok(Self {
            id: id.into(),
            text: text.into(),
            start_us,
            end_us,
            source_segment_id: None,
            source_cue_id: None,
            confidence: None,
            provenance,
            punctuation_prefix: None,
            punctuation_suffix: None,
        })
    }

    /// Evaluates if a given playback time falls within this word's spoken interval $[start\_us, end\_us)$.
    #[inline]
    pub fn contains_time(&self, time_us: MediaTimeUs) -> bool {
        self.start_us <= time_us && time_us < self.end_us
    }

    /// Returns the duration of this word in integer microseconds.
    #[inline]
    pub fn duration_us(&self) -> MediaTimeUs {
        MediaTimeUs(self.end_us.as_micros() - self.start_us.as_micros())
    }

    /// Returns display text including any punctuation prefixes and suffixes.
    pub fn display_text(&self) -> String {
        format!(
            "{}{}{}",
            self.punctuation_prefix.as_deref().unwrap_or(""),
            self.text,
            self.punctuation_suffix.as_deref().unwrap_or("")
        )
    }
}

/// Configurable heuristics guiding smart subtitle cue segmentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentationPolicy {
    /// Maximum characters allowed per subtitle line (standard broadcast recommendation: 37-42).
    pub max_chars_per_line: usize,
    /// Maximum lines allowed per subtitle cue (standard: 2).
    pub max_lines_per_cue: usize,
    /// Maximum allowed cue duration in integer microseconds (default: 7,000,000 µs = 7s).
    pub max_cue_duration_us: i64,
    /// Minimum allowed cue duration in integer microseconds (default: 1,000,000 µs = 1s).
    pub min_cue_duration_us: i64,
    /// Maximum reading speed threshold in characters per second (default: 21.0 chars/sec).
    pub max_chars_per_second: f32,
    /// Silence pause duration in microseconds that triggers a natural cue split (default: 400,000 µs = 400ms).
    pub split_on_silence_gap_us: i64,
    /// Whether to force a cue break on sentence-ending punctuation (`.`, `?`, `!`, `।`, `—`).
    pub split_on_punctuation: bool,
}

impl Default for SegmentationPolicy {
    fn default() -> Self {
        Self {
            max_chars_per_line: 42,
            max_lines_per_cue: 2,
            max_cue_duration_us: 7_000_000, // 7 seconds
            min_cue_duration_us: 1_000_000, // 1 second
            max_chars_per_second: 21.0,
            split_on_silence_gap_us: 400_000, // 400ms pause
            split_on_punctuation: true,
        }
    }
}

/// Complete alignment result containing ordered words, quality rating, and audit warnings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignmentResult {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Associated source media asset identifier.
    pub source_asset_id: String,
    /// Originating transcription job ID.
    pub job_id: String,
    /// Model used during recognition/alignment.
    pub model_id: String,
    /// IETF BCP 47 language code.
    pub language: String,
    /// Quality rating of the timing evidence.
    pub timing_quality: TimingQuality,
    /// Chronologically ordered sequence of aligned words.
    pub words: Vec<AlignedWord>,
    /// Non-fatal warnings generated during alignment or segmentation.
    pub warnings: Vec<String>,
}
