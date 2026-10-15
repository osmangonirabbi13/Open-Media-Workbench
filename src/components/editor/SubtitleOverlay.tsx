import React from "react";
import type { SubtitleCue } from "../../types/subtitles";
import type { SubtitleStyle } from "../../types/styling";
import { colorRgbaToCss, DEFAULT_SUBTITLE_STYLE } from "../../lib/styling";

export interface SubtitleOverlayProps {
  activeCues: SubtitleCue[];
  isVisible: boolean;
  currentTimeUs?: number;
  enableKaraoke?: boolean;
  style?: SubtitleStyle;
}

/**
 * High-precision, accessibility-aware subtitle caption overlay for video preview
 * with full SubtitleStyle typography, colors, outline, shadow, background box,
 * positioning, and word-level live timing highlighting (Karaoke foundation).
 *
 * Security & Parity:
 * - 100% plain text React node rendering (zero dangerouslySetInnerHTML).
 * - Full Unicode and Bengali (বাংলা) multiline script support.
 * - Non-interactive pointer-events-none to avoid interfering with playback clicks.
 * - Matches canonical SubtitleStyle properties rendered by FFmpeg libass.
 */
export function SubtitleOverlay({
  activeCues,
  isVisible,
  currentTimeUs,
  enableKaraoke = true,
  style = DEFAULT_SUBTITLE_STYLE,
}: SubtitleOverlayProps): React.ReactElement | null {
  if (!isVisible || !activeCues || activeCues.length === 0) {
    return null;
  }

  // Derive positioning classes & inline styles
  let containerPositionClasses = "bottom-8 left-0 right-0 items-center justify-end px-6";
  let customPositionStyle: React.CSSProperties = {};

  switch (style.position.type) {
    case "bottomCenter":
      containerPositionClasses = "bottom-8 left-0 right-0 items-center justify-end px-6";
      break;
    case "bottomLeft":
      containerPositionClasses = "bottom-8 left-8 right-auto items-start justify-end";
      break;
    case "bottomRight":
      containerPositionClasses = "bottom-8 right-8 left-auto items-end justify-end";
      break;
    case "center":
      containerPositionClasses = "inset-0 items-center justify-center px-6";
      break;
    case "topCenter":
      containerPositionClasses = "top-8 left-0 right-0 items-center justify-start px-6";
      break;
    case "custom":
      containerPositionClasses = "items-center justify-center";
      customPositionStyle = {
        left: `${style.position.xPct}%`,
        top: `${style.position.yPct}%`,
        transform: "translate(-50%, -50%)",
        position: "absolute",
      };
      break;
  }

  // Build outline and shadow text-shadow string
  const ow = style.outlineWidth;
  const outlineCol = colorRgbaToCss(style.outlineColor);
  const so = style.shadowOffset;
  const shadowCol = colorRgbaToCss(style.shadowColor);

  const shadowParts: string[] = [];
  if (ow > 0) {
    shadowParts.push(
      `-${ow}px -${ow}px 0 ${outlineCol}`,
      `${ow}px -${ow}px 0 ${outlineCol}`,
      `-${ow}px ${ow}px 0 ${outlineCol}`,
      `${ow}px ${ow}px 0 ${outlineCol}`,
      `0px -${ow}px 0 ${outlineCol}`,
      `0px ${ow}px 0 ${outlineCol}`,
      `-${ow}px 0px 0 ${outlineCol}`,
      `${ow}px 0px 0 ${outlineCol}`
    );
  }
  if (so > 0) {
    shadowParts.push(`${so}px ${so}px ${Math.max(so, 2)}px ${shadowCol}`);
  }
  const textShadowCss = shadowParts.join(", ");

  // Background styling
  const bgCss =
    style.backgroundOpacity > 0
      ? colorRgbaToCss({
          ...style.backgroundColor,
          a: Math.round(style.backgroundOpacity * 255),
        })
      : "transparent";

  const highlightCss = colorRgbaToCss(style.wordHighlightColor);

  return (
    <div
      aria-live="polite"
      aria-atomic="true"
      data-testid="subtitle-overlay-container"
      className={`absolute z-25 flex flex-col pointer-events-none select-none space-y-2 ${containerPositionClasses}`}
      style={customPositionStyle}
    >
      {activeCues.map((cue) => {
        const hasWords = cue.words && cue.words.length > 0;
        const showKaraoke =
          hasWords &&
          (enableKaraoke || style.animation === "karaokeHighlight") &&
          currentTimeUs !== undefined;

        const isPop = style.animation === "pop";
        const isFade = style.animation === "fade";

        return (
          <div
            key={cue.id}
            data-testid="subtitle-caption-box"
            className={`max-w-2xl whitespace-pre-line rounded leading-relaxed tracking-wide ${
              isPop ? "animate-in zoom-in-90 duration-150" : ""
            } ${isFade ? "transition-opacity duration-200" : ""}`}
            style={{
              color: colorRgbaToCss(style.textColor),
              fontFamily: `${style.fontFamily}, sans-serif`,
              fontSize: `${style.fontSize}px`,
              fontWeight: style.fontWeight,
              fontStyle: style.italic ? "italic" : "normal",
              textAlign: style.textAlignment,
              backgroundColor: bgCss,
              padding: `${style.backgroundPadding}px ${style.backgroundPadding * 1.5}px`,
              textShadow: textShadowCss,
              lineHeight: style.lineSpacing,
            }}
          >
            {cue.speaker && (
              <span
                className="font-semibold mr-1.5 text-xs opacity-90 inline-block"
                style={{
                  color: highlightCss,
                  textShadow: textShadowCss,
                }}
              >
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
                      className="inline transition-colors duration-75"
                      style={
                        isCurrent
                          ? {
                              color: highlightCss,
                              fontWeight: Math.min(style.fontWeight + 200, 900),
                              textShadow: `${textShadowCss}, 0 0 8px ${highlightCss}`,
                            }
                          : isPast
                          ? {
                              color: colorRgbaToCss(style.textColor),
                            }
                          : {
                              color: colorRgbaToCss({
                                ...style.textColor,
                                a: Math.round(style.textColor.a * 0.75),
                              }),
                            }
                      }
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
