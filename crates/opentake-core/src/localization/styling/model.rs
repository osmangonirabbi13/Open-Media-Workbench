//! Canonical Subtitle Style, Presets, and Export Configuration Models.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::errors::StylingError;

/// An RGBA color representation with checked conversions for CSS and ASS/SSA formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorRgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    /// Parses hex string in format `#RGB`, `#RRGGBB`, or `#RRGGBBAA`.
    pub fn parse_hex(hex: &str) -> Result<Self, StylingError> {
        let clean = hex.trim().trim_start_matches('#');
        match clean.len() {
            3 => {
                let r = u8::from_str_radix(&clean[0..1].repeat(2), 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                let g = u8::from_str_radix(&clean[1..2].repeat(2), 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                let b = u8::from_str_radix(&clean[2..3].repeat(2), 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                Ok(Self::opaque(r, g, b))
            }
            6 => {
                let r = u8::from_str_radix(&clean[0..2], 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                let g = u8::from_str_radix(&clean[2..4], 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                let b = u8::from_str_radix(&clean[4..6], 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                Ok(Self::opaque(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&clean[0..2], 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                let g = u8::from_str_radix(&clean[2..4], 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                let b = u8::from_str_radix(&clean[4..6], 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                let a = u8::from_str_radix(&clean[6..8], 16)
                    .map_err(|e| StylingError::InvalidColor(format!("{e}")))?;
                Ok(Self::new(r, g, b, a))
            }
            _ => Err(StylingError::InvalidColor(format!(
                "Invalid hex length: '{hex}'"
            ))),
        }
    }

    /// Formats as standard CSS `#RRGGBB` or `#RRGGBBAA`.
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }

    /// Formats as CSS `rgba(r, g, b, a)`.
    pub fn to_css_rgba(&self) -> String {
        let alpha = self.a as f64 / 255.0;
        format!("rgba({}, {}, {}, {:.3})", self.r, self.g, self.b, alpha)
    }

    /// Converts to ASS color string in `&HAABBGGRR` format.
    /// Note: ASS alpha is inverted (00 = fully opaque, FF = fully transparent).
    pub fn to_ass_color(&self) -> String {
        let ass_alpha = 255 - self.a;
        format!(
            "&H{:02X}{:02X}{:02X}{:02X}",
            ass_alpha, self.b, self.g, self.r
        )
    }
}

impl Default for ColorRgba {
    fn default() -> Self {
        Self::opaque(255, 255, 255)
    }
}

impl fmt::Display for ColorRgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Subtitle frame position preset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubtitlePosition {
    #[default]
    BottomCenter,
    BottomLeft,
    BottomRight,
    Center,
    TopCenter,
    Custom { x_pct: f64, y_pct: f64 },
}

impl SubtitlePosition {
    /// Returns ASS numpad alignment code (1-9).
    /// 1=BottomLeft, 2=BottomCenter, 3=BottomRight,
    /// 4=MiddleLeft, 5=MiddleCenter, 6=MiddleRight,
    /// 7=TopLeft, 8=TopCenter, 9=TopRight.
    pub fn to_ass_alignment(&self) -> u32 {
        match self {
            Self::BottomCenter => 2,
            Self::BottomLeft => 1,
            Self::BottomRight => 3,
            Self::Center => 5,
            Self::TopCenter => 8,
            Self::Custom { x_pct, y_pct } => {
                if *y_pct > 66.0 {
                    if *x_pct < 33.0 {
                        1
                    } else if *x_pct > 66.0 {
                        3
                    } else {
                        2
                    }
                } else if *y_pct < 33.0 {
                    if *x_pct < 33.0 {
                        7
                    } else if *x_pct > 66.0 {
                        9
                    } else {
                        8
                    }
                } else {
                    if *x_pct < 33.0 {
                        4
                    } else if *x_pct > 66.0 {
                        6
                    } else {
                        5
                    }
                }
            }
        }
    }
}

/// Supported subtitle animation effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SubtitleAnimation {
    #[default]
    None,
    Fade,
    Pop,
    KaraokeHighlight,
}

/// Canonical subtitle styling model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStyle {
    pub id: String,
    pub display_name: String,
    pub font_family: String,
    pub font_size: u32,
    pub font_weight: u32,
    pub italic: bool,
    pub text_color: ColorRgba,
    pub outline_color: ColorRgba,
    pub outline_width: f64,
    pub shadow_color: ColorRgba,
    pub shadow_offset: f64,
    pub background_color: ColorRgba,
    pub background_opacity: f64,
    pub background_padding: f64,
    pub text_alignment: TextAlignment,
    pub position: SubtitlePosition,
    pub margin_vertical: u32,
    pub margin_horizontal: u32,
    pub line_spacing: f64,
    pub word_highlight_color: ColorRgba,
    pub animation: SubtitleAnimation,
    pub safe_area_enabled: bool,
    pub schema_version: u32,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            id: "default-clean".to_string(),
            display_name: "Clean Minimal".to_string(),
            font_family: "DejaVu Sans".to_string(),
            font_size: 32,
            font_weight: 600,
            italic: false,
            text_color: ColorRgba::opaque(255, 255, 255),
            outline_color: ColorRgba::opaque(0, 0, 0),
            outline_width: 2.0,
            shadow_color: ColorRgba::new(0, 0, 0, 180),
            shadow_offset: 1.5,
            background_color: ColorRgba::opaque(0, 0, 0),
            background_opacity: 0.0,
            background_padding: 6.0,
            text_alignment: TextAlignment::Center,
            position: SubtitlePosition::BottomCenter,
            margin_vertical: 36,
            margin_horizontal: 40,
            line_spacing: 1.25,
            word_highlight_color: ColorRgba::opaque(245, 158, 11), // Amber-500
            animation: SubtitleAnimation::None,
            safe_area_enabled: true,
            schema_version: 1,
        }
    }
}

impl SubtitleStyle {
    /// Validates all style attributes against safe physical bounds.
    pub fn validate(&self) -> Result<(), StylingError> {
        if self.font_size < 10 || self.font_size > 140 {
            return Err(StylingError::InvalidProperty(format!(
                "Font size must be between 10 and 140 pt, got {}",
                self.font_size
            )));
        }
        if self.outline_width < 0.0 || self.outline_width > 20.0 {
            return Err(StylingError::InvalidProperty(format!(
                "Outline width must be between 0.0 and 20.0 px, got {}",
                self.outline_width
            )));
        }
        if self.shadow_offset < 0.0 || self.shadow_offset > 20.0 {
            return Err(StylingError::InvalidProperty(format!(
                "Shadow offset must be between 0.0 and 20.0 px, got {}",
                self.shadow_offset
            )));
        }
        if self.background_opacity < 0.0 || self.background_opacity > 1.0 {
            return Err(StylingError::InvalidProperty(format!(
                "Background opacity must be between 0.0 and 1.0, got {}",
                self.background_opacity
            )));
        }
        if self.font_family.trim().is_empty() {
            return Err(StylingError::InvalidProperty(
                "Font family cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// A named, reusable subtitle style preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStylePreset {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub is_builtin: bool,
    pub style: SubtitleStyle,
}

/// Video export resolution option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ExportResolution {
    #[default]
    Original,
    Res1080p,
    Res720p,
    Custom { width: u32, height: u32 },
}

/// Audio track handling policy during burn-in export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AudioExportPolicy {
    #[default]
    Copy,
    Aac,
}

/// Subtitle Burn-In Video Export Profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleExportProfile {
    pub container: String,
    pub resolution: ExportResolution,
    pub video_codec: String,
    pub crf: u32,
    pub preset: String,
    pub audio_policy: AudioExportPolicy,
    pub selected_track_id: String,
    pub style: SubtitleStyle,
}

impl Default for SubtitleExportProfile {
    fn default() -> Self {
        Self {
            container: "mp4".to_string(),
            resolution: ExportResolution::Original,
            video_codec: "libx264".to_string(),
            crf: 23,
            preset: "medium".to_string(),
            audio_policy: AudioExportPolicy::Copy,
            selected_track_id: String::new(),
            style: SubtitleStyle::default(),
        }
    }
}
