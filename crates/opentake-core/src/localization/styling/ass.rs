use crate::editing::time::MediaTimeUs;
use crate::localization::subtitles::model::SubtitleTrack;

use super::errors::StylingError;
use super::fonts::resolve_safe_font_family;
use super::model::{SubtitleAnimation, SubtitleStyle};

/// Converts integer microseconds into ASS timestamp format `H:MM:SS.cc` (centisecond precision).
pub fn format_ass_timestamp(time_us: MediaTimeUs) -> String {
    let total_micros = time_us.as_micros().max(0) as u64;
    // Nearest-centisecond rounding (1 centisecond = 10,000 microseconds)
    let total_cs = (total_micros + 5_000) / 10_000;

    let cs = total_cs % 100;
    let total_secs = total_cs / 100;
    let s = total_secs % 60;
    let total_mins = total_secs / 60;
    let m = total_mins % 60;
    let h = total_mins / 60;

    format!("{h}:{m:02}:{s:02}.{cs:02}")
}

/// Safely escapes subtitle text for ASS dialogue lines:
/// - Replaces `{` and `}` with full-width or escaped equivalents to prevent arbitrary override injection
/// - Converts newlines into ASS `\N`
/// - Normalizes Unicode and Bengali characters cleanly
pub fn escape_ass_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

    for c in normalized.chars() {
        match c {
            '{' => out.push('｛'), // Unicode full-width brace (prevents tag injection)
            '}' => out.push('｝'),
            '\\' => out.push_str("\\\\"), // Escape raw backslashes
            '\n' => out.push_str("\\N"),
            other => out.push(other),
        }
    }
    out
}

/// Generates a complete, compliant ASS script string from a SubtitleTrack and SubtitleStyle.
pub fn generate_ass_script(
    track: &SubtitleTrack,
    style: &SubtitleStyle,
) -> Result<String, StylingError> {
    style.validate()?;

    let mut ass = String::with_capacity(8192);

    // 1. Script Info Header
    ass.push_str("[Script Info]\n");
    ass.push_str("Title: Open Media Workbench Generated Subtitles\n");
    ass.push_str("ScriptType: v4.00+\n");
    ass.push_str("WrapStyle: 0\n");
    ass.push_str("ScaledBorderAndShadow: yes\n");
    ass.push_str("YCbCr Matrix: None\n");
    ass.push_str("PlayResX: 1920\n");
    ass.push_str("PlayResY: 1080\n\n");

    // 2. Styles Definition
    ass.push_str("[V4+ Styles]\n");
    ass.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n");

    let is_bengali = track.language.to_lowercase().starts_with("bn")
        || track.cues.iter().any(|c| {
            c.text
                .chars()
                .any(|ch| ('\u{0980}'..='\u{09FF}').contains(&ch))
        });
    let font_name = resolve_safe_font_family(&style.font_family, is_bengali);

    let bold_val = if style.font_weight >= 700 { -1 } else { 0 };
    let italic_val = if style.italic { -1 } else { 0 };

    let primary_col = style.text_color.to_ass_color();
    let secondary_col = style.word_highlight_color.to_ass_color();
    let outline_col = style.outline_color.to_ass_color();

    // If background opacity >= 0.5, use BorderStyle = 3 (opaque background box).
    // Otherwise use BorderStyle = 1 (outline + drop shadow).
    let (border_style, back_col) = if style.background_opacity >= 0.5 {
        let alpha = (255.0 * (1.0 - style.background_opacity)).clamp(0.0, 255.0) as u8;
        let box_col = format!(
            "&H{:02X}{:02X}{:02X}{:02X}",
            alpha, style.background_color.b, style.background_color.g, style.background_color.r
        );
        (3, box_col)
    } else {
        (1, style.shadow_color.to_ass_color())
    };

    let alignment = style.position.to_ass_alignment();

    // Base Style
    ass.push_str(&format!(
        "Style: Default,{},{},{},{},{},{},{},{},0,0,100,100,0,0,{},{:.1},{:.1},{},{},{},{},1\n\n",
        font_name,
        style.font_size,
        primary_col,
        secondary_col,
        outline_col,
        back_col,
        bold_val,
        italic_val,
        border_style,
        style.outline_width,
        style.shadow_offset,
        alignment,
        style.margin_horizontal,
        style.margin_horizontal,
        style.margin_vertical
    ));

    // 3. Events Section
    ass.push_str("[Events]\n");
    ass.push_str(
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
    );

    for cue in &track.cues {
        if cue.text.trim().is_empty() {
            continue;
        }

        let start_str = format_ass_timestamp(cue.start_us);
        let end_str = format_ass_timestamp(cue.end_us);

        let speaker_prefix = if let Some(ref spk) = cue.speaker {
            if !spk.trim().is_empty() {
                format!("[{spk}]: ")
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let mut dialogue_text = String::new();

        // Animation tag injection
        match style.animation {
            SubtitleAnimation::Fade => {
                dialogue_text.push_str("{\\fad(200,200)}");
            }
            SubtitleAnimation::Pop => {
                dialogue_text.push_str("{\\fscx85\\fscy85\\t(0,120,\\fscx100\\fscy100)}");
            }
            SubtitleAnimation::KaraokeHighlight => {
                // If genuine word timing is present, render karaoke tags
                if let Some(ref words) = cue.words {
                    if !words.is_empty() {
                        let mut karaoke_part = String::new();
                        for w in words {
                            let w_start = w.start_us.as_micros().max(0);
                            let w_end = w.end_us.as_micros().max(w_start);
                            let dur_cs = ((w_end - w_start) as u64 + 5_000) / 10_000;
                            let clean_word = escape_ass_text(&w.text);
                            let prefix = w.punctuation_prefix.as_deref().unwrap_or("");
                            let suffix = w.punctuation_suffix.as_deref().unwrap_or("");
                            karaoke_part.push_str(&format!(
                                "{{\\kf{dur_cs}}}{prefix}{clean_word}{suffix} "
                            ));
                        }
                        dialogue_text.push_str(karaoke_part.trim_end());
                    }
                }
            }
            SubtitleAnimation::None => {}
        }

        // If karaoke wasn't applied (e.g. no words or animation != KaraokeHighlight), render escaped text
        if dialogue_text.is_empty() || style.animation != SubtitleAnimation::KaraokeHighlight {
            let escaped = escape_ass_text(&cue.text);
            dialogue_text.push_str(&speaker_prefix);
            dialogue_text.push_str(&escaped);
        }

        ass.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            start_str, end_str, dialogue_text
        ));
    }

    Ok(ass)
}
