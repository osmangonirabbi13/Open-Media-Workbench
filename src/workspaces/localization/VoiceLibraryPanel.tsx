import React, { useState, useEffect, useRef, useMemo } from "react";
import type { SubtitleDocument, SubtitleTrack } from "../../types/subtitles";
import type {
  VoiceCatalogEntry,
  VoiceCompatibility,
  VoicePreferences,
  VoicePreviewResult,
  VoiceComparisonResult,
} from "../../types/voices";
import {
  listVoiceLibrary,
  getVoiceCompatibility,
  getVoicePreferences,
  toggleVoiceFavorite,
  setPreferredVoice,
  startVoicePreview,
  compareVoices,
  clearVoicePreviewCache,
  assignVoiceToTrack,
  setProjectDefaultVoice,
  getProjectVoiceAssignments,
} from "../../lib/voices";
import { Button } from "../../components/ui/Button";
import {
  PlayIcon,
  PauseIcon,
  VolumeIcon,
  StarIcon,
  AlertTriangleIcon,
  CheckIcon,
  ResetIcon,
  InfoIcon,
} from "../../components/ui/Icons";

export interface VoiceLibraryPanelProps {
  document: SubtitleDocument | null;
  activeTrack: SubtitleTrack | null;
  onDocumentChange?: (doc: SubtitleDocument) => void;
  onOpenModelManager?: () => void;
  onSelectVoiceForTts?: (voiceId: string) => void;
}

function getAudioSrc(path: string): string {
  if (!path) return "";
  if (path.startsWith("blob:") || path.startsWith("data:") || path.startsWith("http")) {
    return path;
  }
  return `opentake-media://${encodeURIComponent(path)}`;
}

export function VoiceLibraryPanel({
  document,
  activeTrack,
  onDocumentChange,
  onOpenModelManager,
  onSelectVoiceForTts,
}: VoiceLibraryPanelProps): React.ReactElement {
  // Catalog & state
  const [voices, setVoices] = useState<VoiceCatalogEntry[]>([]);
  const [compatMap, setCompatMap] = useState<Record<string, VoiceCompatibility>>({});
  const [preferences, setPreferences] = useState<VoicePreferences | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [notification, setNotification] = useState<string | null>(null);

  // Filters & Search
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [selectedLanguage, setSelectedLanguage] = useState<string>("all");
  const [selectedLocale, setSelectedLocale] = useState<string>("all");
  const [statusFilter, setStatusFilter] = useState<"all" | "ready" | "uninstalled">("all");

  // Selection & Speaker
  const [selectedVoiceId, setSelectedVoiceId] = useState<string | null>(null);
  const [speakerSelections, setSpeakerSelections] = useState<Record<string, number>>({});

  // Preview State
  const [previewText, setPreviewText] = useState<string>("Welcome to Open Media Workbench.");
  const [isSynthesizing, setIsSynthesizing] = useState<boolean>(false);
  const [activePreviewResult, setActivePreviewResult] = useState<VoicePreviewResult | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [isPlayingPreview, setIsPlayingPreview] = useState<boolean>(false);
  const [previewCurrentTime, setPreviewCurrentTime] = useState<number>(0);
  const [previewDuration, setPreviewDuration] = useState<number>(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  // Comparison State
  const [showComparison, setShowComparison] = useState<boolean>(false);
  const [compareVoiceAId, setCompareVoiceAId] = useState<string>("");
  const [compareVoiceBId, setCompareVoiceBId] = useState<string>("");
  const [comparisonText, setComparisonText] = useState<string>("Welcome to Open Media Workbench.");
  const [isComparing, setIsComparing] = useState<boolean>(false);
  const [comparisonResult, setComparisonResult] = useState<VoiceComparisonResult | null>(null);
  const [comparisonError, setComparisonError] = useState<string | null>(null);
  const [playingCompareSlot, setPlayingCompareSlot] = useState<"A" | "B" | null>(null);
  const compareAudioARef = useRef<HTMLAudioElement | null>(null);
  const compareAudioBRef = useRef<HTMLAudioElement | null>(null);

  // Load catalog & preferences
  const loadData = async () => {
    setIsLoading(true);
    setErrorMsg(null);
    try {
      const [vList, prefs] = await Promise.all([
        listVoiceLibrary(),
        getVoicePreferences(),
      ]);
      setVoices(vList);
      setPreferences(prefs);

      // Check compatibility for all voices
      const cm: Record<string, VoiceCompatibility> = {};
      await Promise.all(
        vList.map(async (v) => {
          try {
            const c = await getVoiceCompatibility(v.id);
            cm[v.id] = c;
          } catch {
            // Non-blocking fallback
          }
        })
      );
      setCompatMap(cm);

      if (vList.length > 0 && !selectedVoiceId) {
        // Default to preferred voice if set, otherwise first ready or first voice
        const preferred = prefs.preferredVoiceId && vList.find((v) => v.id === prefs.preferredVoiceId);
        const readyFirst = vList.find((v) => cm[v.id]?.status === "ready");
        setSelectedVoiceId(preferred ? preferred.id : readyFirst ? readyFirst.id : vList[0].id);
      }
    } catch (err) {
      setErrorMsg(`Failed to load voice catalog: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  // Compute available languages & locales
  const availableLanguages = useMemo(() => {
    const set = new Set<string>();
    voices.forEach((v) => set.add(v.language));
    return Array.from(set).sort();
  }, [voices]);

  const availableLocales = useMemo(() => {
    const set = new Set<string>();
    voices.forEach((v) => {
      if (selectedLanguage === "all" || v.language === selectedLanguage) {
        set.add(v.locale);
      }
    });
    return Array.from(set).sort();
  }, [voices, selectedLanguage]);

  // Filtered voices
  const filteredVoices = useMemo(() => {
    return voices.filter((v) => {
      const q = searchQuery.toLowerCase().trim();
      const matchesSearch =
        !q ||
        v.displayName.toLowerCase().includes(q) ||
        v.language.toLowerCase().includes(q) ||
        v.locale.toLowerCase().includes(q) ||
        v.id.toLowerCase().includes(q) ||
        v.license.toLowerCase().includes(q);

      const matchesLang = selectedLanguage === "all" || v.language === selectedLanguage;
      const matchesLocale = selectedLocale === "all" || v.locale === selectedLocale;

      const comp = compatMap[v.id];
      const isReady = comp?.status === "ready";
      const matchesStatus =
        statusFilter === "all" ||
        (statusFilter === "ready" && isReady) ||
        (statusFilter === "uninstalled" && !isReady);

      return matchesSearch && matchesLang && matchesLocale && matchesStatus;
    });
  }, [voices, searchQuery, selectedLanguage, selectedLocale, statusFilter, compatMap]);

  // Ready voices for comparison
  const readyVoices = useMemo(() => {
    return voices.filter((v) => compatMap[v.id]?.status === "ready");
  }, [voices, compatMap]);

  // Track assignments from document
  const projectAssignments = useMemo(() => {
    return document ? getProjectVoiceAssignments(document) : { trackAssignments: [] };
  }, [document]);

  const activeTrackAssignment = useMemo(() => {
    if (!activeTrack) return null;
    return projectAssignments.trackAssignments.find((a) => a.trackId === activeTrack.id) || null;
  }, [activeTrack, projectAssignments]);

  // Handle favorite toggle
  const handleToggleFavorite = async (voiceId: string) => {
    try {
      await toggleVoiceFavorite(voiceId);
      const updated = await getVoicePreferences();
      setPreferences(updated);
    } catch (err) {
      setErrorMsg(`Failed to toggle favorite: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  // Handle set preferred
  const handleSetPreferred = async (voiceId: string) => {
    try {
      const isAlreadyPreferred = preferences?.preferredVoiceId === voiceId;
      await setPreferredVoice(isAlreadyPreferred ? undefined : voiceId);
      const updated = await getVoicePreferences();
      setPreferences(updated);
      setNotification(isAlreadyPreferred ? "Default project voice unset." : "Preferred default voice updated.");
      setTimeout(() => setNotification(null), 3000);
    } catch (err) {
      setErrorMsg(`Failed to set preferred voice: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  // Handle assign voice to active track
  const handleAssignToActiveTrack = async (voice: VoiceCatalogEntry) => {
    if (!document || !activeTrack) return;
    const speakerId = speakerSelections[voice.id] ?? (voice.supportedSpeakerIds.length > 0 ? voice.supportedSpeakerIds[0] : undefined);
    const updatedDoc = await assignVoiceToTrack(document, activeTrack.id, voice.id, speakerId);
    onDocumentChange?.(updatedDoc);
    setNotification(`Assigned ${voice.displayName} to track "${activeTrack.displayName}".`);
    setTimeout(() => setNotification(null), 3000);
  };

  // Handle set project default voice in document
  const handleSetProjectDefault = async (voice: VoiceCatalogEntry) => {
    if (!document) return;
    const updatedDoc = await setProjectDefaultVoice(document, voice.id);
    onDocumentChange?.(updatedDoc);
    setNotification(`Set ${voice.displayName} as project default voice.`);
    setTimeout(() => setNotification(null), 3000);
  };

  // Handle speaker selection change
  const handleSpeakerChange = (voiceId: string, speakerId: number) => {
    setSpeakerSelections((prev) => ({ ...prev, [voiceId]: speakerId }));
  };

  // Preview Generation
  const handleSynthesizePreview = async (voiceId: string) => {
    const voice = voices.find((v) => v.id === voiceId);
    if (!voice) return;

    const comp = compatMap[voiceId];
    if (comp?.status !== "ready") {
      setPreviewError(`Voice "${voice.displayName}" is not ready for synthesis (${comp?.status || "Unknown"}).`);
      return;
    }

    setIsSynthesizing(true);
    setPreviewError(null);
    if (audioRef.current) {
      audioRef.current.pause();
      setIsPlayingPreview(false);
    }

    try {
      const speakerId = speakerSelections[voiceId] ?? (voice.supportedSpeakerIds.length > 0 ? voice.supportedSpeakerIds[0] : undefined);
      const res = await startVoicePreview({
        voiceId,
        customText: previewText.trim() || "Welcome to Open Media Workbench.",
        speakerId,
        settings: { speedScale: 1.0, noiseScale: 0.667, noiseW: 0.8 },
      });
      setActivePreviewResult(res);

      // Play audio automatically
      if (audioRef.current) {
        audioRef.current.src = getAudioSrc(res.stagedAudioPath);
        audioRef.current.load();
        audioRef.current.play().then(() => setIsPlayingPreview(true)).catch(() => setIsPlayingPreview(false));
      }
    } catch (err) {
      setPreviewError(`Preview generation failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsSynthesizing(false);
    }
  };

  // Clear preview cache
  const handleClearCache = async () => {
    try {
      const freed = await clearVoicePreviewCache();
      setNotification(`Preview cache cleared (${(freed / 1024).toFixed(1)} KB freed).`);
      setTimeout(() => setNotification(null), 3000);
      setActivePreviewResult(null);
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current.src = "";
      }
    } catch (err) {
      setErrorMsg(`Failed to clear cache: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  // Compare voices
  const handleStartComparison = async () => {
    if (!compareVoiceAId || !compareVoiceBId) {
      setComparisonError("Please select both Voice A and Voice B to compare.");
      return;
    }
    if (compareVoiceAId === compareVoiceBId) {
      setComparisonError("Cannot compare a voice against itself. Please select two distinct voices.");
      return;
    }

    setIsComparing(true);
    setComparisonError(null);
    try {
      const res = await compareVoices({
        voiceIdA: compareVoiceAId,
        voiceIdB: compareVoiceBId,
        text: comparisonText.trim() || "Welcome to Open Media Workbench.",
      });
      setComparisonResult(res);
    } catch (err) {
      setComparisonError(`Comparison failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsComparing(false);
    }
  };

  return (
    <div className="h-full flex flex-col bg-zinc-950 text-zinc-100 overflow-hidden select-none">
      {/* Hidden Audio Elements */}
      <audio
        ref={audioRef}
        onTimeUpdate={() => {
          if (audioRef.current) {
            setPreviewCurrentTime(audioRef.current.currentTime);
          }
        }}
        onLoadedMetadata={() => {
          if (audioRef.current) {
            setPreviewDuration(audioRef.current.duration);
          }
        }}
        onEnded={() => {
          setIsPlayingPreview(false);
          setPreviewCurrentTime(0);
        }}
      />
      <audio
        ref={compareAudioARef}
        onEnded={() => setPlayingCompareSlot(null)}
      />
      <audio
        ref={compareAudioBRef}
        onEnded={() => setPlayingCompareSlot(null)}
      />

      {/* Top Header */}
      <div className="p-3.5 border-b border-zinc-800 bg-zinc-900/60 flex items-center justify-between shrink-0">
        <div className="flex items-center space-x-2">
          <div className="p-1.5 rounded-md bg-indigo-950/80 border border-indigo-700/60 text-indigo-400">
            <VolumeIcon size={16} />
          </div>
          <div>
            <div className="flex items-center space-x-2">
              <h3 className="text-xs font-semibold text-zinc-100 tracking-wide uppercase">AI Voice Library</h3>
              <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-950 text-emerald-300 border border-emerald-800">
                Offline Verified
              </span>
            </div>
            <p className="text-[11px] text-zinc-400">
              Browse, preview, compare, and assign verified offline voice models.
            </p>
          </div>
        </div>

        <div className="flex items-center space-x-2">
          <button
            onClick={() => setShowComparison(!showComparison)}
            className={`px-2.5 py-1.5 rounded text-xs font-medium border transition-colors ${
              showComparison
                ? "bg-indigo-600 border-indigo-500 text-white"
                : "bg-zinc-800 border-zinc-700 text-zinc-300 hover:bg-zinc-700"
            }`}
          >
            A/B Compare
          </button>
          <button
            onClick={handleClearCache}
            title="Clear preview cache"
            className="p-1.5 rounded bg-zinc-800/80 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 border border-zinc-700/60 transition-colors text-xs flex items-center space-x-1"
          >
            <ResetIcon size={13} />
            <span className="text-[11px]">Clear Cache</span>
          </button>
          {onOpenModelManager && (
            <Button
              variant="secondary"
              size="sm"
              onClick={onOpenModelManager}
              className="text-xs border-zinc-700 text-zinc-300 hover:bg-zinc-800"
            >
              Model Manager
            </Button>
          )}
        </div>
      </div>

      {/* Notification Toast */}
      {notification && (
        <div className="px-4 py-2 bg-emerald-950/80 border-b border-emerald-800 text-emerald-200 text-xs flex items-center space-x-2">
          <CheckIcon size={14} className="text-emerald-400 shrink-0" />
          <span>{notification}</span>
        </div>
      )}

      {/* Error Banner */}
      {errorMsg && (
        <div className="px-4 py-2 bg-red-950/80 border-b border-red-800 text-red-200 text-xs flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <AlertTriangleIcon size={14} className="text-red-400 shrink-0" />
            <span>{errorMsg}</span>
          </div>
          <button onClick={() => setErrorMsg(null)} className="text-red-400 hover:text-red-200 text-xs">✕</button>
        </div>
      )}

      {/* A/B Comparison Drawer / Section */}
      {showComparison && (
        <div className="border-b border-zinc-800 bg-zinc-900/90 p-4 space-y-3 shrink-0">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <span className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">Voice A/B Comparison</span>
              <span className="text-[10px] text-zinc-400">Compare two ready voices on identical text</span>
            </div>
            <button
              onClick={() => setShowComparison(false)}
              className="text-zinc-400 hover:text-zinc-200 text-xs"
            >
              Close
            </button>
          </div>

          {readyVoices.length < 2 ? (
            <div className="p-3 rounded-lg border border-amber-900/60 bg-amber-950/30 text-amber-200 text-xs flex items-start space-x-2">
              <InfoIcon size={15} className="text-amber-400 shrink-0 mt-0.5" />
              <div>
                <p className="font-medium">Install another compatible voice to compare.</p>
                <p className="text-[11px] text-amber-300/80 mt-0.5">
                  Real A/B comparison requires two distinct verified voices installed locally. Currently, {readyVoices.length} ready voice is available.
                </p>
                {onOpenModelManager && (
                  <button
                    onClick={onOpenModelManager}
                    className="mt-2 px-2.5 py-1 rounded text-[11px] font-medium bg-amber-600/30 hover:bg-amber-600/50 border border-amber-500/60 text-amber-100"
                  >
                    Open AI Model Manager
                  </button>
                )}
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-[11px] text-zinc-400 block mb-1">Voice A</label>
                  <select
                    value={compareVoiceAId}
                    onChange={(e) => setCompareVoiceAId(e.target.value)}
                    className="w-full bg-zinc-950 border border-zinc-700 rounded px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
                  >
                    <option value="">Select Voice A</option>
                    {readyVoices.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.displayName} ({v.locale})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="text-[11px] text-zinc-400 block mb-1">Voice B</label>
                  <select
                    value={compareVoiceBId}
                    onChange={(e) => setCompareVoiceBId(e.target.value)}
                    className="w-full bg-zinc-950 border border-zinc-700 rounded px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
                  >
                    <option value="">Select Voice B</option>
                    {readyVoices.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.displayName} ({v.locale})
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div>
                <label className="text-[11px] text-zinc-400 block mb-1">Comparison Text</label>
                <input
                  type="text"
                  value={comparisonText}
                  onChange={(e) => setComparisonText(e.target.value)}
                  className="w-full bg-zinc-950 border border-zinc-700 rounded px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
                  placeholder="Enter sample speech to compare..."
                />
              </div>

              {comparisonError && (
                <p className="text-xs text-red-400">{comparisonError}</p>
              )}

              <Button
                variant="primary"
                size="sm"
                onClick={handleStartComparison}
                disabled={isComparing || !compareVoiceAId || !compareVoiceBId}
                className="w-full"
              >
                {isComparing ? "Synthesizing Both Voices..." : "Run Side-by-Side Comparison"}
              </Button>

              {comparisonResult && (
                <div className="grid grid-cols-2 gap-3 pt-2">
                  {/* Slot A */}
                  <div className="p-3 rounded-lg border border-zinc-700 bg-zinc-950/80 space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-semibold text-indigo-400">Voice A: {comparisonResult.voiceA.voiceId}</span>
                      <span className="text-[10px] font-mono text-zinc-400">{(comparisonResult.voiceA.durationUs / 1_000_000).toFixed(2)}s</span>
                    </div>
                    <p className="text-[10px] text-zinc-400 font-mono">
                      {comparisonResult.voiceA.sampleRate} Hz · {comparisonResult.voiceA.cached ? "Cached" : "Synthesized"}
                    </p>
                    <div className="flex items-center space-x-2 pt-1">
                      <button
                        onClick={() => {
                          if (compareAudioARef.current) {
                            if (playingCompareSlot === "A") {
                              compareAudioARef.current.pause();
                              setPlayingCompareSlot(null);
                            } else {
                              if (compareAudioBRef.current) compareAudioBRef.current.pause();
                              compareAudioARef.current.src = getAudioSrc(comparisonResult.voiceA.stagedAudioPath);
                              compareAudioARef.current.play();
                              setPlayingCompareSlot("A");
                            }
                          }
                        }}
                        className="px-2 py-1 rounded bg-indigo-600 hover:bg-indigo-500 text-white text-[11px] font-medium flex items-center space-x-1"
                      >
                        {playingCompareSlot === "A" ? <PauseIcon size={12} /> : <PlayIcon size={12} />}
                        <span>{playingCompareSlot === "A" ? "Pause" : "Play A"}</span>
                      </button>
                      <button
                        onClick={() => handleSetPreferred(comparisonResult.voiceA.voiceId)}
                        className="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-[11px]"
                      >
                        Set Preferred
                      </button>
                    </div>
                  </div>

                  {/* Slot B */}
                  <div className="p-3 rounded-lg border border-zinc-700 bg-zinc-950/80 space-y-2">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-semibold text-purple-400">Voice B: {comparisonResult.voiceB.voiceId}</span>
                      <span className="text-[10px] font-mono text-zinc-400">{(comparisonResult.voiceB.durationUs / 1_000_000).toFixed(2)}s</span>
                    </div>
                    <p className="text-[10px] text-zinc-400 font-mono">
                      {comparisonResult.voiceB.sampleRate} Hz · {comparisonResult.voiceB.cached ? "Cached" : "Synthesized"}
                    </p>
                    <div className="flex items-center space-x-2 pt-1">
                      <button
                        onClick={() => {
                          if (compareAudioBRef.current) {
                            if (playingCompareSlot === "B") {
                              compareAudioBRef.current.pause();
                              setPlayingCompareSlot(null);
                            } else {
                              if (compareAudioARef.current) compareAudioARef.current.pause();
                              compareAudioBRef.current.src = getAudioSrc(comparisonResult.voiceB.stagedAudioPath);
                              compareAudioBRef.current.play();
                              setPlayingCompareSlot("B");
                            }
                          }
                        }}
                        className="px-2 py-1 rounded bg-purple-600 hover:bg-purple-500 text-white text-[11px] font-medium flex items-center space-x-1"
                      >
                        {playingCompareSlot === "B" ? <PauseIcon size={12} /> : <PlayIcon size={12} />}
                        <span>{playingCompareSlot === "B" ? "Pause" : "Play B"}</span>
                      </button>
                      <button
                        onClick={() => handleSetPreferred(comparisonResult.voiceB.voiceId)}
                        className="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-[11px]"
                      >
                        Set Preferred
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Filter and Search Bar */}
      <div className="p-3 border-b border-zinc-800 bg-zinc-900/40 grid grid-cols-4 gap-2.5 shrink-0">
        <div>
          <label className="text-[10px] text-zinc-400 block mb-1">Search Voices</label>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search by name, locale, license..."
            className="w-full bg-zinc-950 border border-zinc-700 rounded px-2.5 py-1 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-indigo-500"
          />
        </div>
        <div>
          <label className="text-[10px] text-zinc-400 block mb-1">Language</label>
          <select
            value={selectedLanguage}
            onChange={(e) => {
              setSelectedLanguage(e.target.value);
              setSelectedLocale("all");
            }}
            className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
          >
            <option value="all">All Languages ({availableLanguages.length})</option>
            {availableLanguages.map((lang) => (
              <option key={lang} value={lang}>{lang.toUpperCase()}</option>
            ))}
          </select>
        </div>
        <div>
          <label className="text-[10px] text-zinc-400 block mb-1">Locale</label>
          <select
            value={selectedLocale}
            onChange={(e) => setSelectedLocale(e.target.value)}
            className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
          >
            <option value="all">All Locales</option>
            {availableLocales.map((loc) => (
              <option key={loc} value={loc}>{loc}</option>
            ))}
          </select>
        </div>
        <div>
          <label className="text-[10px] text-zinc-400 block mb-1">Install Status</label>
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as "all" | "ready" | "uninstalled")}
            className="w-full bg-zinc-950 border border-zinc-700 rounded px-2 py-1 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
          >
            <option value="all">All Statuses</option>
            <option value="ready">Ready / Installed</option>
            <option value="uninstalled">Available for Install</option>
          </select>
        </div>
      </div>

      {/* Main Content Area: Split List & Detail/Preview */}
      <div className="flex-1 min-h-0 flex overflow-hidden">
        {/* Left: Voice Cards List */}
        <div className="w-1/2 border-r border-zinc-800 overflow-y-auto p-3 space-y-2">
          {isLoading ? (
            <div className="p-8 text-center text-xs text-zinc-400">Loading verified voice catalog...</div>
          ) : filteredVoices.length === 0 ? (
            <div className="p-8 text-center text-xs text-zinc-500">
              No voices match your search and filter criteria.
            </div>
          ) : (
            filteredVoices.map((voice) => {
              const comp = compatMap[voice.id];
              const isSelected = selectedVoiceId === voice.id;
              const isFavorite = preferences?.favoriteVoiceIds.includes(voice.id) || false;
              const isPreferred = preferences?.preferredVoiceId === voice.id;
              const isReady = comp?.status === "ready";

              return (
                <div
                  key={voice.id}
                  onClick={() => setSelectedVoiceId(voice.id)}
                  className={`p-3 rounded-lg border transition-all cursor-pointer ${
                    isSelected
                      ? "bg-zinc-800/80 border-indigo-500 shadow-sm"
                      : "bg-zinc-900/50 border-zinc-800 hover:bg-zinc-800/40 hover:border-zinc-700"
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="text-xs font-semibold text-zinc-100">{voice.displayName}</span>
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-zinc-800 text-zinc-300 border border-zinc-700">
                          {voice.locale}
                        </span>
                        {voice.qualityTier && (
                          <span className="px-1.5 py-0.5 rounded text-[10px] bg-zinc-800/60 text-zinc-400">
                            {voice.qualityTier}
                          </span>
                        )}
                      </div>
                      <p className="text-[10px] text-zinc-400 mt-0.5 font-mono">
                        {voice.modelId} · {(voice.artifactSizeBytes / 1024 / 1024).toFixed(1)} MB
                      </p>
                    </div>

                    {/* Star & Status */}
                    <div className="flex items-center space-x-1.5" onClick={(e) => e.stopPropagation()}>
                      {isPreferred && (
                        <span className="px-1.5 py-0.5 rounded text-[9px] font-medium bg-amber-950 text-amber-300 border border-amber-800">
                          Default
                        </span>
                      )}
                      <button
                        onClick={() => handleToggleFavorite(voice.id)}
                        className={`p-1 rounded transition-colors ${
                          isFavorite ? "text-amber-400 hover:text-amber-300" : "text-zinc-600 hover:text-zinc-400"
                        }`}
                        title={isFavorite ? "Remove favorite" : "Mark as favorite"}
                      >
                        <StarIcon size={14} filled={isFavorite} />
                      </button>
                    </div>
                  </div>

                  <div className="flex items-center justify-between mt-2 pt-2 border-t border-zinc-800/60">
                    <div className="flex items-center space-x-2">
                      {/* Readiness status badge */}
                      <span
                        className={`px-1.5 py-0.5 rounded text-[10px] font-medium border ${
                          isReady
                            ? "bg-emerald-950 text-emerald-300 border-emerald-800"
                            : comp?.status === "incompatible"
                            ? "bg-red-950 text-red-300 border-red-800"
                            : "bg-zinc-800 text-zinc-400 border-zinc-700"
                        }`}
                      >
                        {comp?.status === "ready"
                          ? "Ready"
                          : comp?.status === "availableForInstall"
                          ? "Available to Install"
                          : comp?.status || "Checking..."}
                      </span>

                      {/* License badge */}
                      <span className="text-[10px] text-zinc-500 truncate max-w-[140px]" title={voice.license}>
                        {voice.license}
                      </span>
                    </div>

                    {voice.speakerCount > 1 && (
                      <span className="text-[10px] text-indigo-400">
                        {voice.speakerCount} speakers
                      </span>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Right: Selected Voice Details & Real Preview Controls */}
        <div className="w-1/2 flex flex-col overflow-y-auto p-4 space-y-4">
          {(() => {
            const voice = voices.find((v) => v.id === selectedVoiceId);
            if (!voice) {
              return (
                <div className="p-8 text-center text-xs text-zinc-500">
                  Select a voice from the library to inspect metadata, test preview, or assign to tracks.
                </div>
              );
            }

            const comp = compatMap[voice.id];
            const isReady = comp?.status === "ready";
            const isFavorite = preferences?.favoriteVoiceIds.includes(voice.id) || false;
            const isPreferred = preferences?.preferredVoiceId === voice.id;
            const currentSpeaker = speakerSelections[voice.id] ?? (voice.supportedSpeakerIds.length > 0 ? voice.supportedSpeakerIds[0] : 0);

            return (
              <div className="space-y-4">
                {/* Header Card */}
                <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-3">
                  <div className="flex items-start justify-between">
                    <div>
                      <h4 className="text-sm font-semibold text-zinc-100">{voice.displayName}</h4>
                      <p className="text-xs text-zinc-400 font-mono mt-0.5">{voice.modelId}</p>
                    </div>

                    <div className="flex items-center space-x-1.5">
                      <button
                        onClick={() => handleToggleFavorite(voice.id)}
                        className={`px-2 py-1 rounded text-xs border flex items-center space-x-1 ${
                          isFavorite
                            ? "bg-amber-950/60 border-amber-800 text-amber-300"
                            : "bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        <StarIcon size={12} filled={isFavorite} />
                        <span>{isFavorite ? "Favorited" : "Favorite"}</span>
                      </button>
                      <button
                        onClick={() => handleSetPreferred(voice.id)}
                        className={`px-2 py-1 rounded text-xs border ${
                          isPreferred
                            ? "bg-indigo-950/60 border-indigo-700 text-indigo-300"
                            : "bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {isPreferred ? "Project Default" : "Set Default"}
                      </button>
                    </div>
                  </div>

                  {/* Metadata Grid */}
                  <div className="grid grid-cols-2 gap-2 text-xs pt-1 border-t border-zinc-800">
                    <div>
                      <span className="text-[10px] text-zinc-500 block">Language & Locale</span>
                      <span className="text-zinc-300 font-medium">{voice.language.toUpperCase()} ({voice.locale})</span>
                    </div>
                    <div>
                      <span className="text-[10px] text-zinc-500 block">Runtime Backend</span>
                      <span className="text-zinc-300">{voice.runtimeBackend}</span>
                    </div>
                    <div>
                      <span className="text-[10px] text-zinc-500 block">License</span>
                      <span className="text-zinc-300">{voice.license}</span>
                    </div>
                    <div>
                      <span className="text-[10px] text-zinc-500 block">Model Size</span>
                      <span className="text-zinc-300 font-mono">{(voice.artifactSizeBytes / 1024 / 1024).toFixed(1)} MB</span>
                    </div>
                  </div>

                  {/* Attribution Notice */}
                  {voice.attribution && (
                    <div className="p-2 rounded bg-zinc-950/60 border border-zinc-800 text-[11px] text-zinc-400">
                      {voice.attribution}
                    </div>
                  )}

                  {/* Multi-speaker selection if applicable */}
                  {voice.speakerCount > 1 && (
                    <div className="pt-2 border-t border-zinc-800">
                      <label className="text-[11px] text-zinc-400 block mb-1">Speaker Variant</label>
                      <select
                        value={currentSpeaker}
                        onChange={(e) => handleSpeakerChange(voice.id, parseInt(e.target.value, 10))}
                        className="w-full bg-zinc-950 border border-zinc-700 rounded px-2.5 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500"
                      >
                        {voice.supportedSpeakerIds.map((sid) => (
                          <option key={sid} value={sid}>
                            Speaker {sid}
                          </option>
                        ))}
                      </select>
                    </div>
                  )}

                  {/* Action buttons: Install, Assign to Track, Select for TTS */}
                  <div className="flex flex-wrap gap-2 pt-2 border-t border-zinc-800">
                    {!isReady ? (
                      onOpenModelManager && (
                        <Button
                          variant="primary"
                          size="sm"
                          onClick={onOpenModelManager}
                          className="w-full text-xs"
                        >
                          Install Model via AI Model Manager
                        </Button>
                      )
                    ) : (
                      <>
                        {activeTrack && (
                          <Button
                            variant="secondary"
                            size="sm"
                            onClick={() => handleAssignToActiveTrack(voice)}
                            className="text-xs border-zinc-700 text-zinc-300 hover:bg-zinc-800"
                          >
                            Assign to Active Track ({activeTrack.displayName})
                          </Button>
                        )}
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => handleSetProjectDefault(voice)}
                          className="text-xs border-zinc-700 text-zinc-300 hover:bg-zinc-800"
                        >
                          Set as Project Default
                        </Button>
                        {onSelectVoiceForTts && (
                          <Button
                            variant="subtle"
                            size="sm"
                            onClick={() => onSelectVoiceForTts(voice.id)}
                            className="text-xs text-indigo-300 hover:bg-indigo-950"
                          >
                            Use in Text-to-Speech
                          </Button>
                        )}
                      </>
                    )}
                  </div>
                </div>

                {/* Real Preview Section */}
                <div className="p-3.5 rounded-lg border border-zinc-800 bg-zinc-900/60 space-y-3">
                  <div className="flex items-center justify-between">
                    <h5 className="text-xs font-semibold text-zinc-200 flex items-center space-x-1.5">
                      <VolumeIcon size={14} className="text-indigo-400" />
                      <span>Real Voice Preview</span>
                    </h5>
                    <span className="text-[10px] text-zinc-500 font-mono">100% Offline Synthesis</span>
                  </div>

                  <div>
                    <label className="text-[11px] text-zinc-400 block mb-1">Preview Sample Text</label>
                    <textarea
                      rows={2}
                      value={previewText}
                      onChange={(e) => setPreviewText(e.target.value)}
                      className="w-full bg-zinc-950 border border-zinc-700 rounded p-2 text-xs text-zinc-200 focus:outline-none focus:border-indigo-500 resize-none"
                      placeholder="Enter preview text..."
                    />
                  </div>

                  {previewError && (
                    <div className="p-2 rounded bg-red-950/60 border border-red-800 text-[11px] text-red-300">
                      {previewError}
                    </div>
                  )}

                  <Button
                    variant="primary"
                    size="sm"
                    onClick={() => handleSynthesizePreview(voice.id)}
                    disabled={isSynthesizing || !isReady}
                    className="w-full text-xs"
                  >
                    {isSynthesizing
                      ? "Synthesizing Speech..."
                      : !isReady
                      ? "Voice Not Installed"
                      : "Generate & Play Preview"}
                  </Button>

                  {/* Active Preview Audio Player */}
                  {activePreviewResult && (
                    <div className="p-3 rounded-lg border border-zinc-700 bg-zinc-950 space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-medium text-zinc-300">Preview Audio</span>
                        <span className="text-[10px] font-mono text-zinc-400">
                          {previewCurrentTime.toFixed(1)}s / {(activePreviewResult.durationUs / 1_000_000).toFixed(2)}s
                        </span>
                      </div>

                      {/* Timeline Slider */}
                      <input
                        type="range"
                        min={0}
                        max={previewDuration || activePreviewResult.durationUs / 1_000_000}
                        step={0.05}
                        value={previewCurrentTime}
                        onChange={(e) => {
                          const val = parseFloat(e.target.value);
                          setPreviewCurrentTime(val);
                          if (audioRef.current) {
                            audioRef.current.currentTime = val;
                          }
                        }}
                        className="w-full h-1 bg-zinc-700 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                      />

                      {/* Controls & Badges */}
                      <div className="flex items-center justify-between pt-1">
                        <div className="flex items-center space-x-2">
                          <button
                            onClick={() => {
                              if (audioRef.current) {
                                if (isPlayingPreview) {
                                  audioRef.current.pause();
                                  setIsPlayingPreview(false);
                                } else {
                                  audioRef.current.play().then(() => setIsPlayingPreview(true));
                                }
                              }
                            }}
                            className="p-1.5 rounded-full bg-indigo-600 hover:bg-indigo-500 text-white"
                          >
                            {isPlayingPreview ? <PauseIcon size={13} /> : <PlayIcon size={13} />}
                          </button>
                          <span className="text-[10px] font-mono text-zinc-400">
                            {activePreviewResult.sampleRate} Hz · {activePreviewResult.channelCount}ch
                          </span>
                        </div>

                        <span className="text-[10px] text-zinc-500">
                          {activePreviewResult.cached ? "Reused from Cache" : "Fresh Synthesis"}
                        </span>
                      </div>
                    </div>
                  )}
                </div>

                {/* Track Assignment Status Display */}
                {activeTrackAssignment && activeTrackAssignment.voiceId === voice.id && (
                  <div className="p-3 rounded-lg border border-emerald-900/60 bg-emerald-950/30 text-emerald-300 text-xs flex items-center space-x-2">
                    <CheckIcon size={14} className="text-emerald-400 shrink-0" />
                    <span>Currently assigned to active track: "{activeTrack?.displayName}"</span>
                  </div>
                )}
              </div>
            );
          })()}
        </div>
      </div>
    </div>
  );
}
