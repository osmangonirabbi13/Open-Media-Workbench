import React, { useState, useEffect, useMemo, useCallback } from "react";
import type { SubtitleDocument, SubtitleTrack } from "../../types/subtitles";
import type {
  TranslationReviewDocument,
  ReviewStatus,
  Glossary,
  TranslationMemoryEntry,
} from "../../types/translationReview";
import {
  getOrCreateTranslationReview,
  updateReviewEntryTargetText,
  setReviewEntryStatus,
  batchSetReviewStatus,
  listGlossaries,
  lookupTranslationMemory,
  recordApprovedTranslation,
  computeReviewStats,
} from "../../lib/translationReview";
import {
  AlertTriangleIcon,
  SparklesIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";

interface BilingualReviewPanelProps {
  document: SubtitleDocument | null;
  onDocumentChange: (doc: SubtitleDocument | null) => void;
  currentTime: number;
  onSeek: (seconds: number) => void;
  canUndo?: boolean;
  canRedo?: boolean;
  onUndo?: () => void;
  onRedo?: () => void;
}

export function BilingualReviewPanel({
  document,
  onDocumentChange,
  currentTime,
  onSeek,
}: BilingualReviewPanelProps): React.ReactElement {
  const [sourceTrackId, setSourceTrackId] = useState<string>("");
  const [targetTrackId, setTargetTrackId] = useState<string>("");
  const [selectedGlossaryId, setSelectedGlossaryId] = useState<string>("");
  const [glossaries, setGlossaries] = useState<Glossary[]>([]);

  const [reviewDoc, setReviewDoc] = useState<TranslationReviewDocument | null>(null);
  const [selectedEntryId, setSelectedEntryId] = useState<string | null>(null);
  const [editingText, setEditingText] = useState<Record<string, string>>({});
  const [selectedEntryIds, setSelectedEntryIds] = useState<Set<string>>(new Set());

  // Filter & Search states
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("All");
  const [issueFilter, setIssueFilter] = useState<boolean>(false);

  // Translation memory suggestion for selected cue
  const [tmSuggestion, setTmSuggestion] = useState<TranslationMemoryEntry | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  // Available tracks
  const tracks: SubtitleTrack[] = useMemo(() => document?.tracks || [], [document]);

  // Load available glossaries on mount
  useEffect(() => {
    listGlossaries()
      .then((gList) => {
        setGlossaries(gList);
        if (gList.length > 0 && !selectedGlossaryId) {
          setSelectedGlossaryId(gList[0].id);
        }
      })
      .catch((err) => console.error("Failed to list glossaries:", err));
  }, []);

  // Auto-select source and target tracks if not set
  useEffect(() => {
    if (tracks.length > 0) {
      if (!sourceTrackId) {
        setSourceTrackId(tracks[0].id);
      }
      if (!targetTrackId) {
        const potentialTarget = tracks.find((t) => t.id !== tracks[0].id) || tracks[0];
        setTargetTrackId(potentialTarget.id);
      }
    }
  }, [tracks, sourceTrackId, targetTrackId]);

  // Synchronize or load review document
  const syncReview = useCallback(async () => {
    if (!document || !sourceTrackId || !targetTrackId) return;

    try {
      const doc = await getOrCreateTranslationReview(
        document,
        sourceTrackId,
        targetTrackId,
        selectedGlossaryId || undefined,
      );
      setReviewDoc(doc);
    } catch (err) {
      console.error("Failed to load review document:", err);
      setStatusMessage(`Error loading review: ${err}`);
    }
  }, [document, sourceTrackId, targetTrackId, selectedGlossaryId]);

  useEffect(() => {
    syncReview();
  }, [syncReview]);

  // When selected entry changes, query Translation Memory
  useEffect(() => {
    if (!selectedEntryId || !reviewDoc) {
      setTmSuggestion(null);
      return;
    }

    const entry = reviewDoc.entries.find((e) => e.id === selectedEntryId);
    if (!entry) {
      setTmSuggestion(null);
      return;
    }

    lookupTranslationMemory(entry.sourceText, reviewDoc.sourceLanguage, reviewDoc.targetLanguage)
      .then((res) => setTmSuggestion(res))
      .catch(() => setTmSuggestion(null));
  }, [selectedEntryId, reviewDoc]);

  // Handle in-line text change
  const handleTextChange = (entryId: string, text: string) => {
    setEditingText((prev) => ({ ...prev, [entryId]: text }));
  };

  // Save edited text
  const handleSaveText = async (entryId: string) => {
    if (!document || !reviewDoc || !targetTrackId) return;
    const newText = editingText[entryId];
    if (newText === undefined) return;

    try {
      const [updatedDoc, updatedReview] = await updateReviewEntryTargetText(
        document,
        reviewDoc,
        targetTrackId,
        entryId,
        newText,
        selectedGlossaryId || undefined,
      );

      setReviewDoc(updatedReview);
      onDocumentChange(updatedDoc);
      setStatusMessage("Saved changes. Approval reset to NeedsReview for human verification.");
    } catch (err) {
      setStatusMessage(`Failed to update text: ${err}`);
    }
  };

  // Change individual status
  const handleSetStatus = async (entryId: string, status: ReviewStatus) => {
    if (!reviewDoc) return;
    try {
      const updated = await setReviewEntryStatus(reviewDoc, entryId, status, true);
      setReviewDoc(updated);

      // If Approved, optionally record to translation memory
      if (status === "Approved") {
        const entry = updated.entries.find((e) => e.id === entryId);
        if (entry && entry.targetText) {
          recordApprovedTranslation(
            entry.sourceText,
            entry.targetText,
            updated.sourceLanguage,
            updated.targetLanguage,
          ).catch((e) => console.warn("TM auto-record failed:", e));
        }
      }
    } catch (err) {
      setStatusMessage(`Failed to set status: ${err}`);
    }
  };

  // Batch operations
  const handleBatchStatus = async (status: ReviewStatus) => {
    if (!reviewDoc || selectedEntryIds.size === 0) return;
    const ids = Array.from(selectedEntryIds);
    try {
      const updated = await batchSetReviewStatus(reviewDoc, ids, status, true);
      setReviewDoc(updated);
      setSelectedEntryIds(new Set());
      setStatusMessage(`Updated ${ids.length} cues to ${status}.`);
    } catch (err) {
      setStatusMessage(`Batch action failed: ${err}`);
    }
  };

  // Apply TM suggestion
  const handleApplyTm = async (entryId: string) => {
    if (!tmSuggestion) return;
    handleTextChange(entryId, tmSuggestion.targetText);
    setEditingText((prev) => ({ ...prev, [entryId]: tmSuggestion.targetText }));
    if (!document || !reviewDoc || !targetTrackId) return;

    try {
      const [updatedDoc, updatedReview] = await updateReviewEntryTargetText(
        document,
        reviewDoc,
        targetTrackId,
        entryId,
        tmSuggestion.targetText,
        selectedGlossaryId || undefined,
      );
      setReviewDoc(updatedReview);
      onDocumentChange(updatedDoc);
      setStatusMessage("Applied translation memory suggestion.");
    } catch (err) {
      setStatusMessage(`Failed to apply suggestion: ${err}`);
    }
  };

  // Stats
  const stats = useMemo(() => {
    if (!reviewDoc) return null;
    return computeReviewStats(reviewDoc);
  }, [reviewDoc]);

  // Filtered entries
  const filteredEntries = useMemo(() => {
    if (!reviewDoc) return [];
    return reviewDoc.entries.filter((entry) => {
      if (statusFilter !== "All" && entry.status !== statusFilter) {
        return false;
      }
      if (issueFilter && entry.findings.length === 0) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matchesSource = entry.sourceText.toLowerCase().includes(q);
        const matchesTarget = (editingText[entry.id] ?? entry.targetText).toLowerCase().includes(q);
        if (!matchesSource && !matchesTarget) return false;
      }
      return true;
    });
  }, [reviewDoc, statusFilter, issueFilter, searchQuery, editingText]);

  // Status badge styling
  const getStatusBadge = (status: ReviewStatus) => {
    switch (status) {
      case "Approved":
        return "bg-emerald-500/20 text-emerald-400 border border-emerald-500/40";
      case "Reviewed":
        return "bg-sky-500/20 text-sky-400 border border-sky-500/40";
      case "NeedsReview":
        return "bg-amber-500/20 text-amber-400 border border-amber-500/40";
      case "Rejected":
        return "bg-rose-500/20 text-rose-400 border border-rose-500/40";
      default:
        return "bg-zinc-700/50 text-zinc-400 border border-zinc-600/50";
    }
  };

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-100 overflow-hidden select-none">
      {/* Top Header: Track Selection & Glossaries */}
      <div className="p-3 border-b border-zinc-800 bg-zinc-900/60 shrink-0 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center space-x-3">
          <div>
            <label className="text-[10px] text-zinc-400 uppercase font-semibold tracking-wider block mb-1">
              Source Track (Original)
            </label>
            <select
              aria-label="Select source track"
              value={sourceTrackId}
              onChange={(e) => setSourceTrackId(e.target.value)}
              className="bg-zinc-800 border border-zinc-700 rounded text-xs px-2 py-1 text-zinc-200 focus:outline-none focus:border-indigo-500"
            >
              {tracks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.displayName} ({t.language}) [{t.cues.length} cues]
                </option>
              ))}
            </select>
          </div>

          <span className="text-zinc-600 mt-3 font-mono font-bold">→</span>

          <div>
            <label className="text-[10px] text-zinc-400 uppercase font-semibold tracking-wider block mb-1">
              Target Track (Translated)
            </label>
            <select
              aria-label="Select target track"
              value={targetTrackId}
              onChange={(e) => setTargetTrackId(e.target.value)}
              className="bg-zinc-800 border border-zinc-700 rounded text-xs px-2 py-1 text-zinc-200 focus:outline-none focus:border-indigo-500"
            >
              {tracks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.displayName} ({t.language}) [{t.cues.length} cues]
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="text-[10px] text-zinc-400 uppercase font-semibold tracking-wider block mb-1">
              Protected Glossary
            </label>
            <select
              aria-label="Select active glossary"
              value={selectedGlossaryId}
              onChange={(e) => setSelectedGlossaryId(e.target.value)}
              className="bg-zinc-800 border border-zinc-700 rounded text-xs px-2 py-1 text-zinc-200 focus:outline-none focus:border-indigo-500"
            >
              <option value="">(None / No Glossary)</option>
              {glossaries.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name} ({g.sourceLanguage} → {g.targetLanguage}) [{g.entries.length} terms]
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Sync / Refresh */}
        <div className="flex items-center space-x-2">
          <Button variant="subtle" size="sm" onClick={syncReview} className="text-xs">
            Resync Tracks
          </Button>
        </div>
      </div>

      {/* Progress & Statistics Banner */}
      {stats && (
        <div className="px-4 py-2 border-b border-zinc-800/80 bg-zinc-900/40 shrink-0 flex items-center justify-between text-xs">
          <div className="flex items-center space-x-4">
            <div>
              <span className="text-zinc-400">Total Cues: </span>
              <span className="font-semibold text-zinc-200">{stats.totalMappedCues}</span>
            </div>
            <div>
              <span className="text-emerald-400 font-medium">{stats.approvedCount} Approved</span>
              <span className="text-zinc-500 mx-1">/</span>
              <span className="text-sky-400 font-medium">{stats.reviewedCount} Reviewed</span>
              <span className="text-zinc-500 mx-1">/</span>
              <span className="text-amber-400 font-medium">{stats.needsReviewCount} Needs Review</span>
            </div>
            {stats.cuesWithWarningsCount > 0 && (
              <div className="text-amber-400 flex items-center space-x-1">
                <AlertTriangleIcon size={12} />
                <span>{stats.cuesWithWarningsCount} Warnings</span>
              </div>
            )}
            {stats.cuesWithErrorsCount > 0 && (
              <div className="text-rose-400 flex items-center space-x-1 font-semibold">
                <AlertTriangleIcon size={12} />
                <span>{stats.cuesWithErrorsCount} Errors</span>
              </div>
            )}
          </div>

          <div className="flex items-center space-x-3">
            <div className="w-32 bg-zinc-800 rounded-full h-2 overflow-hidden flex">
              <div
                className="bg-emerald-500 h-full transition-all"
                style={{ width: `${stats.approvedPercentage}%` }}
                title={`Approved: ${stats.approvedPercentage}%`}
              />
              <div
                className="bg-sky-500 h-full transition-all"
                style={{ width: `${stats.reviewedPercentage - stats.approvedPercentage}%` }}
                title={`Reviewed: ${stats.reviewedPercentage}%`}
              />
            </div>
            <span className="text-zinc-400 font-mono text-[11px]">
              {stats.approvedPercentage}% Approved
            </span>
          </div>
        </div>
      )}

      {/* Filter and Batch Bar */}
      <div className="px-3 py-2 border-b border-zinc-800 bg-zinc-900/20 shrink-0 flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center space-x-2">
          <input
            type="text"
            placeholder="Search source or target..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="bg-zinc-800/80 border border-zinc-700/80 rounded text-xs px-2.5 py-1 text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-indigo-500 w-48"
          />

          <select
            aria-label="Filter by review status"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="bg-zinc-800/80 border border-zinc-700/80 rounded text-xs px-2 py-1 text-zinc-300 focus:outline-none"
          >
            <option value="All">All Statuses</option>
            <option value="Pending">Pending</option>
            <option value="NeedsReview">Needs Review</option>
            <option value="Reviewed">Reviewed</option>
            <option value="Approved">Approved</option>
            <option value="Rejected">Rejected</option>
          </select>

          <button
            onClick={() => setIssueFilter(!issueFilter)}
            className={`px-2 py-1 rounded text-xs border transition-colors ${
              issueFilter
                ? "bg-amber-500/20 border-amber-500/40 text-amber-300"
                : "border-zinc-700 text-zinc-400 hover:text-zinc-200"
            }`}
          >
            Issues Only
          </button>
        </div>

        {/* Batch Operations */}
        {selectedEntryIds.size > 0 && (
          <div className="flex items-center space-x-2 bg-zinc-800/80 px-2 py-1 rounded border border-zinc-700">
            <span className="text-xs text-zinc-300 font-medium">
              {selectedEntryIds.size} Selected:
            </span>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => handleBatchStatus("Reviewed")}
              className="text-xs py-0.5 px-2 text-sky-400"
            >
              Mark Reviewed
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => handleBatchStatus("Approved")}
              className="text-xs py-0.5 px-2 text-emerald-400"
            >
              Approve Selected
            </Button>
            <Button
              size="sm"
              variant="subtle"
              onClick={() => setSelectedEntryIds(new Set())}
              className="text-xs py-0.5 px-1.5 text-zinc-400"
            >
              Clear
            </Button>
          </div>
        )}
      </div>

      {/* Main Bilingual Side-by-Side Table */}
      <div className="flex-1 overflow-y-auto">
        {filteredEntries.length === 0 ? (
          <div className="p-8 text-center text-zinc-500 text-xs">
            No matching bilingual subtitle cues found for review.
          </div>
        ) : (
          <div className="divide-y divide-zinc-800/60">
            {filteredEntries.map((entry, index) => {
              const isSelected = selectedEntryId === entry.id;
              const isChecked = selectedEntryIds.has(entry.id);
              const currentText = editingText[entry.id] ?? entry.targetText;
              const isDirty = currentText !== entry.targetText;

              const timingMismatch =
                entry.sourceStartUs !== entry.targetStartUs ||
                entry.sourceEndUs !== entry.targetEndUs;

              const isCurrentPlayback =
                currentTime >= entry.sourceStartUs / 1_000_000 &&
                currentTime <= entry.sourceEndUs / 1_000_000;

              return (
                <div
                  key={entry.id}
                  onClick={() => {
                    setSelectedEntryId(entry.id);
                    onSeek(entry.sourceStartUs / 1_000_000);
                  }}
                  className={`p-3 transition-colors cursor-pointer ${
                    isSelected
                      ? "bg-indigo-950/30 border-l-2 border-indigo-500"
                      : isCurrentPlayback
                      ? "bg-zinc-800/40"
                      : "hover:bg-zinc-900/50"
                  }`}
                >
                  <div className="flex items-start gap-3">
                    {/* Checkbox for batch */}
                    <input
                      type="checkbox"
                      aria-label={`Select cue ${index + 1}`}
                      checked={isChecked}
                      onChange={(e) => {
                        e.stopPropagation();
                        const next = new Set(selectedEntryIds);
                        if (e.target.checked) next.add(entry.id);
                        else next.delete(entry.id);
                        setSelectedEntryIds(next);
                      }}
                      className="mt-1 rounded border-zinc-700 bg-zinc-800 text-indigo-500 focus:ring-0"
                    />

                    {/* Cue Index & Timing */}
                    <div className="w-20 shrink-0 text-[11px] font-mono text-zinc-400">
                      <div className="font-semibold text-zinc-300">#{index + 1}</div>
                      <div className="flex items-center space-x-1 text-zinc-500 mt-1">
                        <span>⏱</span>
                        <span>{(entry.sourceStartUs / 1_000_000).toFixed(2)}s</span>
                      </div>
                      {timingMismatch && (
                        <div
                          className="text-[10px] text-amber-400 mt-0.5"
                          title="Target timing differs from source"
                        >
                          Δ timing
                        </div>
                      )}
                    </div>

                    {/* LEFT: Source Subtitle Text (Read-Only) */}
                    <div className="flex-1 bg-zinc-900/80 p-2.5 rounded border border-zinc-800/80">
                      <div className="text-[10px] text-zinc-500 uppercase font-semibold mb-1">
                        Original ({reviewDoc?.sourceLanguage.toUpperCase() || "EN"})
                      </div>
                      <div className="text-xs text-zinc-200 leading-relaxed font-sans">
                        {entry.sourceText || <span className="italic text-zinc-500">(Empty cue)</span>}
                      </div>
                    </div>

                    {/* RIGHT: Target Subtitle Text (Editable) */}
                    <div className="flex-1 bg-zinc-900/80 p-2.5 rounded border border-zinc-800/80 flex flex-col">
                      <div className="flex items-center justify-between text-[10px] text-zinc-500 uppercase font-semibold mb-1">
                        <span>Translated ({reviewDoc?.targetLanguage.toUpperCase() || "TARGET"})</span>
                        <span className={`px-1.5 py-0.5 rounded text-[10px] ${getStatusBadge(entry.status)}`}>
                          {entry.status}
                        </span>
                      </div>

                      <textarea
                        value={currentText}
                        onChange={(e) => handleTextChange(entry.id, e.target.value)}
                        onClick={(e) => e.stopPropagation()}
                        rows={2}
                        className="w-full bg-zinc-950/80 border border-zinc-700/80 rounded p-1.5 text-xs text-zinc-100 font-sans focus:outline-none focus:border-indigo-500 resize-y"
                        placeholder="Target translation..."
                      />

                      {/* Action buttons under target text */}
                      <div className="mt-2 flex items-center justify-between">
                        <div className="flex items-center space-x-1.5">
                          {isDirty && (
                            <Button
                              size="sm"
                              variant="primary"
                              onClick={(e) => {
                                e.stopPropagation();
                                handleSaveText(entry.id);
                              }}
                              className="text-[11px] py-0.5 px-2"
                            >
                              Save Edit
                            </Button>
                          )}

                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleSetStatus(entry.id, "Reviewed");
                            }}
                            className="px-2 py-0.5 text-[11px] rounded bg-zinc-800 text-sky-400 hover:bg-zinc-700"
                          >
                            Mark Reviewed
                          </button>

                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleSetStatus(entry.id, "Approved");
                            }}
                            className="px-2 py-0.5 text-[11px] rounded bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30 font-medium"
                          >
                            Approve
                          </button>

                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              handleSetStatus(entry.id, "Rejected");
                            }}
                            className="px-2 py-0.5 text-[11px] rounded bg-zinc-800 text-rose-400 hover:bg-zinc-700"
                          >
                            Reject
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Diagnostic Quality Findings Warnings */}
                  {entry.findings.length > 0 && (
                    <div className="mt-2 ml-24 space-y-1">
                      {entry.findings.map((f, fIdx) => (
                        <div
                          key={fIdx}
                          className={`text-[11px] px-2 py-1 rounded flex items-center space-x-1.5 ${
                            f.severity === "Error"
                              ? "bg-rose-950/40 border border-rose-900/60 text-rose-300"
                              : "bg-amber-950/40 border border-amber-900/60 text-amber-300"
                          }`}
                        >
                          <AlertTriangleIcon size={12} className="shrink-0" />
                          <span className="font-medium">[{f.code}]</span>
                          <span>{f.message}</span>
                        </div>
                      ))}
                    </div>
                  )}

                  {/* Translation Memory suggestion box for selected row */}
                  {isSelected && tmSuggestion && tmSuggestion.targetText !== currentText && (
                    <div className="mt-2 ml-24 p-2 bg-indigo-950/30 border border-indigo-500/40 rounded flex items-center justify-between text-xs">
                      <div className="flex items-center space-x-2">
                        <SparklesIcon size={14} className="text-indigo-400 shrink-0" />
                        <div>
                          <div className="text-[10px] text-indigo-400 font-semibold uppercase">
                            Translation Memory Suggestion (Used {tmSuggestion.usageCount}x):
                          </div>
                          <div className="text-zinc-200">{tmSuggestion.targetText}</div>
                        </div>
                      </div>

                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleApplyTm(entry.id);
                        }}
                        className="text-[11px] py-1 px-2.5 text-indigo-300"
                      >
                        Apply Suggestion
                      </Button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Footer Status Bar */}
      {statusMessage && (
        <div className="p-2 px-4 bg-zinc-900 border-t border-zinc-800 text-xs text-zinc-300 flex items-center justify-between">
          <span>{statusMessage}</span>
          <button
            onClick={() => setStatusMessage(null)}
            className="text-zinc-500 hover:text-zinc-300 ml-4"
          >
            ✕
          </button>
        </div>
      )}
    </div>
  );
}
