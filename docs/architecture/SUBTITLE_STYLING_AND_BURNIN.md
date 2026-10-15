# Subtitle Styling & Burn-In Video Export Architecture (STEP 21)

> **Status:** Implemented (STEP 21)  
> **Workspace:** Localization Studio  
> **Module Path:** `crates/opentake-core/src/localization/styling/`  
> **Frontend:** `src/workspaces/localization/SubtitleStylingPanel.tsx`, `src/workspaces/localization/SubtitleBurnInPanel.tsx`, `src/components/editor/SubtitleOverlay.tsx`  

---

## 1. Executive Summary

STEP 21 completes the visual subtitle pipeline in Open Media Workbench by introducing:
1. **Canonical Subtitle Style Domain:** A strongly typed Rust model representing typography, colors, borders, shadows, background boxes, positions, and animations.
2. **8 Professional Built-in Presets:** Clean Minimal, Cinematic, Modern Outline, Social Bold, Karaoke Highlight, Documentary, Accessible High Contrast, and Soft Background Box, alongside user-defined custom presets.
3. **Local System Font Discovery:** Non-privileged font detection via `fc-list` with specific detection for complex Unicode scripts including Bengali (`Noto Sans Bengali`, `Noto Serif Bengali`).
4. **Injection-Resistant ASS Generation:** Advanced SubStation Alpha script serialization with centisecond rounding, brace escaping (`｛` and `｝`), dialogue formatting, and timing-accurate karaoke tags (`\kf`).
5. **Live Interactive React Preview:** Real-time subtitle overlay tracking canonical styles, responsive to playhead updates, font changes, and live karaoke highlighting without player teardown or `dangerouslySetInnerHTML`.
6. **Production FFmpeg Burn-In Video Export:** Genuine video re-encoding utilizing system FFmpeg and `libass` filtergraph integration (`ass=`), with progress reporting, cooperative cancellation, output probing, and safe staging guarantees.
7. **Strict Source Immutability:** Pre- and post-render SHA-256 hashing verifies original source files remain 100% byte-for-byte unchanged.

---

## 2. Canonical Style Domain Model

Located in `crates/opentake-core/src/localization/styling/model.rs`:

```rust
pub struct SubtitleStyle {
    pub id: String,
    pub display_name: String,
    pub font_family: String,
    pub font_size: u32,       // Clamped 10..120 pt
    pub font_weight: u32,     // 100..900
    pub italic: bool,
    pub text_color: ColorRgba,
    pub outline_color: ColorRgba,
    pub outline_width: f64,   // 0.0..10.0 px
    pub shadow_color: ColorRgba,
    pub shadow_offset: f64,   // 0.0..20.0 px
    pub background_color: ColorRgba,
    pub background_opacity: f64, // 0.0..1.0
    pub position: SubtitlePosition,
    pub safe_margin_v: u32,   // Safe area vertical margin (px)
    pub safe_margin_h: u32,   // Safe area horizontal margin (px)
    pub line_spacing: i32,    // -20..50 px
    pub word_highlight_color: ColorRgba,
    pub animation: SubtitleAnimation,
    pub schema_version: u32,
}
```

### 2.1 Color Storage and ASS Serialization
Colors are stored canonical as `ColorRgba { r: u8, g: u8, b: u8, a: u8 }`.
- In CSS/React preview: formatted as `rgba(r, g, b, alpha_float)`.
- In ASS format: formatted as `&HAABBGGRR` (Little-Endian hexadecimal with inverted alpha channel where `00` = opaque, `FF` = fully transparent).
$$\text{ass\_alpha} = 255 - a$$

---

## 3. Preset Library & Persistence

### 3.1 Built-in Presets
The system provides 8 immutable professional presets:
1. **Clean Minimal:** Crisp white sans-serif text with soft subtle drop shadow, ideal for vlogs and tutorials.
2. **Cinematic:** Elegant Georgia serif with gentle letter spacing and filmic shadow for cinema aesthetics.
3. **Modern Outline:** High-visibility Inter bold text with solid dark outline, preventing loss over bright backgrounds.
4. **Social Bold:** Punchy yellow-on-dark bold typography tailored for short-form mobile and social clips.
5. **Karaoke Highlight:** Golden active-word highlighting with dynamic `\kf` timing and animated subtitle delivery.
6. **Documentary:** Readable neutral subtitles over a translucent dark background bar for journalistic clarity.
7. **Accessible High Contrast:** Pure black box background with stark white text for WCAG AAA legibility.
8. **Soft Background Box:** Rounded translucent container box for versatile web video presentation.

### 3.2 Custom Presets
- Saved to atomic JSON files in `~/.local/share/open-media-workbench/presets/`.
- Built-in presets are protected by validation rules rejecting modification or deletion (`StylingError::BuiltinPresetImmutable`).

---

## 4. Local Font Management & Bengali Support

- **System Discovery:** Probes local fonts using non-privileged `fc-list : family`.
- **Font Fallbacks:** Defaults to verified standard fonts (`Inter`, `Noto Sans`, `Roboto`, `DejaVu Sans`, `sans-serif`).
- **Bengali & Complex Scripts:** Detects local Bengali-capable fonts (`Noto Sans Bengali`, `Noto Serif Bengali`, `Kalpurush`, `SolaimanLipi`) and flags support in the UI.
- **Safety Policy:** No commercial or proprietary fonts are downloaded over the network; untrusted filesystem font paths are never loaded.

---

## 5. ASS Subtitle Generation & Security

Generated via `crates/opentake-core/src/localization/styling/ass.rs`:
- **Header & Script Info:** Declares `PlayResX: 1920` and `PlayResY: 1080` for consistent coordinate scaling.
- **Centisecond Timestamp Rounding:**
  $$\text{cs} = \lfloor (\text{micros} + 5000) / 10000 \rfloor$$
  $$\text{format} = \text{H:MM:SS.cc}$$
- **Anti-Injection Escaping:**
  - Backslashes `\` are converted to full-width `＼` to prevent injecting ASS control tags.
  - Braces `{` and `}` are converted to full-width `｛` and `｝` to prevent injecting custom override codes.
  - Line breaks are normalized to ASS `\N`.
- **Animation & Karaoke Tags:**
  - `Fade`: Emits `{\fad(200,200)}`.
  - `Pop`: Emits `{\fscx85\fscy85\t(0,120,\fscx100\fscy100)}`.
  - `KaraokeHighlight`: Emits `{\kf<dur_cs>}word ` for cues with genuine word timings.

---

## 6. Real FFmpeg Burn-In Video Pipeline

Located in `crates/opentake-core/src/localization/styling/export.rs`:

1. **Pre-flight Check:**
   - Verifies system FFmpeg availability.
   - Confirms `libass` support and `ass` / `subtitles` filter availability via `ffmpeg -filters`.
2. **Path Escaping for Filtergraph:**
   - Single quotes, colons, and backslashes in the staged ASS path are escaped safely (`ass='/path/to/sub.ass'`).
3. **Structured Arguments (Non-Shell Execution):**
   ```bash
   ffmpeg -y -i <source> -vf "ass='...'" -c:v libx264 -preset medium -crf 20 -c:a copy <staged_output.mp4>
   ```
4. **Output Verification with FFprobe:**
   - Runs `ffprobe -v error -show_entries ... -of json` to confirm video stream existence, valid dimensions, and container sanity.
5. **Safe Staging & Immutability:**
   - Video renders into `~/.local/share/open-media-workbench/burnin-staging/`.
   - Native file dialog determines final export location.
   - Finalizer strictly rejects overwriting the original source video.
   - Pre- and post-render SHA-256 hashes confirm zero-byte source modification.

---

## 7. Preview / Export Parity Matrix

| Property | React Overlay Mapping | FFmpeg / libass Mapping | Parity Status |
| :--- | :--- | :--- | :--- |
| **Font Family** | CSS `font-family` | ASS `Style.Fontname` | High (requires font installed on system) |
| **Font Size** | CSS `font-size` (scaled to video height) | ASS `Style.Fontsize` (scaled to PlayResY) | High parity |
| **Font Weight** | CSS `font-weight: 700` | ASS `Style.Bold: 1` | High |
| **Italic** | CSS `font-style: italic` | ASS `Style.Italic: 1` | Exact match |
| **Text Color** | CSS `color: rgba(...)` | ASS `&HAABBGGRR` Primary | Exact match |
| **Outline** | CSS multi-direction `text-shadow` | ASS `Style.Outline` + Secondary/Outline color | Very High (ASS uses true vector strokes) |
| **Shadow** | CSS `text-shadow: offset offset blur color` | ASS `Style.Shadow` + Shadow color | High |
| **Background Box** | CSS `background-color: rgba(...)` | ASS `Style.BorderStyle: 3` (box mode) | High |
| **Position** | CSS flexbox / percentage bounds | ASS `Style.Alignment` (numpad 1..9) + margins | High |
| **Karaoke** | Dynamic active word highlight span | ASS `\kf` fill tags | High parity on words with valid timings |
| **Animations** | CSS transitions (fade / pop) | ASS `\fad` and `\t(\fscx\fscy)` | High parity |
