//! Subtitle Styling, ASS Generation, and Burn-In Export module.

pub mod ass;
pub mod errors;
pub mod export;
pub mod fonts;
pub mod model;
pub mod presets;

pub use ass::{escape_ass_text, format_ass_timestamp, generate_ass_script};
pub use errors::StylingError;
pub use export::{
    build_ass_filter_arg, execute_subtitle_burn_in, probe_video_output,
    verify_ffmpeg_burnin_support, BurnInExportManager, SubtitleBurnInRequest, SubtitleBurnInResult,
    SubtitleBurnInStatus,
};
pub use fonts::{discover_system_fonts, resolve_safe_font_family, FontFamilyInfo};
pub use model::{
    AudioExportPolicy, ColorRgba, ExportResolution, SubtitleAnimation, SubtitleExportProfile,
    SubtitlePosition, SubtitleStyle, SubtitleStylePreset, TextAlignment,
};
pub use presets::{get_builtin_presets, StylePresetManager};
