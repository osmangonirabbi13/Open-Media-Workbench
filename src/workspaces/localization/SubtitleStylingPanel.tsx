import React, { useEffect, useState } from "react";
import type {
  FontFamilyInfo,
  SubtitleAnimation,
  SubtitleStyle,
  SubtitleStylePreset,
  TextAlignment,
} from "../../types/styling";
import {
  colorRgbaToHex,
  deleteCustomSubtitleStylePreset,
  getSubtitleStylePresets,
  getSystemFonts,
  parseHexToColorRgba,
  saveCustomSubtitleStylePreset,
} from "../../lib/styling";
import { Button } from "../../components/ui/Button";
import {
  CheckIcon,
  SparklesIcon,
  DownloadIcon,
} from "../../components/ui/Icons";

export interface SubtitleStylingPanelProps {
  currentStyle: SubtitleStyle;
  onStyleChange: (newStyle: SubtitleStyle) => void;
  onNavigateToExport?: () => void;
}

export function SubtitleStylingPanel({
  currentStyle,
  onStyleChange,
  onNavigateToExport,
}: SubtitleStylingPanelProps): React.ReactElement {
  const [presets, setPresets] = useState<SubtitleStylePreset[]>([]);
  const [systemFonts, setSystemFonts] = useState<FontFamilyInfo[]>([]);
  const [activePresetId, setActivePresetId] = useState<string>(currentStyle.id || "clean-minimal");
  const [customPresetName, setCustomPresetName] = useState("");
  const [showSavePresetModal, setShowSavePresetModal] = useState(false);
  const [notification, setNotification] = useState<string | null>(null);

  useEffect(() => {
    getSubtitleStylePresets().then(setPresets);
    getSystemFonts().then(setSystemFonts);
  }, []);

  const showToast = (msg: string) => {
    setNotification(msg);
    setTimeout(() => setNotification(null), 3000);
  };

  const handleSelectPreset = (preset: SubtitleStylePreset) => {
    setActivePresetId(preset.id);
    onStyleChange({ ...preset.style });
    showToast(`Applied preset: ${preset.displayName}`);
  };

  const handleUpdate = <K extends keyof SubtitleStyle>(key: K, value: SubtitleStyle[K]) => {
    onStyleChange({ ...currentStyle, [key]: value });
  };

  const handleSaveCustomPreset = async () => {
    if (!customPresetName.trim()) return;
    const newId = `custom-${Date.now()}`;
    const newPreset: SubtitleStylePreset = {
      id: newId,
      displayName: customPresetName.trim(),
      description: "User-defined custom subtitle style preset.",
      isBuiltin: false,
      style: {
        ...currentStyle,
        id: newId,
        displayName: customPresetName.trim(),
      },
    };

    try {
      await saveCustomSubtitleStylePreset(newPreset);
      const updated = await getSubtitleStylePresets();
      setPresets(updated);
      setActivePresetId(newId);
      setShowSavePresetModal(false);
      setCustomPresetName("");
      showToast(`Saved custom preset: ${newPreset.displayName}`);
    } catch (e) {
      showToast(`Error saving preset: ${e}`);
    }
  };

  const handleDeleteCustomPreset = async (id: string, name: string) => {
    try {
      await deleteCustomSubtitleStylePreset(id);
      const updated = await getSubtitleStylePresets();
      setPresets(updated);
      if (activePresetId === id) {
        handleSelectPreset(updated[0]);
      }
      showToast(`Deleted preset: ${name}`);
    } catch (e) {
      showToast(`Error deleting preset: ${e}`);
    }
  };

  return (
    <div className="space-y-6 text-xs text-zinc-300 pb-8">
      {/* Toast Notification */}
      {notification && (
        <div className="p-2.5 rounded bg-amber-500/10 border border-amber-500/30 text-amber-300 flex items-center space-x-2">
          <CheckIcon size={14} className="shrink-0" />
          <span>{notification}</span>
        </div>
      )}

      {/* Preset Gallery */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between">
          <label className="text-xs font-semibold text-zinc-100 uppercase tracking-wider flex items-center space-x-1.5">
            <SparklesIcon size={14} className="text-amber-400" />
            <span>Caption Style Presets</span>
          </label>
          <button
            onClick={() => setShowSavePresetModal(true)}
            className="text-[11px] text-amber-400 hover:text-amber-300 font-medium"
          >
            + Save Custom
          </button>
        </div>

        <div className="grid grid-cols-2 gap-2">
          {presets.map((preset) => {
            const isSelected = activePresetId === preset.id;
            return (
              <div
                key={preset.id}
                onClick={() => handleSelectPreset(preset)}
                className={`p-2.5 rounded-lg border text-left cursor-pointer transition-all relative group ${
                  isSelected
                    ? "bg-amber-950/30 border-amber-500/60 ring-1 ring-amber-500/40"
                    : "bg-zinc-900/60 border-zinc-800 hover:border-zinc-700"
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-zinc-100 text-xs truncate">
                    {preset.displayName}
                  </span>
                  {!preset.isBuiltin && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteCustomPreset(preset.id, preset.displayName);
                      }}
                      className="opacity-0 group-hover:opacity-100 text-red-400 hover:text-red-300 text-[10px] ml-1"
                      title="Delete custom preset"
                    >
                      ×
                    </button>
                  )}
                </div>
                <p className="text-[10px] text-zinc-400 line-clamp-2 mt-1 leading-snug">
                  {preset.description}
                </p>
              </div>
            );
          })}
        </div>
      </div>

      {/* Typography Section */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/40 space-y-3">
        <span className="text-xs font-semibold text-zinc-200 block border-b border-zinc-800 pb-1.5">
          Typography & Font
        </span>

        <div className="space-y-1.5">
          <label className="text-[11px] text-zinc-400 block">Font Family</label>
          <select
            value={currentStyle.fontFamily}
            onChange={(e) => handleUpdate("fontFamily", e.target.value)}
            className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200"
          >
            {systemFonts.map((font) => (
              <option key={font.family} value={font.family}>
                {font.family} {font.supportsBengali ? "(বাংলা)" : ""} {!font.isInstalled ? "(Fallback)" : ""}
              </option>
            ))}
          </select>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <div className="flex justify-between">
              <label className="text-[11px] text-zinc-400">Font Size</label>
              <span className="text-[11px] font-mono text-zinc-300">{currentStyle.fontSize}px</span>
            </div>
            <input
              type="range"
              min={14}
              max={64}
              value={currentStyle.fontSize}
              onChange={(e) => handleUpdate("fontSize", parseInt(e.target.value, 10))}
              className="w-full accent-amber-500"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11px] text-zinc-400 block">Font Weight</label>
            <select
              value={currentStyle.fontWeight}
              onChange={(e) => handleUpdate("fontWeight", parseInt(e.target.value, 10))}
              className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200"
            >
              <option value={400}>Regular (400)</option>
              <option value={500}>Medium (500)</option>
              <option value={600}>Semi-Bold (600)</option>
              <option value={700}>Bold (700)</option>
              <option value={800}>Extra Bold (800)</option>
            </select>
          </div>
        </div>

        <div className="flex items-center space-x-4 pt-1">
          <label className="flex items-center space-x-2 text-xs cursor-pointer">
            <input
              type="checkbox"
              checked={currentStyle.italic}
              onChange={(e) => handleUpdate("italic", e.target.checked)}
              className="rounded bg-zinc-800 border-zinc-700 text-amber-500 focus:ring-0"
            />
            <span>Italic</span>
          </label>
        </div>
      </div>

      {/* Colors & Appearance Section */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/40 space-y-3.5">
        <span className="text-xs font-semibold text-zinc-200 block border-b border-zinc-800 pb-1.5">
          Colors & Outline
        </span>

        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <label className="text-[11px] text-zinc-400 block">Text Color</label>
            <div className="flex items-center space-x-2">
              <input
                type="color"
                value={colorRgbaToHex(currentStyle.textColor)}
                onChange={(e) => handleUpdate("textColor", parseHexToColorRgba(e.target.value))}
                className="w-7 h-7 rounded border border-zinc-700 bg-transparent cursor-pointer"
              />
              <span className="text-[11px] font-mono">{colorRgbaToHex(currentStyle.textColor)}</span>
            </div>
          </div>

          <div className="space-y-1">
            <label className="text-[11px] text-zinc-400 block">Karaoke Word Color</label>
            <div className="flex items-center space-x-2">
              <input
                type="color"
                value={colorRgbaToHex(currentStyle.wordHighlightColor)}
                onChange={(e) => handleUpdate("wordHighlightColor", parseHexToColorRgba(e.target.value))}
                className="w-7 h-7 rounded border border-zinc-700 bg-transparent cursor-pointer"
              />
              <span className="text-[11px] font-mono">{colorRgbaToHex(currentStyle.wordHighlightColor)}</span>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3 pt-1">
          <div className="space-y-1">
            <label className="text-[11px] text-zinc-400 block">Outline Color</label>
            <div className="flex items-center space-x-2">
              <input
                type="color"
                value={colorRgbaToHex(currentStyle.outlineColor)}
                onChange={(e) => handleUpdate("outlineColor", parseHexToColorRgba(e.target.value))}
                className="w-7 h-7 rounded border border-zinc-700 bg-transparent cursor-pointer"
              />
              <span className="text-[11px] font-mono">{colorRgbaToHex(currentStyle.outlineColor)}</span>
            </div>
          </div>

          <div className="space-y-1">
            <div className="flex justify-between">
              <label className="text-[11px] text-zinc-400">Outline Width</label>
              <span className="text-[11px] font-mono">{currentStyle.outlineWidth}px</span>
            </div>
            <input
              type="range"
              min={0}
              max={6}
              step={0.5}
              value={currentStyle.outlineWidth}
              onChange={(e) => handleUpdate("outlineWidth", parseFloat(e.target.value))}
              className="w-full accent-amber-500"
            />
          </div>
        </div>

        {/* Background Box */}
        <div className="border-t border-zinc-800/80 pt-2 space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-[11px] text-zinc-300 font-medium">Background Box</label>
            <span className="text-[10px] text-zinc-400 font-mono">
              {Math.round(currentStyle.backgroundOpacity * 100)}% Opacity
            </span>
          </div>

          <div className="grid grid-cols-2 gap-3 items-center">
            <div className="flex items-center space-x-2">
              <input
                type="color"
                value={colorRgbaToHex(currentStyle.backgroundColor)}
                onChange={(e) => handleUpdate("backgroundColor", parseHexToColorRgba(e.target.value))}
                className="w-7 h-7 rounded border border-zinc-700 bg-transparent cursor-pointer"
              />
              <span className="text-[11px] font-mono">{colorRgbaToHex(currentStyle.backgroundColor)}</span>
            </div>

            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={currentStyle.backgroundOpacity}
              onChange={(e) => handleUpdate("backgroundOpacity", parseFloat(e.target.value))}
              className="w-full accent-amber-500"
            />
          </div>
        </div>
      </div>

      {/* Positioning & Alignment */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/40 space-y-3">
        <span className="text-xs font-semibold text-zinc-200 block border-b border-zinc-800 pb-1.5">
          Position & Layout
        </span>

        <div className="space-y-1.5">
          <label className="text-[11px] text-zinc-400 block">Position on Screen</label>
          <div className="grid grid-cols-3 gap-1.5">
            {[
              { label: "Top Center", pos: { type: "topCenter" as const } },
              { label: "Center", pos: { type: "center" as const } },
              { label: "Bottom Center", pos: { type: "bottomCenter" as const } },
              { label: "Bottom Left", pos: { type: "bottomLeft" as const } },
              { label: "Bottom Right", pos: { type: "bottomRight" as const } },
            ].map((item) => (
              <button
                key={item.label}
                onClick={() => handleUpdate("position", item.pos)}
                className={`py-1.5 px-2 rounded border text-[11px] font-medium truncate ${
                  currentStyle.position.type === item.pos.type
                    ? "bg-amber-500/20 border-amber-500/60 text-amber-300"
                    : "bg-zinc-950 border-zinc-800 hover:border-zinc-700 text-zinc-300"
                }`}
              >
                {item.label}
              </button>
            ))}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3 pt-1">
          <div className="space-y-1">
            <div className="flex justify-between">
              <label className="text-[11px] text-zinc-400">Vertical Margin</label>
              <span className="text-[11px] font-mono">{currentStyle.marginVertical}px</span>
            </div>
            <input
              type="range"
              min={10}
              max={100}
              value={currentStyle.marginVertical}
              onChange={(e) => handleUpdate("marginVertical", parseInt(e.target.value, 10))}
              className="w-full accent-amber-500"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11px] text-zinc-400 block">Text Alignment</label>
            <div className="flex space-x-1">
              {(["left", "center", "right"] as TextAlignment[]).map((align) => (
                <button
                  key={align}
                  onClick={() => handleUpdate("textAlignment", align)}
                  className={`flex-1 py-1 rounded border text-[11px] capitalize ${
                    currentStyle.textAlignment === align
                      ? "bg-amber-500/20 border-amber-500/60 text-amber-300 font-medium"
                      : "bg-zinc-950 border-zinc-800 text-zinc-400"
                  }`}
                >
                  {align}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Animation & Karaoke Section */}
      <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/40 space-y-3">
        <span className="text-xs font-semibold text-zinc-200 block border-b border-zinc-800 pb-1.5">
          Caption Animation & Timing
        </span>

        <div className="space-y-1.5">
          <label className="text-[11px] text-zinc-400 block">Animation Effect</label>
          <select
            value={currentStyle.animation}
            onChange={(e) => handleUpdate("animation", e.target.value as SubtitleAnimation)}
            className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1.5 text-xs text-zinc-200"
          >
            <option value="none">None (Standard Display)</option>
            <option value="fade">Fade In / Fade Out (\fad)</option>
            <option value="pop">Pop In (Bounce Scale)</option>
            <option value="karaokeHighlight">Karaoke Word Highlighting (\kf)</option>
          </select>
        </div>

        <p className="text-[10px] text-zinc-400 leading-relaxed">
          Karaoke highlighting utilizes genuine STEP 20 word timestamps. If word timing is absent on a cue, the subtitle falls back cleanly to the primary text color without errors.
        </p>
      </div>

      {/* Navigation Shortcut to Burn-In Export */}
      {onNavigateToExport && (
        <div className="pt-2">
          <Button
            size="sm"
            variant="primary"
            onClick={onNavigateToExport}
            className="w-full bg-amber-600 hover:bg-amber-500 text-black font-semibold flex items-center justify-center space-x-2"
          >
            <DownloadIcon size={14} />
            <span>Proceed to Video Burn-In Export</span>
          </Button>
        </div>
      )}

      {/* Save Custom Preset Modal */}
      {showSavePresetModal && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-xs flex items-center justify-center p-4">
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-sm p-4 space-y-3">
            <span className="text-sm font-semibold text-zinc-100 block">Save Custom Preset</span>
            <p className="text-xs text-zinc-400">
              Save your current styling choices as a reusable custom preset.
            </p>
            <input
              type="text"
              placeholder="e.g. My Branded Yellow Box"
              value={customPresetName}
              onChange={(e) => setCustomPresetName(e.target.value)}
              className="w-full bg-zinc-950 border border-zinc-700 rounded px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-amber-500"
            />
            <div className="flex justify-end space-x-2 pt-2">
              <Button size="xs" variant="subtle" onClick={() => setShowSavePresetModal(false)}>
                Cancel
              </Button>
              <Button
                size="xs"
                variant="primary"
                onClick={handleSaveCustomPreset}
                disabled={!customPresetName.trim()}
                className="bg-amber-600 hover:bg-amber-500 text-black font-semibold"
              >
                Save Preset
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
