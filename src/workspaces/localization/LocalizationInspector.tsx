import React, { useState } from "react";
import type { MediaAsset } from "../../types/media";
import {
  SubtitlesIcon,
  GlobeIcon,
  MicIcon,
  UploadIcon,
  SparklesIcon,
  ShieldCheckIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";

export type LocalizationTab = "subtitles" | "translation" | "dubbing" | "export";

export interface LocalizationInspectorProps {
  asset: MediaAsset | null;
  activeTab?: LocalizationTab;
  onTabChange?: (tab: LocalizationTab) => void;
}

export function LocalizationInspector({
  asset,
  activeTab: controlledTab,
  onTabChange,
}: LocalizationInspectorProps): React.ReactElement {
  const [internalTab, setInternalTab] = useState<LocalizationTab>("subtitles");
  const activeTab = controlledTab !== undefined ? controlledTab : internalTab;

  const handleTabClick = (tab: LocalizationTab) => {
    setInternalTab(tab);
    onTabChange?.(tab);
  };

  return (
    <div className="h-full flex flex-col bg-zinc-950 text-zinc-100 select-none">
      {/* Tab Navigation Header */}
      <div
        role="tablist"
        aria-label="Localization Inspector Views"
        className="h-10 shrink-0 border-b border-zinc-800 bg-zinc-900/60 px-2 flex items-center space-x-1"
      >
        <button
          role="tab"
          id="loc-tab-subtitles"
          aria-selected={activeTab === "subtitles"}
          aria-controls="loc-panel-subtitles"
          tabIndex={activeTab === "subtitles" ? 0 : -1}
          onClick={() => handleTabClick("subtitles")}
          className={`flex items-center space-x-1.5 px-2.5 py-1.5 rounded-md text-xs font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-indigo-500 ${
            activeTab === "subtitles"
              ? "bg-zinc-800 text-zinc-100 shadow-xs"
              : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
          }`}
        >
          <SubtitlesIcon size={13} className={activeTab === "subtitles" ? "text-indigo-400" : ""} />
          <span>Subtitles</span>
        </button>

        <button
          role="tab"
          id="loc-tab-translation"
          aria-selected={activeTab === "translation"}
          aria-controls="loc-panel-translation"
          tabIndex={activeTab === "translation" ? 0 : -1}
          onClick={() => handleTabClick("translation")}
          className={`flex items-center space-x-1.5 px-2.5 py-1.5 rounded-md text-xs font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-indigo-500 ${
            activeTab === "translation"
              ? "bg-zinc-800 text-zinc-100 shadow-xs"
              : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
          }`}
        >
          <GlobeIcon size={13} className={activeTab === "translation" ? "text-indigo-400" : ""} />
          <span>Translate</span>
        </button>

        <button
          role="tab"
          id="loc-tab-dubbing"
          aria-selected={activeTab === "dubbing"}
          aria-controls="loc-panel-dubbing"
          tabIndex={activeTab === "dubbing" ? 0 : -1}
          onClick={() => handleTabClick("dubbing")}
          className={`flex items-center space-x-1.5 px-2.5 py-1.5 rounded-md text-xs font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-indigo-500 ${
            activeTab === "dubbing"
              ? "bg-zinc-800 text-zinc-100 shadow-xs"
              : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
          }`}
        >
          <MicIcon size={13} className={activeTab === "dubbing" ? "text-indigo-400" : ""} />
          <span>Dubbing</span>
        </button>

        <button
          role="tab"
          id="loc-tab-export"
          aria-selected={activeTab === "export"}
          aria-controls="loc-panel-export"
          tabIndex={activeTab === "export" ? 0 : -1}
          onClick={() => handleTabClick("export")}
          className={`flex items-center space-x-1.5 px-2.5 py-1.5 rounded-md text-xs font-medium transition-colors outline-none focus-visible:ring-1 focus-visible:ring-indigo-500 ${
            activeTab === "export"
              ? "bg-zinc-800 text-zinc-100 shadow-xs"
              : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
          }`}
        >
          <UploadIcon size={13} className={activeTab === "export" ? "text-indigo-400" : ""} />
          <span>Export</span>
        </button>
      </div>

      {/* Tab Panels */}
      <div className="flex-1 min-h-0 overflow-y-auto p-4 select-text">
        {/* SUBTITLES PANEL */}
        {activeTab === "subtitles" && (
          <div id="loc-panel-subtitles" role="tabpanel" aria-labelledby="loc-tab-subtitles" className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold text-zinc-200 flex items-center space-x-1.5">
                <SubtitlesIcon size={14} className="text-indigo-400" />
                <span>Subtitle Cues & Transcription</span>
              </h4>
              <span className="px-1.5 py-0.2 rounded text-[10px] bg-zinc-800 text-zinc-400 border border-zinc-700/50">
                Scheduled: STEP 16–18
              </span>
            </div>

            <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-2.5">
              <label className="text-xs font-medium text-zinc-300 block">Audio Transcription Engine</label>
              <div className="flex items-center space-x-2">
                <select
                  disabled
                  className="flex-1 text-xs bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1.5 text-zinc-500 cursor-not-allowed"
                >
                  <option>Local Whisper Engine (Offline ggml-small)</option>
                  <option>Local Whisper Engine (Offline ggml-base)</option>
                </select>
                <Button size="xs" variant="secondary" disabled>
                  Transcribe
                </Button>
              </div>
              <p className="text-[11px] text-zinc-500">
                Transcribes speech directly on-device using local Whisper weights without cloud uploads.
              </p>
            </div>

            {/* Subtitle Cue List Truthful Empty State */}
            <div className="p-5 rounded-lg border border-dashed border-zinc-800 text-center space-y-2">
              <SubtitlesIcon size={24} className="mx-auto text-zinc-600" />
              <div className="text-xs font-medium text-zinc-400">No Subtitle Cues Generated</div>
              <p className="text-[11px] text-zinc-500 max-w-xs mx-auto leading-relaxed">
                {asset
                  ? "Local AI transcription and interactive subtitle editing (splitting, retiming, merging) will be unlocked in STEPS 16–18."
                  : "Select a media asset from the library to configure subtitle localization."}
              </p>
            </div>
          </div>
        )}

        {/* TRANSLATION PANEL */}
        {activeTab === "translation" && (
          <div id="loc-panel-translation" role="tabpanel" aria-labelledby="loc-tab-translation" className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold text-zinc-200 flex items-center space-x-1.5">
                <GlobeIcon size={14} className="text-indigo-400" />
                <span>Multilingual Translation</span>
              </h4>
              <span className="px-1.5 py-0.2 rounded text-[10px] bg-zinc-800 text-zinc-400 border border-zinc-700/50">
                Scheduled: STEP 22–23
              </span>
            </div>

            <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-3">
              <div className="grid grid-cols-2 gap-2">
                <div>
                  <label className="text-[11px] text-zinc-400 block mb-1">Source Language</label>
                  <select
                    disabled
                    className="w-full text-xs bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-500 cursor-not-allowed"
                  >
                    <option>Auto Detect (English)</option>
                  </select>
                </div>
                <div>
                  <label className="text-[11px] text-zinc-400 block mb-1">Target Language</label>
                  <select
                    disabled
                    className="w-full text-xs bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-500 cursor-not-allowed"
                  >
                    <option>Spanish (Español)</option>
                    <option>French (Français)</option>
                    <option>German (Deutsch)</option>
                    <option>Japanese (日本語)</option>
                    <option>Bengali (বাংলা)</option>
                  </select>
                </div>
              </div>

              <Button size="xs" variant="secondary" disabled className="w-full">
                Translate Subtitles (Scheduled STEP 22)
              </Button>
            </div>

            <div className="p-4 rounded-lg border border-indigo-900/40 bg-indigo-950/20 text-indigo-300 text-xs flex items-start space-x-2">
              <SparklesIcon size={16} className="shrink-0 mt-0.5 text-indigo-400" />
              <p className="text-[11px] leading-relaxed">
                Offline neural machine translation will translate subtitle cues with side-by-side bilingual review in STEP 22–23.
              </p>
            </div>
          </div>
        )}

        {/* VOICE & DUBBING PANEL */}
        {activeTab === "dubbing" && (
          <div id="loc-panel-dubbing" role="tabpanel" aria-labelledby="loc-tab-dubbing" className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold text-zinc-200 flex items-center space-x-1.5">
                <MicIcon size={14} className="text-indigo-400" />
                <span>AI Voice Synthesis & Dubbing</span>
              </h4>
              <span className="px-1.5 py-0.2 rounded text-[10px] bg-zinc-800 text-zinc-400 border border-zinc-700/50">
                Scheduled: STEP 24–28
              </span>
            </div>

            <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-3">
              <div>
                <label className="text-[11px] text-zinc-400 block mb-1">Neural Voice Model</label>
                <select
                  disabled
                  className="w-full text-xs bg-zinc-900 border border-zinc-800 rounded px-2 py-1 text-zinc-500 cursor-not-allowed"
                >
                  <option>Piper TTS — Local Studio Natural Voice (Permissive)</option>
                </select>
              </div>

              <div className="grid grid-cols-2 gap-2 text-[11px] text-zinc-400">
                <div>
                  <label className="block mb-1">Audio Ducking</label>
                  <input type="range" disabled min="0" max="100" defaultValue="70" className="w-full opacity-50 cursor-not-allowed" />
                </div>
                <div>
                  <label className="block mb-1">Sync Time-Scale</label>
                  <input type="range" disabled min="80" max="120" defaultValue="100" className="w-full opacity-50 cursor-not-allowed" />
                </div>
              </div>

              <Button size="xs" variant="secondary" disabled className="w-full">
                Generate Dubbed Audio Track (Scheduled STEP 24)
              </Button>
            </div>

            <div className="p-4 rounded-lg border border-indigo-900/40 bg-indigo-950/20 text-indigo-300 text-xs flex items-start space-x-2">
              <ShieldCheckIcon size={16} className="shrink-0 mt-0.5 text-indigo-400" />
              <p className="text-[11px] leading-relaxed">
                All voice synthesis runs locally on your CPU/GPU with strict license-filtered models. Automatic audio time-stretching keeps speech synced with subtitle intervals.
              </p>
            </div>
          </div>
        )}

        {/* EXPORT PANEL */}
        {activeTab === "export" && (
          <div id="loc-panel-export" role="tabpanel" aria-labelledby="loc-tab-export" className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold text-zinc-200 flex items-center space-x-1.5">
                <UploadIcon size={14} className="text-indigo-400" />
                <span>Localization Export</span>
              </h4>
              <span className="px-1.5 py-0.2 rounded text-[10px] bg-zinc-800 text-zinc-400 border border-zinc-700/50">
                Scheduled: STEP 29–31
              </span>
            </div>

            <div className="p-3 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-3">
              <div className="space-y-2">
                <label className="flex items-center space-x-2 text-xs text-zinc-300 cursor-not-allowed">
                  <input type="checkbox" disabled checked className="rounded bg-zinc-800 border-zinc-700" />
                  <span>Export Subtitle File (.srt / .vtt)</span>
                </label>
                <label className="flex items-center space-x-2 text-xs text-zinc-300 cursor-not-allowed">
                  <input type="checkbox" disabled className="rounded bg-zinc-800 border-zinc-700" />
                  <span>Burn-in Subtitles onto Video</span>
                </label>
                <label className="flex items-center space-x-2 text-xs text-zinc-300 cursor-not-allowed">
                  <input type="checkbox" disabled className="rounded bg-zinc-800 border-zinc-700" />
                  <span>Multiplex Dubbed Audio Stream</span>
                </label>
              </div>

              <Button size="xs" variant="secondary" disabled className="w-full">
                Export Localized Video (Scheduled STEP 30)
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
