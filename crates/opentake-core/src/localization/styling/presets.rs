//! Built-in Subtitle Style Presets and Custom Preset Storage Manager.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use super::errors::StylingError;
use super::model::{
    ColorRgba, SubtitleAnimation, SubtitlePosition, SubtitleStyle, SubtitleStylePreset,
    TextAlignment,
};

/// Generates the standard library of 8 built-in professional subtitle presets.
pub fn get_builtin_presets() -> Vec<SubtitleStylePreset> {
    vec![
        SubtitleStylePreset {
            id: "clean-minimal".to_string(),
            display_name: "Clean Minimal".to_string(),
            description:
                "Crisp white text with subtle dark outline. Universal, timeless readability."
                    .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "clean-minimal".to_string(),
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
                background_color: ColorRgba::transparent(),
                background_opacity: 0.0,
                background_padding: 6.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::BottomCenter,
                margin_vertical: 36,
                margin_horizontal: 40,
                line_spacing: 1.25,
                word_highlight_color: ColorRgba::opaque(245, 158, 11),
                animation: SubtitleAnimation::None,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
        SubtitleStylePreset {
            id: "cinematic".to_string(),
            display_name: "Cinematic".to_string(),
            description:
                "Warm off-white typography with delicate dark border for theatrical films."
                    .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "cinematic".to_string(),
                display_name: "Cinematic".to_string(),
                font_family: "DejaVu Serif".to_string(),
                font_size: 30,
                font_weight: 500,
                italic: false,
                text_color: ColorRgba::opaque(254, 252, 232), // Warm ivory
                outline_color: ColorRgba::opaque(15, 15, 15),
                outline_width: 1.8,
                shadow_color: ColorRgba::new(0, 0, 0, 220),
                shadow_offset: 2.0,
                background_color: ColorRgba::transparent(),
                background_opacity: 0.0,
                background_padding: 6.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::BottomCenter,
                margin_vertical: 48,
                margin_horizontal: 50,
                line_spacing: 1.3,
                word_highlight_color: ColorRgba::opaque(251, 191, 36),
                animation: SubtitleAnimation::Fade,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
        SubtitleStylePreset {
            id: "modern-outline".to_string(),
            display_name: "Modern Outline".to_string(),
            description:
                "Bold sans text with heavy outline. Stands out across fast-moving footage."
                    .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "modern-outline".to_string(),
                display_name: "Modern Outline".to_string(),
                font_family: "DejaVu Sans".to_string(),
                font_size: 34,
                font_weight: 700,
                italic: false,
                text_color: ColorRgba::opaque(255, 255, 255),
                outline_color: ColorRgba::opaque(0, 0, 0),
                outline_width: 3.5,
                shadow_color: ColorRgba::new(0, 0, 0, 200),
                shadow_offset: 2.0,
                background_color: ColorRgba::transparent(),
                background_opacity: 0.0,
                background_padding: 6.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::BottomCenter,
                margin_vertical: 40,
                margin_horizontal: 40,
                line_spacing: 1.2,
                word_highlight_color: ColorRgba::opaque(245, 158, 11),
                animation: SubtitleAnimation::None,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
        SubtitleStylePreset {
            id: "social-bold".to_string(),
            display_name: "Social Bold".to_string(),
            description:
                "High-impact bold captioning with vibrant yellow accent and pop animation."
                    .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "social-bold".to_string(),
                display_name: "Social Bold".to_string(),
                font_family: "DejaVu Sans".to_string(),
                font_size: 38,
                font_weight: 800,
                italic: false,
                text_color: ColorRgba::opaque(254, 240, 138), // Vibrant yellow
                outline_color: ColorRgba::opaque(17, 24, 39),
                outline_width: 4.0,
                shadow_color: ColorRgba::new(0, 0, 0, 240),
                shadow_offset: 3.0,
                background_color: ColorRgba::transparent(),
                background_opacity: 0.0,
                background_padding: 8.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::Center,
                margin_vertical: 60,
                margin_horizontal: 30,
                line_spacing: 1.15,
                word_highlight_color: ColorRgba::opaque(56, 189, 248), // Sky blue
                animation: SubtitleAnimation::Pop,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
        SubtitleStylePreset {
            id: "karaoke-highlight".to_string(),
            display_name: "Karaoke Highlight".to_string(),
            description: "Live word-synchronized highlighting using genuine word timing evidence."
                .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "karaoke-highlight".to_string(),
                display_name: "Karaoke Highlight".to_string(),
                font_family: "DejaVu Sans".to_string(),
                font_size: 36,
                font_weight: 700,
                italic: false,
                text_color: ColorRgba::opaque(228, 228, 231), // Zinc-200
                outline_color: ColorRgba::opaque(9, 9, 11),
                outline_width: 2.5,
                shadow_color: ColorRgba::new(0, 0, 0, 200),
                shadow_offset: 1.5,
                background_color: ColorRgba::new(0, 0, 0, 140),
                background_opacity: 0.55,
                background_padding: 8.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::BottomCenter,
                margin_vertical: 44,
                margin_horizontal: 40,
                line_spacing: 1.25,
                word_highlight_color: ColorRgba::opaque(245, 158, 11), // Amber-500
                animation: SubtitleAnimation::KaraokeHighlight,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
        SubtitleStylePreset {
            id: "documentary".to_string(),
            display_name: "Documentary".to_string(),
            description: "High-contrast semi-transparent pill box with clean white text."
                .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "documentary".to_string(),
                display_name: "Documentary".to_string(),
                font_family: "DejaVu Sans".to_string(),
                font_size: 30,
                font_weight: 500,
                italic: false,
                text_color: ColorRgba::opaque(244, 244, 245),
                outline_color: ColorRgba::transparent(),
                outline_width: 0.0,
                shadow_color: ColorRgba::transparent(),
                shadow_offset: 0.0,
                background_color: ColorRgba::new(10, 10, 10, 215),
                background_opacity: 0.85,
                background_padding: 10.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::BottomCenter,
                margin_vertical: 36,
                margin_horizontal: 40,
                line_spacing: 1.3,
                word_highlight_color: ColorRgba::opaque(251, 191, 36),
                animation: SubtitleAnimation::None,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
        SubtitleStylePreset {
            id: "accessible-high-contrast".to_string(),
            display_name: "Accessible High Contrast".to_string(),
            description: "WCAG AAA compliant pure yellow text on solid dark background."
                .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "accessible-high-contrast".to_string(),
                display_name: "Accessible High Contrast".to_string(),
                font_family: "DejaVu Sans".to_string(),
                font_size: 32,
                font_weight: 700,
                italic: false,
                text_color: ColorRgba::opaque(255, 235, 59), // High-visibility yellow
                outline_color: ColorRgba::opaque(0, 0, 0),
                outline_width: 1.0,
                shadow_color: ColorRgba::transparent(),
                shadow_offset: 0.0,
                background_color: ColorRgba::opaque(0, 0, 0),
                background_opacity: 1.0,
                background_padding: 12.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::BottomCenter,
                margin_vertical: 32,
                margin_horizontal: 40,
                line_spacing: 1.35,
                word_highlight_color: ColorRgba::opaque(255, 255, 255),
                animation: SubtitleAnimation::None,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
        SubtitleStylePreset {
            id: "soft-background-box".to_string(),
            display_name: "Soft Background Box".to_string(),
            description: "Modern translucent dark backing with gentle shadow and soft contrast."
                .to_string(),
            is_builtin: true,
            style: SubtitleStyle {
                id: "soft-background-box".to_string(),
                display_name: "Soft Background Box".to_string(),
                font_family: "DejaVu Sans".to_string(),
                font_size: 31,
                font_weight: 500,
                italic: false,
                text_color: ColorRgba::opaque(250, 250, 250),
                outline_color: ColorRgba::opaque(0, 0, 0),
                outline_width: 1.0,
                shadow_color: ColorRgba::new(0, 0, 0, 160),
                shadow_offset: 1.5,
                background_color: ColorRgba::new(24, 24, 27, 180),
                background_opacity: 0.70,
                background_padding: 8.0,
                text_alignment: TextAlignment::Center,
                position: SubtitlePosition::BottomCenter,
                margin_vertical: 38,
                margin_horizontal: 40,
                line_spacing: 1.25,
                word_highlight_color: ColorRgba::opaque(245, 158, 11),
                animation: SubtitleAnimation::Fade,
                safe_area_enabled: true,
                schema_version: 1,
            },
        },
    ]
}

/// Manager overseeing custom preset loading, saving, and deletion.
pub struct StylePresetManager {
    storage_path: PathBuf,
    cache: RwLock<Vec<SubtitleStylePreset>>,
}

impl StylePresetManager {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        let storage_path = storage_dir.as_ref().join("custom_style_presets.json");
        let manager = Self {
            storage_path,
            cache: RwLock::new(Vec::new()),
        };
        manager.reload_cache();
        manager
    }

    fn reload_cache(&self) {
        let mut all = get_builtin_presets();
        if self.storage_path.exists() {
            if let Ok(data) = fs::read_to_string(&self.storage_path) {
                if let Ok(custom) = serde_json::from_str::<Vec<SubtitleStylePreset>>(&data) {
                    for p in custom {
                        if !p.is_builtin && !all.iter().any(|b| b.id == p.id) {
                            all.push(p);
                        }
                    }
                }
            }
        }
        if let Ok(mut lock) = self.cache.write() {
            *lock = all;
        }
    }

    /// Returns all presets (built-in followed by user custom presets).
    pub fn list_presets(&self) -> Vec<SubtitleStylePreset> {
        if let Ok(lock) = self.cache.read() {
            lock.clone()
        } else {
            get_builtin_presets()
        }
    }

    /// Saves or updates a custom preset. Rejects attempts to overwrite built-in presets.
    pub fn save_custom_preset(&self, mut preset: SubtitleStylePreset) -> Result<(), StylingError> {
        preset.style.validate()?;
        preset.is_builtin = false;

        let builtins = get_builtin_presets();
        if builtins.iter().any(|b| b.id == preset.id) {
            return Err(StylingError::BuiltinPresetImmutable(preset.id));
        }

        let mut custom_presets = Vec::new();
        if self.storage_path.exists() {
            if let Ok(data) = fs::read_to_string(&self.storage_path) {
                if let Ok(existing) = serde_json::from_str::<Vec<SubtitleStylePreset>>(&data) {
                    custom_presets = existing;
                }
            }
        }

        if let Some(pos) = custom_presets.iter().position(|p| p.id == preset.id) {
            custom_presets[pos] = preset;
        } else {
            custom_presets.push(preset);
        }

        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&custom_presets)?;
        // Atomic write via temp file
        let temp_path = self.storage_path.with_extension("tmp");
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &self.storage_path)?;

        self.reload_cache();
        Ok(())
    }

    /// Deletes a custom preset. Strictly rejects deleting built-in presets.
    pub fn delete_custom_preset(&self, preset_id: &str) -> Result<(), StylingError> {
        let builtins = get_builtin_presets();
        if builtins.iter().any(|b| b.id == preset_id) {
            return Err(StylingError::BuiltinPresetImmutable(preset_id.to_string()));
        }

        if !self.storage_path.exists() {
            return Ok(());
        }

        let data = fs::read_to_string(&self.storage_path)?;
        let mut custom_presets = serde_json::from_str::<Vec<SubtitleStylePreset>>(&data)?;
        custom_presets.retain(|p| p.id != preset_id);

        let json = serde_json::to_string_pretty(&custom_presets)?;
        let temp_path = self.storage_path.with_extension("tmp");
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &self.storage_path)?;

        self.reload_cache();
        Ok(())
    }
}
