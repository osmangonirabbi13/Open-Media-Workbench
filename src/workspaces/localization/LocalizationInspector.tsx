import React, { useState } from "react";
import type { MediaAsset } from "../../types/media";
import type {
  SubtitleDocument,
  SubtitleFormat,
  SubtitleTrack,
} from "../../types/subtitles";
import {
  SubtitlesIcon,
  GlobeIcon,
  MicIcon,
  UploadIcon,
  SparklesIcon,
  ShieldCheckIcon,
  DownloadIcon,
  CheckIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";
import { SubtitleTrackManager } from "./SubtitleTrackManager";
import { exportSubtitleTrack } from "../../lib/subtitles";

export type LocalizationTab = "subtitles" | "translation" | "dubbing" | "export";

export interface LocalizationInspectorProps {
  asset: MediaAsset | null;
  activeTab?: LocalizationTab;
  onTabChange?: (tab: LocalizationTab) => void;
  document: SubtitleDocument | null;
  activeTrack: SubtitleTrack | null;
  currentTime: number;
  onDocumentChange: (doc: SubtitleDocument | null) => void;
  onActiveTrackChange: (trackId: string) => void;
  onToggleTrackVisibility: (trackId: string) => void;
  onSeek: (seconds: number) => void;
}

export function LocalizationInspector({
  asset,
  activeTab: controlledTab,
  onTabChange,
  document,
  activeTrack,
  currentTime,
  onDocumentChange,
  onActiveTrackChange,
  onToggleTrackVisibility,
  onSeek,
}: LocalizationInspectorProps): React.ReactElement {
  const [internalTab, setInternalTab] = useState<LocalizationTab>("subtitles");
  const activeTab = controlledTab !== undefined ? controlledTab : internalTab;

  const [exportFormat, setExportFormat] = useState<SubtitleFormat>("srt");
  const [exportFeedback, setExportFeedback] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);

  const handleTabClick = (tab: LocalizationTab) => {
    setInternalTab(tab);
    onTabChange?.(tab);
  };

  const handleRunExport = async () => {
    if (!activeTrack) return;
    setIsExporting(true);
    setExportFeedback(null);
    try {
      const res = await exportSubtitleTrack(activeTrack, exportFormat);
      if (res) {
        setExportFeedback(`Exported ${activeTrack.displayName} to ${res.exportedPath}`);
      }
    } catch (err) {
      setExportFeedback(`Export failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsExporting(false);
    }
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
      <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
        {/* SUBTITLES PANEL */}
        {activeTab === "subtitles" && (
          <div id="loc-panel-subtitles" role="tabpanel" aria-labelledby="loc-tab-subtitles" className="h-full flex flex-col">
            <SubtitleTrackManager
              asset={asset}
              document={document}
              activeTrack={activeTrack}
              currentTime={currentTime}
              onDocumentChange={onDocumentChange}
              onActiveTrackChange={onActiveTrackChange}
              onToggleTrackVisibility={onToggleTrackVisibility}
              onSeek={onSeek}
            />
          </div>
        )}

        {/* TRANSLATION PANEL */}
        {activeTab === "translation" && (
          <div id="loc-panel-translation" role="tabpanel" aria-labelledby="loc-tab-translation" className="p-4 space-y-4 overflow-y-auto">
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
          <div id="loc-panel-dubbing" role="tabpanel" aria-labelledby="loc-tab-dubbing" className="p-4 space-y-4 overflow-y-auto">
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
          <div id="loc-panel-export" role="tabpanel" aria-labelledby="loc-tab-export" className="p-4 space-y-4 overflow-y-auto">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold text-zinc-200 flex items-center space-x-1.5">
                <UploadIcon size={14} className="text-indigo-400" />
                <span>Localization Export</span>
              </h4>
              <span className="px-1.5 py-0.2 rounded text-[10px] bg-emerald-950 text-emerald-300 border border-emerald-800">
                STEP 16 Active
              </span>
            </div>

            {/* Subtitle Export Section */}
            <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-3">
              <div className="flex items-center justify-between">
                <label className="text-xs font-semibold text-zinc-200 block">
                  Export Subtitle File (.srt / .vtt)
                </label>
                <span className="text-[10px] text-zinc-400 font-mono">
                  {activeTrack ? `${activeTrack.displayName} (${activeTrack.cues.length} cues)` : "No Active Track"}
                </span>
              </div>

              <div className="space-y-2">
                <label className="text-[11px] text-zinc-400 block">Export Format</label>
                <div className="flex items-center space-x-2">
                  <label className="flex items-center space-x-1.5 text-xs text-zinc-300 cursor-pointer">
                    <input
                      type="radio"
                      name="exportFormat"
                      checked={exportFormat === "srt"}
                      onChange={() => setExportFormat("srt")}
                      className="text-indigo-600 focus:ring-indigo-500"
                    />
                    <span>SubRip (.srt)</span>
                  </label>
                  <label className="flex items-center space-x-1.5 text-xs text-zinc-300 cursor-pointer ml-3">
                    <input
                      type="radio"
                      name="exportFormat"
                      checked={exportFormat === "webVtt"}
                      onChange={() => setExportFormat("webVtt")}
                      className="text-indigo-600 focus:ring-indigo-500"
                    />
                    <span>WebVTT (.vtt)</span>
                  </label>
                </div>
              </div>

              <Button
                size="xs"
                variant="primary"
                onClick={handleRunExport}
                disabled={!activeTrack || isExporting}
                className="w-full bg-indigo-600 hover:bg-indigo-500 text-white font-medium"
              >
                <DownloadIcon size={12} className="mr-1.5" />
                {activeTrack
                  ? `Export ${activeTrack.displayName} as ${exportFormat.toUpperCase()}`
                  : "Select a track to export"}
              </Button>

              {exportFeedback && (
                <div className="text-[11px] p-2 rounded bg-zinc-950 border border-zinc-800 text-zinc-300 flex items-center space-x-1.5">
                  <CheckIcon size={12} className="text-indigo-400 shrink-0" />
                  <span className="truncate">{exportFeedback}</span>
                </div>
              )}
            </div>

            {/* Future Export Capabilities Notice */}
            <div className="p-3.5 rounded-lg border border-zinc-800/80 bg-zinc-900/30 space-y-2.5">
              <div className="text-xs font-semibold text-zinc-400">Advanced Video Muxing (Scheduled)</div>
              <div className="space-y-2 opacity-60">
                <label className="flex items-center space-x-2 text-xs text-zinc-400 cursor-not-allowed">
                  <input type="checkbox" disabled className="rounded bg-zinc-800 border-zinc-700" />
                  <span>Burn-in Subtitles onto Video (Scheduled STEP 21)</span>
                </label>
                <label className="flex items-center space-x-2 text-xs text-zinc-400 cursor-not-allowed">
                  <input type="checkbox" disabled className="rounded bg-zinc-800 border-zinc-700" />
                  <span>Multiplex Dubbed Audio Stream (Scheduled STEP 30)</span>
                </label>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
