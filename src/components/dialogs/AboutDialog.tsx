import React, { useEffect, useRef } from "react";
import { CloseIcon, OpenTakeLogo } from "../ui/Icons";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import type { AppInfo } from "../../types/ipc";

export interface AboutDialogProps {
  isOpen: boolean;
  onClose: () => void;
  appInfo?: AppInfo;
}

export function AboutDialog({
  isOpen,
  onClose,
  appInfo,
}: AboutDialogProps): React.ReactElement | null {
  const dialogRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (isOpen) {
      previousFocusRef.current = document.activeElement as HTMLElement | null;
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
        previousFocusRef.current?.focus();
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
      aria-labelledby="about-dialog-title"
    >
      <div
        ref={dialogRef}
        tabIndex={-1}
        className="w-full max-w-md rounded-xl border border-zinc-800 bg-zinc-950 shadow-2xl flex flex-col overflow-hidden focus:outline-none"
      >
        {/* Header */}
        <div className="h-12 shrink-0 flex items-center justify-between px-5 border-b border-zinc-800 bg-zinc-900/60">
          <div className="flex items-center space-x-2.5">
            <OpenTakeLogo size={20} />
            <h2
              id="about-dialog-title"
              className="text-sm font-semibold text-zinc-100 tracking-tight"
            >
              About OpenTake
            </h2>
          </div>
          <IconButton
            icon={<CloseIcon size={14} />}
            label="Close About Dialog (Escape)"
            size="sm"
            variant="subtle"
            onClick={onClose}
          />
        </div>

        {/* Content */}
        <div className="p-6 space-y-5 text-xs text-zinc-300">
          <div>
            <h3 className="text-base font-bold text-zinc-100 tracking-tight">OpenTake</h3>
            <p className="text-zinc-400 mt-1 leading-relaxed">
              Free AI Video Editing Studio — Automatic Silence Removal & Rough-Cut Desktop Studio.
            </p>
          </div>

          <div className="rounded-lg border border-zinc-800 bg-zinc-900/60 p-3.5 space-y-2.5 font-mono text-[11px]">
            <div className="flex justify-between items-center">
              <span className="text-zinc-500">Desktop Shell Version</span>
              <span className="text-zinc-200">v{appInfo?.appVersion || "0.1.0-alpha.1"}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-zinc-500">Rust Core Version</span>
              <span className="text-zinc-200">v{appInfo?.coreVersion || "0.1.0-alpha.1"}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-zinc-500">Host Operating System</span>
              <span className="text-zinc-200 capitalize">{appInfo?.osName || "Linux x86_64"}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-zinc-500">Software License</span>
              <span className="text-zinc-200">GPL-3.0-or-later (Proposed)</span>
            </div>
          </div>

          <div className="space-y-1.5 text-zinc-400 leading-relaxed text-[11px]">
            <p>
              Built with Tauri 2, Rust, React 19, TypeScript, and Tailwind CSS. Dedicated to open-source, local-first creators worldwide.
            </p>
            <p className="text-zinc-500">
              Current Roadmap: Milestone 1 (Foundation) • Step 4 of 32
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="h-12 shrink-0 flex items-center justify-end px-5 border-t border-zinc-800 bg-zinc-900/40">
          <Button variant="primary" size="sm" onClick={onClose}>
            Close
          </Button>
        </div>
      </div>
    </div>
  );
}
