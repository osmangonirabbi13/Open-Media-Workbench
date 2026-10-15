/**
 * Subtitle Styling and Video Burn-In Export client bridge.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  ColorRgba,
  FontFamilyInfo,
  SubtitleBurnInRequest,
  SubtitleBurnInStatus,
  SubtitleStyle,
  SubtitleStylePreset,
} from "../types/styling";
import type { SubtitleTrack } from "../types/subtitles";

export const DEFAULT_SUBTITLE_STYLE: SubtitleStyle = {
  id: "clean-minimal",
  displayName: "Clean Minimal",
  fontFamily: "DejaVu Sans",
  fontSize: 32,
  fontWeight: 600,
  italic: false,
  textColor: { r: 255, g: 255, b: 255, a: 255 },
  outlineColor: { r: 0, g: 0, b: 0, a: 255 },
  outlineWidth: 2.0,
  shadowColor: { r: 0, g: 0, b: 0, a: 180 },
  shadowOffset: 1.5,
  backgroundColor: { r: 0, g: 0, b: 0, a: 0 },
  backgroundOpacity: 0.0,
  backgroundPadding: 6.0,
  textAlignment: "center",
  position: { type: "bottomCenter" },
  marginVertical: 36,
  marginHorizontal: 40,
  lineSpacing: 1.25,
  wordHighlightColor: { r: 245, g: 158, b: 11, a: 255 },
  animation: "none",
  safeAreaEnabled: true,
  schemaVersion: 1,
};

export const MOCK_PRESETS: SubtitleStylePreset[] = [
  {
    id: "clean-minimal",
    displayName: "Clean Minimal",
    description: "Crisp white text with subtle dark outline. Universal, timeless readability.",
    isBuiltin: true,
    style: { ...DEFAULT_SUBTITLE_STYLE },
  },
  {
    id: "cinematic",
    displayName: "Cinematic",
    description: "Warm off-white typography with delicate dark border for theatrical films.",
    isBuiltin: true,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "cinematic",
      displayName: "Cinematic",
      fontFamily: "DejaVu Serif",
      fontSize: 30,
      fontWeight: 500,
      textColor: { r: 254, g: 252, b: 232, a: 255 },
      outlineColor: { r: 15, g: 15, b: 15, a: 255 },
      outlineWidth: 1.8,
      shadowOffset: 2.0,
      marginVertical: 48,
      animation: "fade",
    },
  },
  {
    id: "modern-outline",
    displayName: "Modern Outline",
    description: "Bold sans text with heavy outline. Stands out across fast-moving footage.",
    isBuiltin: true,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "modern-outline",
      displayName: "Modern Outline",
      fontSize: 34,
      fontWeight: 700,
      outlineWidth: 3.5,
      animation: "none",
    },
  },
  {
    id: "social-bold",
    displayName: "Social Bold",
    description: "High-impact bold captioning with vibrant yellow accent and pop animation.",
    isBuiltin: true,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "social-bold",
      displayName: "Social Bold",
      fontSize: 38,
      fontWeight: 800,
      textColor: { r: 254, g: 240, b: 138, a: 255 },
      outlineColor: { r: 17, g: 24, b: 39, a: 255 },
      outlineWidth: 4.0,
      shadowOffset: 3.0,
      position: { type: "center" },
      wordHighlightColor: { r: 56, g: 189, b: 248, a: 255 },
      animation: "pop",
    },
  },
  {
    id: "karaoke-highlight",
    displayName: "Karaoke Highlight",
    description: "Live word-synchronized highlighting using genuine word timing evidence.",
    isBuiltin: true,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "karaoke-highlight",
      displayName: "Karaoke Highlight",
      fontSize: 36,
      fontWeight: 700,
      textColor: { r: 228, g: 228, b: 231, a: 255 },
      outlineWidth: 2.5,
      backgroundColor: { r: 0, g: 0, b: 0, a: 140 },
      backgroundOpacity: 0.55,
      backgroundPadding: 8.0,
      wordHighlightColor: { r: 245, g: 158, b: 11, a: 255 },
      animation: "karaokeHighlight",
    },
  },
  {
    id: "documentary",
    displayName: "Documentary",
    description: "High-contrast semi-transparent pill box with clean white text.",
    isBuiltin: true,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "documentary",
      displayName: "Documentary",
      fontSize: 30,
      fontWeight: 500,
      outlineWidth: 0,
      shadowOffset: 0,
      backgroundColor: { r: 10, g: 10, b: 10, a: 215 },
      backgroundOpacity: 0.85,
      backgroundPadding: 10.0,
    },
  },
  {
    id: "accessible-high-contrast",
    displayName: "Accessible High Contrast",
    description: "WCAG AAA compliant pure yellow text on solid dark background.",
    isBuiltin: true,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "accessible-high-contrast",
      displayName: "Accessible High Contrast",
      fontSize: 32,
      fontWeight: 700,
      textColor: { r: 255, g: 235, b: 59, a: 255 },
      outlineWidth: 1.0,
      shadowOffset: 0,
      backgroundColor: { r: 0, g: 0, b: 0, a: 255 },
      backgroundOpacity: 1.0,
      backgroundPadding: 12.0,
    },
  },
  {
    id: "soft-background-box",
    displayName: "Soft Background Box",
    description: "Modern translucent dark backing with gentle shadow and soft contrast.",
    isBuiltin: true,
    style: {
      ...DEFAULT_SUBTITLE_STYLE,
      id: "soft-background-box",
      displayName: "Soft Background Box",
      fontSize: 31,
      fontWeight: 500,
      textColor: { r: 250, g: 250, b: 250, a: 255 },
      outlineWidth: 1.0,
      backgroundColor: { r: 24, g: 24, b: 27, a: 180 },
      backgroundOpacity: 0.7,
      backgroundPadding: 8.0,
      animation: "fade",
    },
  },
];

export const MOCK_FONTS: FontFamilyInfo[] = [
  { family: "DejaVu Sans", isInstalled: true, supportsBengali: false, isGenericFallback: false },
  { family: "DejaVu Serif", isInstalled: true, supportsBengali: false, isGenericFallback: false },
  { family: "Noto Sans", isInstalled: true, supportsBengali: false, isGenericFallback: false },
  { family: "Noto Sans Bengali", isInstalled: true, supportsBengali: true, isGenericFallback: false },
  { family: "Noto Serif Bengali", isInstalled: true, supportsBengali: true, isGenericFallback: false },
  { family: "Liberation Sans", isInstalled: true, supportsBengali: false, isGenericFallback: false },
  { family: "Arial", isInstalled: false, supportsBengali: false, isGenericFallback: true },
  { family: "Inter", isInstalled: false, supportsBengali: false, isGenericFallback: true },
];

export function colorRgbaToCss(c: ColorRgba): string {
  const a = (c.a / 255).toFixed(3);
  return `rgba(${c.r}, ${c.g}, ${c.b}, ${a})`;
}

export function colorRgbaToHex(c: ColorRgba): string {
  const toHex = (n: number) => n.toString(16).padStart(2, "0").toUpperCase();
  return `#${toHex(c.r)}${toHex(c.g)}${toHex(c.b)}`;
}

export function parseHexToColorRgba(hex: string, alpha = 255): ColorRgba {
  const clean = hex.trim().replace(/^#/, "");
  if (clean.length === 6) {
    return {
      r: parseInt(clean.slice(0, 2), 16) || 0,
      g: parseInt(clean.slice(2, 4), 16) || 0,
      b: parseInt(clean.slice(4, 6), 16) || 0,
      a: alpha,
    };
  }
  return { r: 255, g: 255, b: 255, a: alpha };
}

function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function getSubtitleStylePresets(): Promise<SubtitleStylePreset[]> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<SubtitleStylePreset[]>("get_subtitle_style_presets");
    } catch {
      return MOCK_PRESETS;
    }
  }
  return MOCK_PRESETS;
}

export async function saveCustomSubtitleStylePreset(preset: SubtitleStylePreset): Promise<void> {
  if (isTauriEnvironment()) {
    await invoke("save_custom_subtitle_style_preset", { preset });
    return;
  }
  // Mock/test environment
  const idx = MOCK_PRESETS.findIndex((p) => p.id === preset.id);
  if (idx >= 0) {
    MOCK_PRESETS[idx] = preset;
  } else {
    MOCK_PRESETS.push(preset);
  }
}

export async function deleteCustomSubtitleStylePreset(presetId: string): Promise<void> {
  if (isTauriEnvironment()) {
    await invoke("delete_custom_subtitle_style_preset", { presetId });
    return;
  }
  const idx = MOCK_PRESETS.findIndex((p) => p.id === presetId);
  if (idx >= 0 && !MOCK_PRESETS[idx].isBuiltin) {
    MOCK_PRESETS.splice(idx, 1);
  }
}

export async function getSystemFonts(): Promise<FontFamilyInfo[]> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<FontFamilyInfo[]>("get_system_fonts");
    } catch {
      return MOCK_FONTS;
    }
  }
  return MOCK_FONTS;
}

export async function generateAssScriptPreview(
  track: SubtitleTrack,
  style: SubtitleStyle
): Promise<string> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<string>("generate_ass_script_preview", { track, style });
    } catch {
      return `[Script Info]\nTitle: Mock ASS Preview\n\n[Events]\nDialogue: 0,0:00:00.00,0:00:05.00,Default,,0,0,0,,${track.cues[0]?.text || "Preview"}`;
    }
  }
  return `[Script Info]\nTitle: Mock ASS Preview\n\n[Events]\nDialogue: 0,0:00:00.00,0:00:05.00,Default,,0,0,0,,${track.cues[0]?.text || "Preview"}`;
}

export async function startSubtitleBurnInExport(
  request: SubtitleBurnInRequest
): Promise<string> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<string>("start_subtitle_burn_in_export", { request });
    } catch {
      return request.jobId;
    }
  }
  return request.jobId;
}

export async function getSubtitleBurnInStatus(): Promise<SubtitleBurnInStatus> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<SubtitleBurnInStatus>("get_subtitle_burn_in_status");
    } catch {
      return {
        stage: "readyToSave",
        stagedOutputPath: "/tmp/mock_burned.mp4",
        sizeBytes: 1048576,
        durationSecs: 10.0,
        width: 1920,
        height: 1080,
      };
    }
  }
  return {
    stage: "readyToSave",
    stagedOutputPath: "/tmp/mock_burned.mp4",
    sizeBytes: 1048576,
    durationSecs: 10.0,
    width: 1920,
    height: 1080,
  };
}

export async function cancelSubtitleBurnInExport(): Promise<void> {
  if (isTauriEnvironment()) {
    try {
      await invoke("cancel_subtitle_burn_in_export");
    } catch {
      // Graceful no-op
    }
  }
}

export async function finalizeBurnInExport(
  destinationPath: string,
  sourceOriginalPath: string
): Promise<string> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<string>("finalize_burn_in_export", {
        destinationPath,
        sourceOriginalPath,
      });
    } catch {
      return destinationPath;
    }
  }
  return destinationPath;
}
