import React, { useEffect, useRef, useState } from "react";
import {
  CheckIcon,
  CloseIcon,
  ResetIcon,
  ShieldCheckIcon,
} from "../ui/Icons";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import type { AppInfo } from "../../types/ipc";
import type { WorkspacePreferences } from "../../types/preferences";

export interface SettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  appInfo?: AppInfo;
  preferences: WorkspacePreferences;
  onResetLayout: () => void;
  onSetTimecodeFormat: (format: "timecode" | "frames") => void;
  onToggleSnap: () => void;
}

type SettingsTab = "appearance" | "workspace" | "privacy" | "about";

export function SettingsDialog({
  isOpen,
  onClose,
  appInfo,
  preferences,
  onResetLayout,
  onSetTimecodeFormat,
  onToggleSnap,
}: SettingsDialogProps): React.ReactElement | null {
  const [activeTab, setActiveTab] = useState<SettingsTab>("workspace");
  const dialogRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedElementRef = useRef<HTMLElement | null>(null);

  // Focus management & Escape key listener
  useEffect(() => {
    if (isOpen) {
      previouslyFocusedElementRef.current = document.activeElement as HTMLElement | null;
      dialogRef.current?.focus();

      const handleKeyDown = (e: KeyboardEvent): void => {
        if (e.key === "Escape") {
          e.preventDefault();
          onClose();
        }
      };

      window.addEventListener("keydown", handleKeyDown);
      return () => {
        window.removeEventListener("keydown", handleKeyDown);
        previouslyFocusedElementRef.current?.focus();
      };
    }
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-xs select-none"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      aria-modal="true"
      role="dialog"
      aria-labelledby="settings-dialog-title"
    >
      <div
        ref={dialogRef}
        tabIndex={-1}
        className="w-full max-w-2xl rounded-xl border border-zinc-800 bg-zinc-950 shadow-2xl flex flex-col overflow-hidden focus:outline-none"
      >
        {/* Header */}
        <div className="h-12 shrink-0 flex items-center justify-between px-5 border-b border-zinc-800 bg-zinc-900/60">
          <h2
            id="settings-dialog-title"
            className="text-sm font-semibold text-zinc-100 tracking-tight"
          >
            Preferences & Settings
          </h2>
          <IconButton
            icon={<CloseIcon size={14} />}
            label="Close Settings (Escape)"
            size="sm"
            variant="subtle"
            onClick={onClose}
          />
        </div>

        {/* Body with Sidebar Tabs */}
        <div className="flex h-96">
          {/* Sidebar Tabs */}
          <nav
            aria-label="Settings Categories"
            className="w-44 shrink-0 border-r border-zinc-800/80 bg-zinc-900/30 p-2.5 flex flex-col space-y-1"
          >
            <button
              type="button"
              onClick={() => setActiveTab("workspace")}
              className={`w-full text-left px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                activeTab === "workspace"
                  ? "bg-indigo-600/20 text-indigo-300 font-semibold border border-indigo-500/30"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
              }`}
            >
              Workspace Layout
            </button>
            <button
              type="button"
              onClick={() => setActiveTab("appearance")}
              className={`w-full text-left px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                activeTab === "appearance"
                  ? "bg-indigo-600/20 text-indigo-300 font-semibold border border-indigo-500/30"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
              }`}
            >
              Appearance & Theme
            </button>
            <button
              type="button"
              onClick={() => setActiveTab("privacy")}
              className={`w-full text-left px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                activeTab === "privacy"
                  ? "bg-indigo-600/20 text-indigo-300 font-semibold border border-indigo-500/30"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
              }`}
            >
              Privacy & Local Engine
            </button>
            <button
              type="button"
              onClick={() => setActiveTab("about")}
              className={`w-full text-left px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                activeTab === "about"
                  ? "bg-indigo-600/20 text-indigo-300 font-semibold border border-indigo-500/30"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
              }`}
            >
              About & System
            </button>
          </nav>

          {/* Tab Content Panel */}
          <div className="flex-1 p-6 overflow-y-auto text-xs text-zinc-300">
            {/* WORKSPACE TAB */}
            {activeTab === "workspace" && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-zinc-100 mb-1">
                    Panel Layout & Dimensions
                  </h3>
                  <p className="text-zinc-400 leading-relaxed mb-4">
                    Workspace splitters automatically persist to local storage. You can restore the default video editing geometry at any time.
                  </p>
                  <div className="grid grid-cols-2 gap-3 mb-4 text-xs font-mono">
                    <div className="p-3 rounded-lg bg-zinc-900 border border-zinc-800">
                      <span className="text-zinc-500 block mb-1">Media Panel Width</span>
                      <span className="text-zinc-200 font-semibold">
                        {Math.round(preferences.mediaPanelWidth)} px
                      </span>
                    </div>
                    <div className="p-3 rounded-lg bg-zinc-900 border border-zinc-800">
                      <span className="text-zinc-500 block mb-1">Timeline Deck Height</span>
                      <span className="text-zinc-200 font-semibold">
                        {Math.round(preferences.timelineDeckHeight)} px
                      </span>
                    </div>
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<ResetIcon size={13} />}
                    onClick={onResetLayout}
                  >
                    Reset Layout to Default
                  </Button>
                </div>

                <div className="border-t border-zinc-800/80 pt-5 space-y-3">
                  <h3 className="text-sm font-semibold text-zinc-100">
                    Timecode & Editing Controls
                  </h3>
                  <div className="space-y-2">
                    <label className="text-zinc-400 block">Timecode Display Mode:</label>
                    <div className="flex space-x-2">
                      <button
                        type="button"
                        onClick={() => onSetTimecodeFormat("timecode")}
                        className={`px-3 py-1.5 rounded-lg border text-xs font-medium cursor-pointer ${
                          preferences.timecodeFormat === "timecode"
                            ? "bg-indigo-600/20 text-indigo-300 border-indigo-500/50"
                            : "bg-zinc-900 text-zinc-400 border-zinc-800 hover:text-zinc-200"
                        }`}
                      >
                        Standard SMPTE (HH:MM:SS:FF)
                      </button>
                      <button
                        type="button"
                        onClick={() => onSetTimecodeFormat("frames")}
                        className={`px-3 py-1.5 rounded-lg border text-xs font-medium cursor-pointer ${
                          preferences.timecodeFormat === "frames"
                            ? "bg-indigo-600/20 text-indigo-300 border-indigo-500/50"
                            : "bg-zinc-900 text-zinc-400 border-zinc-800 hover:text-zinc-200"
                        }`}
                      >
                        Absolute Frame Count (Frames)
                      </button>
                    </div>
                  </div>

                  <div className="pt-2">
                    <label className="flex items-center space-x-2 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={preferences.snapEnabled}
                        onChange={onToggleSnap}
                        className="rounded border-zinc-700 bg-zinc-900 text-indigo-600 focus:ring-indigo-500"
                      />
                      <span className="text-zinc-300">
                        Enable magnetic timeline snapping (Shortcut: S)
                      </span>
                    </label>
                  </div>
                </div>
              </div>
            )}

            {/* APPEARANCE TAB */}
            {activeTab === "appearance" && (
              <div className="space-y-6">
                <div>
                  <h3 className="text-sm font-semibold text-zinc-100 mb-1">
                    Application Theme
                  </h3>
                  <p className="text-zinc-400 leading-relaxed mb-4">
                    OpenTake is optimized for low-fatigue studio color grading and editing.
                  </p>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="p-4 rounded-xl border border-indigo-500/60 bg-zinc-900 flex items-center justify-between">
                      <div>
                        <div className="font-semibold text-zinc-100 flex items-center space-x-2">
                          <span>Studio Dark</span>
                          <span className="px-1.5 py-0.2 rounded text-[10px] bg-indigo-950 text-indigo-300 border border-indigo-800/40">
                            Active
                          </span>
                        </div>
                        <p className="text-[11px] text-zinc-400 mt-1">
                          Standard high-contrast dark theme
                        </p>
                      </div>
                      <CheckIcon size={16} className="text-indigo-400" />
                    </div>

                    <div className="p-4 rounded-xl border border-zinc-800/60 bg-zinc-900/30 opacity-50 cursor-not-allowed">
                      <div className="font-semibold text-zinc-400 flex items-center space-x-2">
                        <span>Studio Light</span>
                        <span className="px-1.5 py-0.2 rounded text-[10px] bg-zinc-800 text-zinc-500">
                          Future
                        </span>
                      </div>
                      <p className="text-[11px] text-zinc-500 mt-1">
                        Reserved for future release
                      </p>
                    </div>
                  </div>
                </div>

                <div className="border-t border-zinc-800/80 pt-5">
                  <h3 className="text-sm font-semibold text-zinc-100 mb-1">Typography</h3>
                  <p className="text-zinc-400">
                    Native system typography stack with zero remote font downloads. Reduced motion media queries respected automatically.
                  </p>
                </div>
              </div>
            )}

            {/* PRIVACY TAB */}
            {activeTab === "privacy" && (
              <div className="space-y-4">
                <div className="flex items-center space-x-2 text-emerald-400">
                  <ShieldCheckIcon size={20} />
                  <h3 className="text-sm font-semibold text-zinc-100">
                    Offline-First Privacy Architecture
                  </h3>
                </div>

                <p className="text-zinc-400 leading-relaxed">
                  OpenTake adheres to an uncompromising offline-first standard designed for complete creator sovereignty.
                </p>

                <div className="space-y-2.5 pt-2">
                  <div className="p-3 rounded-lg bg-zinc-900/80 border border-zinc-800 flex items-start space-x-3">
                    <CheckIcon size={16} className="text-emerald-400 shrink-0 mt-0.5" />
                    <div>
                      <span className="font-medium text-zinc-200 block">No Mandatory Accounts</span>
                      <span className="text-zinc-400 text-[11px]">No registration, passwords, cloud tokens, or sign-in walls.</span>
                    </div>
                  </div>

                  <div className="p-3 rounded-lg bg-zinc-900/80 border border-zinc-800 flex items-start space-x-3">
                    <CheckIcon size={16} className="text-emerald-400 shrink-0 mt-0.5" />
                    <div>
                      <span className="font-medium text-zinc-200 block">Zero Telemetry & Tracking</span>
                      <span className="text-zinc-400 text-[11px]">No analytics beacons, tracking pixels, or background data collection.</span>
                    </div>
                  </div>

                  <div className="p-3 rounded-lg bg-zinc-900/80 border border-zinc-800 flex items-start space-x-3">
                    <CheckIcon size={16} className="text-emerald-400 shrink-0 mt-0.5" />
                    <div>
                      <span className="font-medium text-zinc-200 block">100% Local Media Processing</span>
                      <span className="text-zinc-400 text-[11px]">All video editing, audio analysis, and rendering executes on your device.</span>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* ABOUT TAB */}
            {activeTab === "about" && (
              <div className="space-y-4">
                <div>
                  <h3 className="text-sm font-semibold text-zinc-100">OpenTake</h3>
                  <p className="text-zinc-400 text-xs mt-0.5">
                    Free AI Video Editing Studio — Automatic Silence Removal & Rough-Cut Studio
                  </p>
                </div>

                <div className="grid grid-cols-2 gap-3 font-mono text-xs">
                  <div className="p-3 rounded-lg bg-zinc-900 border border-zinc-800">
                    <span className="text-zinc-500 block mb-1">Desktop Shell</span>
                    <span className="text-zinc-200">v{appInfo?.appVersion || "0.1.0-alpha.1"}</span>
                  </div>
                  <div className="p-3 rounded-lg bg-zinc-900 border border-zinc-800">
                    <span className="text-zinc-500 block mb-1">Rust Core Engine</span>
                    <span className="text-zinc-200">v{appInfo?.coreVersion || "0.1.0-alpha.1"}</span>
                  </div>
                  <div className="p-3 rounded-lg bg-zinc-900 border border-zinc-800">
                    <span className="text-zinc-500 block mb-1">Platform</span>
                    <span className="text-zinc-200 capitalize">{appInfo?.osName || "Linux x86_64"}</span>
                  </div>
                  <div className="p-3 rounded-lg bg-zinc-900 border border-zinc-800">
                    <span className="text-zinc-500 block mb-1">Software License</span>
                    <span className="text-zinc-200">GPL-3.0-or-later (Proposed)</span>
                  </div>
                </div>

                <div className="p-3 rounded-lg bg-zinc-900/50 border border-dashed border-zinc-800 text-[11px] text-zinc-400">
                  License status: GPL-3.0-or-later is the recommended project license per ADR-0011, subject to final developer sign-off.
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="h-12 shrink-0 flex items-center justify-end px-5 border-t border-zinc-800 bg-zinc-900/40">
          <Button variant="primary" size="sm" onClick={onClose}>
            Done
          </Button>
        </div>
      </div>
    </div>
  );
}
