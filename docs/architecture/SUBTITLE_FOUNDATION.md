# Open Media Workbench — Subtitle Foundation Architecture (STEP 16)

**Module:** `crates/opentake-core/src/localization/subtitles/`  
**Desktop IPC:** `src-tauri/src/commands/subtitles.rs`  
**Frontend Components:** `src/workspaces/localization/SubtitleTrackManager.tsx`, `src/components/editor/SubtitleOverlay.tsx`  
**Document Version:** 1.0.0  
**Status:** STEP 16 Completed  

---

## 1. Architectural Vision

In Open Media Workbench, the **Localization Studio** provides content creators, subtitlers, and archivists with an uncompromising, offline-first environment for multi-language captioning and localization. 

The subtitle foundation implemented in **STEP 16** establishes the authoritative, lossless subtitle substrate upon which upcoming interactive editing (STEP 17–18), local Whisper speech-to-text (STEP 19), neural translation (STEP 22), and voice dubbing (STEP 24–28) operate:

```
┌────────────────────────────────────────────────────────────────────────┐
│               Localization Studio UI (React 19)                        │
│  ┌──────────────────────────────┐    ┌──────────────────────────────┐  │
│  │   Preview Caption Overlay    │    │    Subtitle Track Manager    │  │
│  │ - HTML5 video synchronization│    │ - SRT / WebVTT file import   │  │
│  │ - Safe Unicode/Bengali text  │    │ - Multi-track management     │  │
│  │ - Closed Captions (CC) toggle│    │ - Click-to-seek cue list     │  │
│  │ - Bounded binary-search sync │    │ - Validation notices drawer  │  │
│  └──────────────┬───────────────┘    └──────────────┬───────────────┘  │
└─────────────────┼───────────────────────────────────┼──────────────────┘
                  │ Tauri IPC (`opentake-core` IPC)  │
┌─────────────────┴───────────────────────────────────┴──────────────────┐
│                   Tauri 2 Native Desktop Layer                         │
│  ├── Native rfd File Choosers (`.srt`, `.vtt`, `.omwsub`)              │
│  ├── Security Authorization (`SourceProvenance::SubtitleFileDialog`)   │
│  └── Atomic Crash-Resistant Persistence (`tempfile` + rename)          │
└─────────────────┬───────────────────────────────────┬──────────────────┘
                  │ Headless Rust Core Domain                            │
┌─────────────────┴───────────────────────────────────┴──────────────────┐
│                 opentake-core::localization::subtitles                 │
│  ├── model.rs          (Canonical microsecond domain data structures)  │
│  ├── srt.rs            (SubRip parser & deterministic serializer)      │
│  ├── webvtt.rs         (WebVTT standards parser, settings & cleaner)   │
│  ├── validation.rs     (Multi-level automated diagnostic inspector)   │
│  ├── serialization.rs  (Unified format dispatcher & detectors)        │
│  └── persistence.rs    (Versioned atomic JSON document store)          │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Canonical Subtitle Domain Model

All temporal values in Open Media Workbench subtitle tracks are represented using the integer microsecond time primitive (`MediaTimeUs`), preventing the timing drift, frame-jitter, and compounding truncation errors inherent in floating-point seconds.

### 2.1 Domain Invariants

1. **Non-Negative Start Times:** $t_{\text{start}} \ge 0$. Negative start timestamps are strictly rejected.
2. **Positive Duration:** $t_{\text{end}} > t_{\text{start}}$. A cue cannot end before or at its start time.
3. **Multiline Unicode Preservation:** Full support for non-Latin scripts (e.g. Bengali `বাংলা`, Arabic, Japanese, emojis) without character loss or mojibake.
4. **Stable Identifiers:** Every cue retains a UUID-based identifier that remains stable across persistence round-trips.

### 2.2 Data Structures

```rust
pub struct SubtitleCue {
    pub id: String,
    pub start_us: MediaTimeUs,
    pub end_us: MediaTimeUs,
    pub text: String,
    pub speaker: Option<String>,
    pub settings: Option<WebVttCueSettings>,
    pub raw_identifier: Option<String>,
    pub custom_metadata: HashMap<String, String>,
}

pub struct SubtitleTrack {
    pub id: String,
    pub language: String,
    pub display_name: String,
    pub source_format: SubtitleFormat,
    pub cues: Vec<SubtitleCue>,
    pub is_enabled: bool,
    pub is_visible: bool,
    pub schema_version: u32,
}

pub struct SubtitleDocument {
    pub schema_version: u32,
    pub id: String,
    pub associated_media_asset_id: Option<String>,
    pub tracks: Vec<SubtitleTrack>,
    pub active_track_id: Option<String>,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
    pub metadata: HashMap<String, String>,
}
```

---

## 3. SubRip (.srt) and WebVTT (.vtt) Specifications

### 3.1 SubRip (.srt) Parsing & Serialization

- **Tolerant Ingestion:** Accepts UTF-8 with or without Byte Order Mark (`\u{feff}`), normalizes Windows (`\r\n`), Mac (`\r`), and Unix (`\n`) line endings, and tolerates missing or out-of-order sequence index numbers.
- **Timestamp Arithmetic:** Supports standard comma millisecond delimiters (`00:01:23,456`) and dot delimiters (`00:01:23.456`). Trailing coordinate tokens (e.g. `X1:000 Y1:000`) are cleanly stripped.
- **Deterministic Serialization:** Sequential cue indices (1, 2, 3...) formatted chronologically with exact nearest-millisecond rounding (`(us + 500) / 1000`).

### 3.2 WebVTT (.vtt) Standards Parsing

- **Signature Header:** Requires valid `WEBVTT` header line.
- **Block Classification:** Cleanly skips `NOTE` comment blocks without error. Transparently bypasses `STYLE` and `REGION` blocks with informative compatibility warnings.
- **Short Timestamps:** Supports both full `HH:MM:SS.mmm` and compact `MM:SS.mmm` syntax.
- **Cue Settings:** Preserves positioning attributes: `line:`, `position:`, `size:`, `align:`, and `vertical:`.
- **Safe Text Extraction:** Extracts `<v Speaker>` voice tags into structured speaker attributes while sanitizing HTML markup (`<b>`, `<i>`, `<c>`). HTML entities (`&amp;`, `&lt;`, `&gt;`, `&nbsp;`) are unescaped to pure Unicode.

---

## 4. Structured Validation Diagnostic Engine

The validation engine inspects tracks without mutating content:

| Finding Code | Severity | Description |
|---|---|---|
| `EMPTY_TRACK` | Warning | Track has no subtitle cues |
| `EMPTY_CUE_TEXT` | Warning | Cue contains empty or whitespace-only text |
| `INVALID_TIME_RANGE` | Error | End time is less than or equal to start time |
| `NEGATIVE_START_TIME` | Error | Start time is less than zero |
| `UNSORTED_CUES` | Warning | Cue start time precedes previous cue start time |
| `OVERLAPPING_CUES` | Warning | Cue start time falls before previous cue end time (permitted for concurrent speakers) |
| `DUPLICATE_CUE_ID` | Warning | Multiple cues share the identical ID |
| `EXCESSIVE_DURATION` | Warning | Cue duration exceeds 30 seconds |
| `EXTREMELY_SHORT_DURATION` | Info | Cue duration is shorter than 100 milliseconds |
| `OUT_OF_MEDIA_BOUNDS` | Warning | Cue timestamp exceeds verified container duration |

---

## 5. Security & File Provenance

1. **Origin Verification:** Subtitle files imported via native file pickers are registered in `MediaAuthorizationRegistry` with `SourceProvenance::SubtitleFileDialog`.
2. **Resource Boundaries:** Hard ceilings are enforced:
   - Maximum file size: **10 MB** (`MAX_SUBTITLE_FILE_SIZE`).
   - Maximum cue count: **20,000 cues** per track (`MAX_SUBTITLE_CUE_COUNT`).
3. **No Arbitrary Filesystem Writes:** React cannot pass arbitrary paths to write files. All exports and saves execute via native OS dialogs with atomic temporary file swapping (`.tmp` -> `rename`).
4. **Media Immutability:** Video files are 100% read-only. Subtitles are bound by asset ID without embedding or touching video bytes.

---

## 6. Timeline Synchronization & Performance

- **Authoritative Coordinate Space:** Cues are authored in **Source Media Time** (`MediaTimeUs`).
- **Binary Search Lookup ($O(\log N)$):** During video playback (up to 60 FPS), the UI uses binary partition search rather than $O(N)$ linear scans to identify active cues, ensuring buttery-smooth laptop performance with files containing thousands of cues.
- **Safe Caption Overlay:** Captions are rendered as pure React text elements with text shadows and backdrop blur. `dangerouslySetInnerHTML` is strictly prohibited.

---

## 7. Interactive Subtitle Editing Architecture (STEP 17)

The foundations laid in STEP 16 are extended into a full interactive timeline editor in STEP 17. See [SUBTITLE_EDITOR.md](SUBTITLE_EDITOR.md) for complete details on:
- The canonical `SubtitleOperation` domain engine in Rust and TypeScript
- Direct visual manipulation on the audio waveform canvas
- Snapping algorithms and visual alignment
- Multi-level history and crash recovery architecture

