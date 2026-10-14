pub mod error;
pub mod model;
pub mod persistence;
pub mod serialization;
pub mod srt;
pub mod validation;
pub mod webvtt;

pub use error::SubtitleError;
pub use model::{
    SubtitleCue, SubtitleDocument, SubtitleFormat, SubtitleTrack, WebVttCueSettings,
    MAX_SUBTITLE_CUE_COUNT, MAX_SUBTITLE_FILE_SIZE, SUBTITLE_DOCUMENT_SCHEMA_VERSION,
};
pub use persistence::{load_subtitle_document, save_subtitle_document};
pub use serialization::{export_subtitles_to_string, parse_subtitles_from_str};
pub use srt::{parse_srt, serialize_srt};
pub use validation::{validate_document, validate_track, FindingSeverity, SubtitleFinding};
pub use webvtt::{parse_webvtt, serialize_webvtt};
