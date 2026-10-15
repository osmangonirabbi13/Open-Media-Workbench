//! Offline Text-to-Speech (TTS) Domain for Open Media Workbench.

pub mod audio;
pub mod engine;
pub mod errors;
pub mod manager;
pub mod models;
pub mod segments;

pub use audio::{concatenate_wav_files, validate_wav_file};
pub use engine::{MockTtsEngine, SystemPiperCliEngine, TtsEngine};
pub use errors::TtsError;
pub use manager::TtsManager;
pub use models::*;
