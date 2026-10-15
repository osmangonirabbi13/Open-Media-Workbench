//! Word-Level Timestamp Alignment, Smart Subtitle Segmentation & Karaoke Foundation.

pub mod errors;
pub mod models;
pub mod parser;
pub mod refinement;
pub mod segmentation;
pub mod validation;

pub use errors::AlignmentError;
pub use models::{
    AlignedWord, AlignmentResult, SegmentationPolicy, TimingQuality, WordTimingProvenance,
};
pub use parser::{parse_whisper_alignment_output, split_word_punctuation};
pub use refinement::{refine_cues_with_words, update_word_timing_in_cue};
pub use segmentation::{format_cue_lines, segment_words_into_cues};
pub use validation::validate_word_sequence;
