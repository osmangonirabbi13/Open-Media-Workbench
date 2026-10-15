import React, { useState, useEffect } from "react";
import type {
  Glossary,
  GlossaryEntry,
  TranslationMemory,
} from "../../types/translationReview";
import {
  listGlossaries,
  saveGlossary,
  deleteGlossary,
  getTranslationMemory,
  deleteTranslationMemoryEntry,
  clearTranslationMemory,
} from "../../lib/translationReview";
import {
  ShieldCheckIcon,
  SparklesIcon,
  TrashIcon,
} from "../../components/ui/Icons";
import { Button } from "../../components/ui/Button";

export function GlossaryManagerPanel(): React.ReactElement {
  const [activeTab, setActiveTab] = useState<"glossaries" | "memory">("glossaries");

  // Glossaries state
  const [glossaries, setGlossaries] = useState<Glossary[]>([]);
  const [selectedGlossaryId, setSelectedGlossaryId] = useState<string | null>(null);

  // New glossary form
  const [showNewGlossaryModal, setShowNewGlossaryModal] = useState(false);
  const [newGlossaryName, setNewGlossaryName] = useState("");
  const [newSourceLang, setNewSourceLang] = useState("en");
  const [newTargetLang, setNewTargetLang] = useState("es");
  const [newIsGlobal, setNewIsGlobal] = useState(true);

  // New term entry form
  const [newSourceTerm, setNewSourceTerm] = useState("");
  const [newPreferredTerm, setNewPreferredTerm] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [newIsProtected, setNewIsProtected] = useState(true);
  const [newCaseSensitive, setNewCaseSensitive] = useState(false);
  const [newDomain] = useState("General");

  // Search
  const [termSearchQuery, setTermSearchQuery] = useState("");

  // Translation Memory state
  const [memory, setMemory] = useState<TranslationMemory | null>(null);
  const [memorySearchQuery, setMemorySearchQuery] = useState("");

  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);

  // Load glossaries
  const refreshGlossaries = () => {
    listGlossaries()
      .then((list) => {
        setGlossaries(list);
        if (list.length > 0 && !selectedGlossaryId) {
          setSelectedGlossaryId(list[0].id);
        }
      })
      .catch((e) => console.error("Error loading glossaries:", e));
  };

  // Load memory
  const refreshMemory = () => {
    getTranslationMemory()
      .then((mem) => setMemory(mem))
      .catch((e) => console.error("Error loading TM:", e));
  };

  useEffect(() => {
    refreshGlossaries();
    refreshMemory();
  }, []);

  const selectedGlossary = glossaries.find((g) => g.id === selectedGlossaryId) || null;

  // Create Glossary
  const handleCreateGlossary = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newGlossaryName.trim()) return;

    const newG: Glossary = {
      schemaVersion: 1,
      id: `glos-${Date.now()}`,
      name: newGlossaryName.trim(),
      sourceLanguage: newSourceLang.trim().toLowerCase(),
      targetLanguage: newTargetLang.trim().toLowerCase(),
      isGlobal: newIsGlobal,
      entries: [
        {
          id: `term-${Date.now()}-1`,
          sourceTerm: "Open Media Workbench",
          preferredTerm: "Open Media Workbench",
          description: "Product proper noun brand name",
          isProtected: true,
          caseSensitive: false,
          domain: "Product",
          createdAtEpochMs: Date.now(),
          updatedAtEpochMs: Date.now(),
        },
      ],
      createdAtEpochMs: Date.now(),
      updatedAtEpochMs: Date.now(),
    };

    try {
      await saveGlossary(newG);
      setNewGlossaryName("");
      setShowNewGlossaryModal(false);
      refreshGlossaries();
      setSelectedGlossaryId(newG.id);
      setFeedbackMessage(`Created glossary "${newG.name}".`);
    } catch (err) {
      setFeedbackMessage(`Failed to create glossary: ${err}`);
    }
  };

  // Delete Glossary
  const handleDeleteGlossary = async (id: string) => {
    if (!confirm("Are you sure you want to delete this glossary?")) return;
    try {
      await deleteGlossary(id);
      refreshGlossaries();
      if (selectedGlossaryId === id) setSelectedGlossaryId(null);
      setFeedbackMessage("Glossary deleted.");
    } catch (err) {
      setFeedbackMessage(`Failed to delete glossary: ${err}`);
    }
  };

  // Add Term Entry
  const handleAddTerm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedGlossary || !newSourceTerm.trim()) return;

    const newEntry: GlossaryEntry = {
      id: `term-${Date.now()}`,
      sourceTerm: newSourceTerm.trim(),
      preferredTerm: newPreferredTerm.trim() || newSourceTerm.trim(),
      description: newDescription.trim() || undefined,
      isProtected: newIsProtected,
      caseSensitive: newCaseSensitive,
      domain: newDomain.trim() || undefined,
      createdAtEpochMs: Date.now(),
      updatedAtEpochMs: Date.now(),
    };

    const updatedGlossary: Glossary = {
      ...selectedGlossary,
      entries: [...selectedGlossary.entries, newEntry],
      updatedAtEpochMs: Date.now(),
    };

    try {
      await saveGlossary(updatedGlossary);
      setNewSourceTerm("");
      setNewPreferredTerm("");
      setNewDescription("");
      refreshGlossaries();
      setFeedbackMessage(`Added terminology rule for "${newEntry.sourceTerm}".`);
    } catch (err) {
      setFeedbackMessage(`Failed to save entry: ${err}`);
    }
  };

  // Delete Term Entry
  const handleDeleteTerm = async (entryId: string) => {
    if (!selectedGlossary) return;
    const updated: Glossary = {
      ...selectedGlossary,
      entries: selectedGlossary.entries.filter((e) => e.id !== entryId),
      updatedAtEpochMs: Date.now(),
    };
    try {
      await saveGlossary(updated);
      refreshGlossaries();
      setFeedbackMessage("Removed terminology entry.");
    } catch (err) {
      setFeedbackMessage(`Failed to remove entry: ${err}`);
    }
  };

  // Delete TM entry
  const handleDeleteTmEntry = async (entryId: string) => {
    try {
      await deleteTranslationMemoryEntry(entryId);
      refreshMemory();
      setFeedbackMessage("Deleted memory entry.");
    } catch (err) {
      setFeedbackMessage(`Failed to delete TM entry: ${err}`);
    }
  };

  // Clear TM
  const handleClearTm = async () => {
    if (!confirm("Are you sure you want to clear all translation memory? This cannot be undone.")) return;
    try {
      await clearTranslationMemory();
      refreshMemory();
      setFeedbackMessage("Cleared translation memory.");
    } catch (err) {
      setFeedbackMessage(`Failed to clear TM: ${err}`);
    }
  };

  // Filtered terms
  const filteredTerms = (selectedGlossary?.entries || []).filter((e) => {
    if (!termSearchQuery.trim()) return true;
    const q = termSearchQuery.toLowerCase();
    return (
      e.sourceTerm.toLowerCase().includes(q) ||
      e.preferredTerm.toLowerCase().includes(q) ||
      (e.description && e.description.toLowerCase().includes(q))
    );
  });

  // Filtered memory entries
  const filteredMemoryEntries = (memory?.entries || []).filter((e) => {
    if (!memorySearchQuery.trim()) return true;
    const q = memorySearchQuery.toLowerCase();
    return (
      e.sourceTextNormalized.includes(q) ||
      e.targetText.toLowerCase().includes(q)
    );
  });

  return (
    <div className="flex flex-col h-full bg-zinc-950 text-zinc-100 overflow-hidden select-none">
      {/* Top Navigation Tabs */}
      <div className="flex items-center space-x-2 p-2 border-b border-zinc-800 bg-zinc-900/60 shrink-0">
        <button
          onClick={() => setActiveTab("glossaries")}
          className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
            activeTab === "glossaries"
              ? "bg-zinc-800 text-zinc-100 shadow-xs"
              : "text-zinc-400 hover:text-zinc-200"
          }`}
        >
          <ShieldCheckIcon size={14} className={activeTab === "glossaries" ? "text-indigo-400" : ""} />
          <span>Terminology Glossaries</span>
        </button>

        <button
          onClick={() => setActiveTab("memory")}
          className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
            activeTab === "memory"
              ? "bg-zinc-800 text-zinc-100 shadow-xs"
              : "text-zinc-400 hover:text-zinc-200"
          }`}
        >
          <SparklesIcon size={14} className={activeTab === "memory" ? "text-indigo-400" : ""} />
          <span>Translation Memory ({memory?.entries.length || 0})</span>
        </button>
      </div>

      {feedbackMessage && (
        <div className="px-4 py-1.5 bg-indigo-950/40 border-b border-indigo-800/40 text-xs text-indigo-300 flex items-center justify-between">
          <span>{feedbackMessage}</span>
          <button onClick={() => setFeedbackMessage(null)} className="text-zinc-500 hover:text-zinc-300">
            ✕
          </button>
        </div>
      )}

      {activeTab === "glossaries" ? (
        <div className="flex-1 flex overflow-hidden">
          {/* Left Sidebar: Glossaries List */}
          <div className="w-64 border-r border-zinc-800 bg-zinc-900/30 flex flex-col shrink-0">
            <div className="p-3 border-b border-zinc-800 flex items-center justify-between">
              <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
                Glossaries ({glossaries.length})
              </span>
              <Button
                size="sm"
                variant="primary"
                onClick={() => setShowNewGlossaryModal(true)}
                className="text-xs py-0.5 px-2"
              >
                + New
              </Button>
            </div>

            <div className="flex-1 overflow-y-auto divide-y divide-zinc-800/50">
              {glossaries.length === 0 ? (
                <div className="p-4 text-center text-xs text-zinc-500">
                  No glossaries found. Create one to protect brand names and proper nouns.
                </div>
              ) : (
                glossaries.map((g) => {
                  const isSelected = selectedGlossaryId === g.id;
                  return (
                    <div
                      key={g.id}
                      onClick={() => setSelectedGlossaryId(g.id)}
                      className={`p-3 cursor-pointer transition-colors ${
                        isSelected
                          ? "bg-indigo-950/30 border-l-2 border-indigo-500"
                          : "hover:bg-zinc-800/30"
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-medium text-zinc-200 truncate">{g.name}</span>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDeleteGlossary(g.id);
                          }}
                          className="text-zinc-500 hover:text-rose-400 p-0.5"
                          title="Delete glossary"
                        >
                          <TrashIcon size={12} />
                        </button>
                      </div>
                      <div className="text-[10px] text-zinc-500 mt-1 flex items-center justify-between">
                        <span>
                          {g.sourceLanguage.toUpperCase()} → {g.targetLanguage.toUpperCase()}
                        </span>
                        <span>{g.entries.length} terms</span>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Right Main Pane: Terminology Entries in Selected Glossary */}
          <div className="flex-1 flex flex-col overflow-hidden">
            {selectedGlossary ? (
              <>
                {/* Header info */}
                <div className="p-3 border-b border-zinc-800 bg-zinc-900/20 flex items-center justify-between">
                  <div>
                    <h3 className="text-sm font-semibold text-zinc-200">{selectedGlossary.name}</h3>
                    <div className="text-[11px] text-zinc-400 mt-0.5">
                      Language Pair: {selectedGlossary.sourceLanguage} → {selectedGlossary.targetLanguage} •{" "}
                      {selectedGlossary.isGlobal ? "Global Library" : "Project-Scoped"}
                    </div>
                  </div>

                  <input
                    type="text"
                    placeholder="Search terminology..."
                    value={termSearchQuery}
                    onChange={(e) => setTermSearchQuery(e.target.value)}
                    className="bg-zinc-800 border border-zinc-700 rounded text-xs px-2.5 py-1 text-zinc-200 placeholder-zinc-500 focus:outline-none w-48"
                  />
                </div>

                {/* Add New Term Inline Form */}
                <form
                  onSubmit={handleAddTerm}
                  className="p-3 border-b border-zinc-800 bg-zinc-900/40 grid grid-cols-1 md:grid-cols-4 gap-2 items-end text-xs"
                >
                  <div>
                    <label className="text-[10px] text-zinc-400 uppercase font-semibold block mb-1">
                      Source Term (e.g. Open Media Workbench)
                    </label>
                    <input
                      type="text"
                      required
                      placeholder="Brand or source phrase"
                      value={newSourceTerm}
                      onChange={(e) => setNewSourceTerm(e.target.value)}
                      className="w-full bg-zinc-800 border border-zinc-700 rounded p-1.5 text-zinc-200 focus:outline-none focus:border-indigo-500 text-xs"
                    />
                  </div>

                  <div>
                    <label className="text-[10px] text-zinc-400 uppercase font-semibold block mb-1">
                      Preferred Target Translation
                    </label>
                    <input
                      type="text"
                      placeholder="Preserved or translated term"
                      value={newPreferredTerm}
                      onChange={(e) => setNewPreferredTerm(e.target.value)}
                      className="w-full bg-zinc-800 border border-zinc-700 rounded p-1.5 text-zinc-200 focus:outline-none focus:border-indigo-500 text-xs"
                    />
                  </div>

                  <div className="flex items-center space-x-3 pt-3">
                    <label className="flex items-center space-x-1.5 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={newIsProtected}
                        onChange={(e) => setNewIsProtected(e.target.checked)}
                        className="rounded border-zinc-700 bg-zinc-800 text-indigo-500"
                      />
                      <span className="text-[11px] text-zinc-300 font-medium">Protect Brand</span>
                    </label>

                    <label className="flex items-center space-x-1.5 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={newCaseSensitive}
                        onChange={(e) => setNewCaseSensitive(e.target.checked)}
                        className="rounded border-zinc-700 bg-zinc-800 text-indigo-500"
                      />
                      <span className="text-[11px] text-zinc-300">Case Sensitive</span>
                    </label>
                  </div>

                  <div>
                    <Button type="submit" variant="primary" size="sm" className="w-full text-xs py-1.5">
                      + Add Term Rule
                    </Button>
                  </div>
                </form>

                {/* Terms Table */}
                <div className="flex-1 overflow-y-auto">
                  <table className="w-full text-left text-xs border-collapse">
                    <thead>
                      <tr className="border-b border-zinc-800 bg-zinc-900/60 text-[10px] text-zinc-400 uppercase">
                        <th className="p-2.5 pl-4">Source Term</th>
                        <th className="p-2.5">Target Term</th>
                        <th className="p-2.5">Protection</th>
                        <th className="p-2.5">Match Type</th>
                        <th className="p-2.5 pr-4 text-right">Actions</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-zinc-800/40">
                      {filteredTerms.length === 0 ? (
                        <tr>
                          <td colSpan={5} className="p-6 text-center text-zinc-500">
                            No terminology entries found.
                          </td>
                        </tr>
                      ) : (
                        filteredTerms.map((entry) => (
                          <tr key={entry.id} className="hover:bg-zinc-900/30">
                            <td className="p-2.5 pl-4 font-medium text-zinc-200 font-sans">
                              {entry.sourceTerm}
                            </td>
                            <td className="p-2.5 text-zinc-300 font-sans">{entry.preferredTerm}</td>
                            <td className="p-2.5">
                              {entry.isProtected ? (
                                <span className="inline-flex items-center space-x-1 px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/20 text-emerald-400 border border-emerald-500/30">
                                  <ShieldCheckIcon size={10} />
                                  <span>Protected</span>
                                </span>
                              ) : (
                                <span className="text-[10px] text-zinc-500">Preferred</span>
                              )}
                            </td>
                            <td className="p-2.5 text-[11px] text-zinc-400">
                              {entry.caseSensitive ? "Exact Case" : "Case Insensitive"} (Whole Word)
                            </td>
                            <td className="p-2.5 pr-4 text-right">
                              <button
                                onClick={() => handleDeleteTerm(entry.id)}
                                className="text-zinc-500 hover:text-rose-400 p-1"
                                title="Delete terminology rule"
                              >
                                <TrashIcon size={12} />
                              </button>
                            </td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </>
            ) : (
              <div className="p-12 text-center text-xs text-zinc-500">
                Select or create a terminology glossary to manage rules.
              </div>
            )}
          </div>
        </div>
      ) : (
        /* Translation Memory Tab */
        <div className="flex-1 flex flex-col overflow-hidden p-4">
          <div className="flex items-center justify-between pb-3 border-b border-zinc-800">
            <div>
              <h3 className="text-sm font-semibold text-zinc-200">Local Translation Memory</h3>
              <p className="text-xs text-zinc-400 mt-0.5">
                Previously approved subtitle translations stored locally for reuse.
              </p>
            </div>

            <div className="flex items-center space-x-3">
              <input
                type="text"
                placeholder="Search TM..."
                value={memorySearchQuery}
                onChange={(e) => setMemorySearchQuery(e.target.value)}
                className="bg-zinc-800 border border-zinc-700 rounded text-xs px-2.5 py-1 text-zinc-200 placeholder-zinc-500 focus:outline-none w-48"
              />

              <Button
                variant="danger"
                size="sm"
                onClick={handleClearTm}
                className="text-xs py-1 px-2.5"
              >
                Clear Memory
              </Button>
            </div>
          </div>

          {/* Privacy Notice */}
          <div className="mt-3 p-2.5 bg-zinc-900/60 border border-zinc-800 rounded text-xs text-zinc-400 flex items-start space-x-2">
            <ShieldCheckIcon size={14} className="text-emerald-400 shrink-0 mt-0.5" />
            <div>
              <span className="font-semibold text-zinc-300">Privacy & Isolation:</span> Translation memory is stored 100% locally on your machine in <code className="text-zinc-300 font-mono text-[11px]">.local/share/opentake/review/</code>. No external cloud services or telemetry APIs are ever contacted.
            </div>
          </div>

          {/* Memory Entries Table */}
          <div className="flex-1 overflow-y-auto mt-3 border border-zinc-800 rounded bg-zinc-900/20">
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-zinc-800 bg-zinc-900/60 text-[10px] text-zinc-400 uppercase">
                  <th className="p-2.5 pl-4">Source Subtitle</th>
                  <th className="p-2.5">Approved Translation</th>
                  <th className="p-2.5">Language</th>
                  <th className="p-2.5">Usage</th>
                  <th className="p-2.5 pr-4 text-right">Delete</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-800/40">
                {filteredMemoryEntries.length === 0 ? (
                  <tr>
                    <td colSpan={5} className="p-8 text-center text-zinc-500">
                      No translation memory entries stored yet. When you approve subtitle cues during review, they are automatically saved here for instant reuse.
                    </td>
                  </tr>
                ) : (
                  filteredMemoryEntries.map((m) => (
                    <tr key={m.id} className="hover:bg-zinc-900/30">
                      <td className="p-2.5 pl-4 text-zinc-200 font-sans">{m.sourceTextNormalized}</td>
                      <td className="p-2.5 text-emerald-400 font-sans font-medium">{m.targetText}</td>
                      <td className="p-2.5 text-[11px] text-zinc-400">
                        {m.sourceLanguage.toUpperCase()} → {m.targetLanguage.toUpperCase()}
                      </td>
                      <td className="p-2.5 text-zinc-400 font-mono text-[11px]">{m.usageCount}x</td>
                      <td className="p-2.5 pr-4 text-right">
                        <button
                          onClick={() => handleDeleteTmEntry(m.id)}
                          className="text-zinc-500 hover:text-rose-400 p-1"
                          title="Delete memory entry"
                        >
                          <TrashIcon size={12} />
                        </button>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Modal for Creating New Glossary */}
      {showNewGlossaryModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center z-50 p-4">
          <div className="bg-zinc-900 border border-zinc-700 rounded-lg p-5 w-full max-w-md shadow-xl text-xs space-y-4">
            <h3 className="text-sm font-semibold text-zinc-100">Create Terminology Glossary</h3>

            <div>
              <label className="text-[11px] text-zinc-400 uppercase font-semibold block mb-1">
                Glossary Display Name
              </label>
              <input
                type="text"
                placeholder="e.g. Open Media Workbench Brand Terms"
                value={newGlossaryName}
                onChange={(e) => setNewGlossaryName(e.target.value)}
                className="w-full bg-zinc-800 border border-zinc-700 rounded p-2 text-zinc-200 focus:outline-none focus:border-indigo-500 text-xs"
              />
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="text-[11px] text-zinc-400 uppercase font-semibold block mb-1">
                  Source Language
                </label>
                <input
                  type="text"
                  value={newSourceLang}
                  onChange={(e) => setNewSourceLang(e.target.value)}
                  className="w-full bg-zinc-800 border border-zinc-700 rounded p-2 text-zinc-200 focus:outline-none text-xs"
                />
              </div>

              <div>
                <label className="text-[11px] text-zinc-400 uppercase font-semibold block mb-1">
                  Target Language
                </label>
                <input
                  type="text"
                  value={newTargetLang}
                  onChange={(e) => setNewTargetLang(e.target.value)}
                  className="w-full bg-zinc-800 border border-zinc-700 rounded p-2 text-zinc-200 focus:outline-none text-xs"
                />
              </div>
            </div>

            <div className="flex items-center space-x-2 pt-2">
              <input
                type="checkbox"
                id="global-glossary"
                checked={newIsGlobal}
                onChange={(e) => setNewIsGlobal(e.target.checked)}
                className="rounded border-zinc-700 bg-zinc-800 text-indigo-500"
              />
              <label htmlFor="global-glossary" className="text-xs text-zinc-300">
                Make available across all projects (Global)
              </label>
            </div>

            <div className="flex items-center justify-end space-x-2 pt-3 border-t border-zinc-800">
              <Button
                variant="subtle"
                size="sm"
                onClick={() => setShowNewGlossaryModal(false)}
                className="text-xs"
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={handleCreateGlossary}
                className="text-xs"
              >
                Create Glossary
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
