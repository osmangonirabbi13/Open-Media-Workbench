# OpenTake — Audio Analysis and Waveform Architecture

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** Implementation Blueprint for STEP 8, STEP 14 & STEP 15  
**Author:** OpenTake Core Architecture & Audio DSP Lead  
**Date:** 2026-10-13  

---

## 1. Architectural Separation: Visualization vs. Silence Detection

OpenTake strictly decouples **waveform visualization** from **silence detection analysis**:

```
                              [Raw Media File]
                                     │
                 ┌───────────────────┴───────────────────┐
                 │ FFmpeg PCM Pipe (s16le, 16kHz/48kHz)  │
                 └───────────────────┬───────────────────┘
                                     │
                 ┌───────────────────┴───────────────────┐
                 ▼                                       ▼
    [Subsystem 1: Waveform Engine]           [Subsystem 2: Silence DSP Engine]
    - Min/Max peak decibel bins              - Sliding-window RMS energy
    - Multi-resolution pyramid (LOD)         - dBFS thresholding with hysteresis
    - Read-only canvas rendering             - Pre/post boundary padding
    - Cached for timeline zoom               - Returns candidate suggestion list
```

---

## 2. Audio Decoding & Waveform Generation Engine

### 2.1 Streaming PCM Sample Extraction
Audio is extracted via a managed FFmpeg child process streaming 16-bit signed little-endian PCM (`s16le`) over stdout. OpenTake **never loads monolithic audio buffers into memory**. Samples are read in streaming 64 KB buffers ($32{,}768$ samples):

```text
ffmpeg -i input.mp4 -vn -ac 1 -ar 16000 -f s16le -
```
Downsampling audio to 16,000 Hz mono for waveform and silence analysis reduces data volume by 66% while preserving 100% of the speech frequency band (up to 8,000 Hz Nyquist).

### 2.2 Multi-Resolution Pyramid Waveform (Level of Detail)
To enable instantaneous, 60 FPS zooming from a 3-hour bird's-eye view down to individual word boundaries, waveform data is organized into three hierarchical resolutions:

1. **Level 0 (Full Timeline View):** 10 buckets per second. (For a 2-hour video: 72,000 bins $\approx 288\text{ KB}$ RAM).
2. **Level 1 (Standard Zoom):** 100 buckets per second. (720,000 bins $\approx 2.8\text{ MB}$ RAM).
3. **Level 2 (Deep Waveform Zoom):** 1,000 buckets per second. (Generated on-demand for active viewport; $< 2\text{ MB}$ RAM).

Each bucket stores two 16-bit integers: `min_sample: i16` and `max_sample: i16`. Total memory footprint for a 3-hour video stays **under 15 MB**.

---

## 3. Deterministic Silence Detection Algorithm

> [!NOTE]
> **Ethical Engineering Baseline:** OpenTake's silence detection is a **deterministic digital signal processing (DSP) energy analyzer**. It is not semantic speech understanding or deep-learning transcription (which is deferred to P1 via `whisper.cpp`).

### 3.1 Mathematical Formulation
1. **Windowed RMS Energy Calculation:**  
   For a sliding analysis window of length $N$ samples (default: $50\text{ ms} = 800$ samples at 16 kHz) with a $50\%$ hop size ($25\text{ ms} = 400$ samples):
   $$\text{RMS} = \sqrt{\frac{1}{N} \sum_{i=1}^{N} s_i^2}$$
2. **Decibels Relative to Full Scale (dBFS):**
   $$\text{dBFS} = 20 \log_{10} \left( \frac{\text{RMS}}{32{,}768.0} + 10^{-9} \right)$$
   *(where $10^{-9}$ prevents $\log(0)$ on absolute digital silence).*

### 3.2 Hysteresis & State Machine
To prevent rapid chattering around the threshold boundary (false flickering between speech and silence), OpenTake uses **dual-threshold hysteresis**:
- **Silence Entry Threshold:** $\text{dBFS} \le T_{\text{silence}}$ (e.g., $-35\text{ dBFS}$).
- **Speech Resumption Threshold:** $\text{dBFS} \ge T_{\text{silence}} + 3.0\text{ dB}$ (e.g., $-32\text{ dBFS}$).

```
             ┌─────────────────────────┐
             │       SPEECH STATE      │
             └────────────┬────────────┘
                          │ Energy drops <= -35 dBFS for >= 50 ms
                          ▼
             ┌─────────────────────────┐
             │      SILENCE STATE      │
             └────────────┬────────────┘
                          │ Energy rises >= -32 dBFS
                          ▼
             ┌─────────────────────────┐
             │       SPEECH STATE      │
             └─────────────────────────┘
```

### 3.3 Boundary Padding & Plosive Protection
Natural human speech begins with unvoiced consonants (plosives: /p/, /t/, /k/) and ends with trailing fricatives or room decay that fall below $-35\text{ dBFS}$. Cutting exactly at the threshold clips words:
- **Pre-Cut Padding:** Retains $150\text{ ms}$ before detected speech resumes.
- **Post-Cut Padding:** Retains $150\text{ ms}$ after speech falls silent.
- **Clamping Formula:**  
  If silence interval is $[t_1, t_2]$, the proposed cut candidate is:
  $$\text{Cut Candidate} = [t_1 + \text{post\_padding},\, t_2 - \text{pre\_padding}]$$
  If $(t_2 - \text{pre\_padding}) - (t_1 + \text{post\_padding}) < 50\text{ ms}$, the cut is automatically discarded.

---

## 4. Interactive Suggestion Review Contract

Silence analysis returns a list of candidate suggestions without touching the active EDL:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceCandidate {
    pub id: uuid::Uuid,
    pub source_start_us: i64,
    pub source_end_us: i64,
    pub duration_us: i64,
    pub min_energy_dbfs: f32,
    pub is_accepted: bool, // Defaults to true; user can toggle
}
```

### Review Capabilities:
1. **Audition Cut:** Jump playhead to $1.0\text{ s}$ before cut, preview transition with cut skipped.
2. **Toggle Individual Suggestion:** Press `X` to reject/keep a specific cut without affecting other suggestions.
3. **Adjust Padding Globally:** Slider adjustments update candidate boundaries in real time on the waveform.
4. **Batch Apply:** Press "Apply Cuts" to atomically convert accepted candidates into EDL segment exclusions.
