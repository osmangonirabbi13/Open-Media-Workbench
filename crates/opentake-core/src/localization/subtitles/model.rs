use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::SubtitleError;
use crate::editing::time::MediaTimeUs;

/// Maximum allowed subtitle file size (10 MB) to guard against unbounded parser memory consumption.
pub const MAX_SUBTITLE_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum number of subtitle cues permitted per track (20,000 cues).
pub const MAX_SUBTITLE_CUE_COUNT: usize = 20_000;

/// Current schema version for SubtitleDocument and SubtitleTrack domain objects.
pub const SUBTITLE_DOCUMENT_SCHEMA_VERSION: u32 = 1;

/// Supported subtitle exchange and project formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubtitleFormat {
    /// SubRip Text format (.srt).
    Srt,
    /// Web Video Text Tracks format (.vtt).
    WebVtt,
    /// OpenTake native project format (.omwsub / .json).
    NativeProject,
}

impl SubtitleFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            SubtitleFormat::Srt => "srt",
            SubtitleFormat::WebVtt => "vtt",
            SubtitleFormat::NativeProject => "omwsub",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            SubtitleFormat::Srt => "application/x-subrip",
            SubtitleFormat::WebVtt => "text/vtt",
            SubtitleFormat::NativeProject => "application/json",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        match clean.as_str() {
            "srt" => Some(SubtitleFormat::Srt),
            "vtt" | "webvtt" => Some(SubtitleFormat::WebVtt),
            "omwsub" | "json" => Some(SubtitleFormat::NativeProject),
            _ => None,
        }
    }
}

/// Standards-aware WebVTT cue positioning and styling settings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebVttCueSettings {
    pub vertical: Option<String>,
    pub line: Option<String>,
    pub position: Option<String>,
    pub size: Option<String>,
    pub align: Option<String>,
    pub raw_settings: Option<String>,
}

impl WebVttCueSettings {
    pub fn is_empty(&self) -> bool {
        self.vertical.is_none()
            && self.line.is_none()
            && self.position.is_none()
            && self.size.is_none()
            && self.align.is_none()
            && self.raw_settings.is_none()
    }

    /// Serializes cue settings into standard WebVTT inline format (e.g. `line:90% align:center`).
    pub fn to_setting_string(&self) -> String {
        if let Some(ref raw) = self.raw_settings {
            if !raw.trim().is_empty() {
                return raw.trim().to_string();
            }
        }

        let mut parts = Vec::new();
        if let Some(ref v) = self.vertical {
            parts.push(format!("vertical:{v}"));
        }
        if let Some(ref l) = self.line {
            parts.push(format!("line:{l}"));
        }
        if let Some(ref p) = self.position {
            parts.push(format!("position:{p}"));
        }
        if let Some(ref s) = self.size {
            parts.push(format!("size:{s}"));
        }
        if let Some(ref a) = self.align {
            parts.push(format!("align:{a}"));
        }
        parts.join(" ")
    }
}

/// A canonical single subtitle cue defined by strict microsecond boundaries and Unicode text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleCue {
    /// Stable, globally unique identifier for this cue.
    pub id: String,
    /// Inclusive start timestamp in integer microseconds ($t \ge 0$).
    pub start_us: MediaTimeUs,
    /// Exclusive end timestamp in integer microseconds ($end\_us > start\_us$).
    pub end_us: MediaTimeUs,
    /// Multiline subtitle text (normalized to Unicode LF line breaks).
    pub text: String,
    /// Optional speaker name or dialogue voice tag.
    pub speaker: Option<String>,
    /// Optional standards-compliant WebVTT cue settings.
    pub settings: Option<WebVttCueSettings>,
    /// Optional original file-level cue identifier (e.g. SRT sequence number "1" or WebVTT ID).
    pub raw_identifier: Option<String>,
    /// Forward-compatible metadata store.
    #[serde(default)]
    pub custom_metadata: HashMap<String, String>,
}

impl SubtitleCue {
    /// Constructs and strictly validates a new SubtitleCue.
    ///
    /// # Errors
    /// Returns `SubtitleError::NegativeTime` if `start_us < 0`.
    /// Returns `SubtitleError::InvalidTimeRange` if `end_us <= start_us`.
    pub fn new(
        id: impl Into<String>,
        start_us: MediaTimeUs,
        end_us: MediaTimeUs,
        text: impl Into<String>,
    ) -> Result<Self, SubtitleError> {
        let start = start_us.as_micros();
        let end = end_us.as_micros();

        if start < 0 {
            return Err(SubtitleError::NegativeTime { time_us: start });
        }
        if end <= start {
            return Err(SubtitleError::InvalidTimeRange {
                start_us: start,
                end_us: end,
            });
        }

        Ok(Self {
            id: id.into(),
            start_us,
            end_us,
            text: text.into(),
            speaker: None,
            settings: None,
            raw_identifier: None,
            custom_metadata: HashMap::new(),
        })
    }

    /// Returns cue duration in integer microseconds.
    #[inline]
    pub fn duration_us(&self) -> MediaTimeUs {
        MediaTimeUs(self.end_us.as_micros() - self.start_us.as_micros())
    }

    /// Evaluates if a given timeline position falls within this half-open interval $[start\_us, end\_us)$.
    #[inline]
    pub fn contains_time(&self, time_us: MediaTimeUs) -> bool {
        self.start_us <= time_us && time_us < self.end_us
    }
}

/// A collection of ordered subtitle cues representing one linguistic or role-based track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    /// Stable unique track ID.
    pub id: String,
    /// IETF BCP 47 language tag (e.g. "en", "bn", "es", "fr", "und").
    pub language: String,
    /// Human-readable track label (e.g. "English (Original)", "Bengali (বাংলা)").
    pub display_name: String,
    /// Originating or primary format of this track.
    pub source_format: SubtitleFormat,
    /// Ordered sequence of subtitle cues.
    pub cues: Vec<SubtitleCue>,
    /// Whether the track is enabled for export and processing.
    pub is_enabled: bool,
    /// Whether the track is currently rendered on the video preview display.
    pub is_visible: bool,
    /// Schema version for track serialization.
    pub schema_version: u32,
}

impl SubtitleTrack {
    pub fn new(
        id: impl Into<String>,
        language: impl Into<String>,
        display_name: impl Into<String>,
        source_format: SubtitleFormat,
    ) -> Self {
        Self {
            id: id.into(),
            language: language.into(),
            display_name: display_name.into(),
            source_format,
            cues: Vec::new(),
            is_enabled: true,
            is_visible: true,
            schema_version: SUBTITLE_DOCUMENT_SCHEMA_VERSION,
        }
    }

    /// Returns total duration spanned by cues in this track, or ZERO if empty.
    pub fn duration_us(&self) -> MediaTimeUs {
        self.cues
            .iter()
            .map(|c| c.end_us)
            .max()
            .unwrap_or(MediaTimeUs::ZERO)
    }

    /// Sorts cues chronologically by start timestamp, resolving ties by end timestamp.
    pub fn sort_cues(&mut self) {
        self.cues.sort_by(|a, b| {
            a.start_us
                .cmp(&b.start_us)
                .then_with(|| a.end_us.cmp(&b.end_us))
        });
    }

    /// Finds all cues active at `time_us` using an optimized partition point search.
    ///
    /// Running time: $O(\log N + k)$ where $N$ is total cues and $k$ is active/overlapping cues.
    pub fn find_active_cues(&self, time_us: MediaTimeUs) -> Vec<&SubtitleCue> {
        if self.cues.is_empty() {
            return Vec::new();
        }

        // Binary search: find partition where cue.start_us <= time_us
        let count = self.cues.partition_point(|cue| cue.start_us <= time_us);
        if count == 0 {
            return Vec::new();
        }

        let mut active = Vec::new();
        // Scan backwards from partition boundary
        // Typical maximum overlap in standard subtitles is 1 to 3 cues
        for cue in self.cues[..count].iter().rev() {
            if cue.end_us > time_us {
                active.push(cue);
            } else if time_us.as_micros() - cue.start_us.as_micros() > 60_000_000 {
                // If we've walked backward further than 60 seconds from playback time
                // and cue already ended, stop early to maintain sub-millisecond response
                break;
            }
        }
        active.reverse();
        active
    }
}

/// Root subtitle project document encapsulating multiple language tracks, media binding, and versioning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleDocument {
    /// Document format version (schema version 1).
    pub schema_version: u32,
    /// Stable document UUID.
    pub id: String,
    /// Unique media asset ID to which this subtitle document is securely bound.
    pub associated_media_asset_id: Option<String>,
    /// Language or version tracks contained within this document.
    pub tracks: Vec<SubtitleTrack>,
    /// ID of the currently selected/active subtitle track.
    pub active_track_id: Option<String>,
    /// Creation timestamp (Unix epoch milliseconds).
    pub created_at_epoch_ms: u64,
    /// Last update timestamp (Unix epoch milliseconds).
    pub updated_at_epoch_ms: u64,
    /// Extension metadata dictionary for future-proof enhancements.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SubtitleDocument {
    pub fn new(associated_media_asset_id: Option<String>) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            schema_version: SUBTITLE_DOCUMENT_SCHEMA_VERSION,
            id: Uuid::new_v4().to_string(),
            associated_media_asset_id,
            tracks: Vec::new(),
            active_track_id: None,
            created_at_epoch_ms: now_ms,
            updated_at_epoch_ms: now_ms,
            metadata: HashMap::new(),
        }
    }

    /// Adds a track to the document. If active_track_id is None, sets it as active.
    pub fn add_track(&mut self, track: SubtitleTrack) {
        if self.active_track_id.is_none() {
            self.active_track_id = Some(track.id.clone());
        }
        self.tracks.push(track);
        self.touch();
    }

    /// Returns a reference to the currently active track if found.
    pub fn active_track(&self) -> Option<&SubtitleTrack> {
        let active_id = self.active_track_id.as_ref()?;
        self.tracks.iter().find(|t| &t.id == active_id)
    }

    /// Returns a mutable reference to the currently active track if found.
    pub fn active_track_mut(&mut self) -> Option<&mut SubtitleTrack> {
        let active_id = self.active_track_id.as_ref()?.clone();
        self.tracks.iter_mut().find(|t| t.id == active_id)
    }

    /// Selects an active track by ID. Returns false if track is not found.
    pub fn set_active_track(&mut self, track_id: &str) -> bool {
        if self.tracks.iter().any(|t| t.id == track_id) {
            self.active_track_id = Some(track_id.to_string());
            self.touch();
            true
        } else {
            false
        }
    }

    /// Updates the `updated_at_epoch_ms` timestamp to now.
    pub fn touch(&mut self) {
        self.updated_at_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
    }
}
