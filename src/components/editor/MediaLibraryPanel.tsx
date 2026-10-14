import React from "react";
import { FilmIcon, PlusIcon } from "../ui/Icons";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";

export function MediaLibraryPanel(): React.ReactElement {
  return (
    <section
      aria-label="Media Library"
      className="flex flex-col h-full bg-zinc-950/60 border-r border-zinc-800/80 overflow-hidden select-none"
    >
      {/* Panel Header */}
      <div className="h-8 shrink-0 flex items-center justify-between px-3 border-b border-zinc-800/70 bg-zinc-900/40">
        <div className="flex items-center space-x-2">
          <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
            Media Library
          </span>
          <span className="px-1.5 py-0.2 rounded text-[10px] font-medium bg-zinc-800 text-zinc-400">
            0 items
          </span>
        </div>

        <button
          type="button"
          disabled
          aria-disabled="true"
          title="Import disabled — Coming in Step 6"
          className="p-1 text-zinc-600 cursor-not-allowed hover:text-zinc-600 rounded"
        >
          <PlusIcon size={14} />
        </button>
      </div>

      {/* Content Area / Empty State */}
      <div className="flex-1 flex items-center justify-center p-4 overflow-y-auto">
        <EmptyState
          icon={<FilmIcon size={24} />}
          badge="Roadmap: Step 6"
          title="No Media Ingested"
          description="Local video import and FFprobe metadata extraction will be enabled in Step 6. No media files are currently loaded into this project."
          action={
            <Button
              variant="secondary"
              size="xs"
              disabled
              aria-disabled="true"
              title="Media file ingestion is scheduled for Step 6"
              icon={<PlusIcon size={12} />}
            >
              Import Media (Step 6)
            </Button>
          }
        />
      </div>
    </section>
  );
}
