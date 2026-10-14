import React from "react";
import type { SubtitleCue } from "../../types/subtitles";

export interface SubtitleOverlayProps {
  activeCues: SubtitleCue[];
  isVisible: boolean;
}

/**
 * High-contrast, accessibility-aware subtitle caption overlay for video preview.
 *
 * Security & Reliability:
 * - 100% plain text React node rendering (zero dangerouslySetInnerHTML).
 * - Full Unicode and Bengali (বাংলা) multiline script support.
 * - Non-interactive pointer-events-none to avoid interfering with playback clicks.
 */
export function SubtitleOverlay({ activeCues, isVisible }: SubtitleOverlayProps): React.ReactElement | null {
  if (!isVisible || !activeCues || activeCues.length === 0) {
    return null;
  }

  return (
    <div
      aria-live="polite"
      aria-atomic="true"
      data-testid="subtitle-overlay-container"
      className="absolute bottom-10 left-0 right-0 z-25 flex flex-col items-center justify-end pointer-events-none px-6 space-y-2 select-none"
    >
      {activeCues.map((cue) => (
        <div
          key={cue.id}
          data-testid="subtitle-caption-box"
          className="bg-black/80 text-zinc-100 text-sm md:text-base font-sans font-medium px-4 py-1.5 rounded shadow-xl text-center leading-relaxed backdrop-blur-[2px] max-w-2xl whitespace-pre-line border border-zinc-700/30 tracking-wide transition-opacity duration-75"
          style={{
            textShadow: "0 1px 2px rgba(0,0,0,0.9), 0 0 4px rgba(0,0,0,0.8)",
          }}
        >
          {cue.speaker && (
            <span className="text-amber-300 font-semibold mr-1.5 text-xs tracking-normal">
              [{cue.speaker}]:
            </span>
          )}
          <span>{cue.text}</span>
        </div>
      ))}
    </div>
  );
}
