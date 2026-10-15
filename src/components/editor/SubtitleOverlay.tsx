import React from "react";
import type { SubtitleCue } from "../../types/subtitles";

export interface SubtitleOverlayProps {
  activeCues: SubtitleCue[];
  isVisible: boolean;
  currentTimeUs?: number;
  enableKaraoke?: boolean;
}

/**
 * High-contrast, accessibility-aware subtitle caption overlay for video preview
 * with word-level live timing highlighting (Karaoke foundation).
 *
 * Security & Reliability:
 * - 100% plain text React node rendering (zero dangerouslySetInnerHTML).
 * - Full Unicode and Bengali (বাংলা) multiline script support.
 * - Non-interactive pointer-events-none to avoid interfering with playback clicks.
 * - Graceful fallback to whole-cue text if word timing is absent or karaoke disabled.
 */
export function SubtitleOverlay({
  activeCues,
  isVisible,
  currentTimeUs,
  enableKaraoke = true,
}: SubtitleOverlayProps): React.ReactElement | null {
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
      {activeCues.map((cue) => {
        const hasWords = cue.words && cue.words.length > 0;
        const showKaraoke = hasWords && enableKaraoke && currentTimeUs !== undefined;

        return (
          <div
            key={cue.id}
            data-testid="subtitle-caption-box"
            className="bg-black/85 text-zinc-100 text-sm md:text-base font-sans font-medium px-4 py-1.5 rounded shadow-xl text-center leading-relaxed backdrop-blur-[2px] max-w-2xl whitespace-pre-line border border-zinc-700/40 tracking-wide transition-opacity duration-75"
            style={{
              textShadow: "0 1px 2px rgba(0,0,0,0.9), 0 0 4px rgba(0,0,0,0.8)",
            }}
          >
            {cue.speaker && (
              <span className="text-amber-300 font-semibold mr-1.5 text-xs tracking-normal">
                [{cue.speaker}]:
              </span>
            )}

            {showKaraoke ? (
              <span className="inline">
                {cue.words!.map((w, wIdx) => {
                  const isCurrent =
                    currentTimeUs >= w.startUs && currentTimeUs < w.endUs;
                  const isPast = currentTimeUs >= w.endUs;

                  return (
                    <span
                      key={w.id || `w-${wIdx}`}
                      data-testid={`karaoke-word-${w.id}`}
                      className={`inline transition-colors duration-75 ${
                        isCurrent
                          ? "text-amber-400 font-bold bg-amber-950/60 px-1 py-0.5 rounded shadow-sm ring-1 ring-amber-500/30"
                          : isPast
                          ? "text-zinc-100"
                          : "text-zinc-400/90"
                      }`}
                    >
                      {w.punctuationPrefix ?? ""}
                      {w.text}
                      {w.punctuationSuffix ?? ""}
                      {wIdx < cue.words!.length - 1 ? " " : ""}
                    </span>
                  );
                })}
              </span>
            ) : (
              <span>{cue.text}</span>
            )}
          </div>
        );
      })}
    </div>
  );
}
