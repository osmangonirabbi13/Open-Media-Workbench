# OpenTake — Open-Source Project License Evaluation & Recommendation

**Document Version:** 1.0.0  
**Target Release:** v0.1.0 Beta  
**Status:** PROPOSED (Pending Developer Sign-Off)  
**Author:** OpenTake Open-Source Governance & Architecture Lead  
**Date:** 2026-10-13  

---

## 1. Context & Decision Drivers

Selecting an open-source license for OpenTake requires balancing five critical goals:
1. **Developer Freedom & Community Growth:** Attracting global open-source contributors without intimidating legal hurdles.
2. **Copyleft Protection:** Preventing proprietary competitors from wrapping OpenTake's core Rust silence-removal engine into closed-source commercial subscriptions without contributing back.
3. **Dependency Alignment:** Harmonizing with FFmpeg's GPL obligations when binaries are redistributed as sidecars.
4. **Patent Protection:** Protecting users and contributors from patent trolling.
5. **No Commercial Paywalls:** Keeping the core editing studio permanently free for all creators.

---

## 2. Comparative Evaluation Matrix

| License | Copyleft Strength | Contributor Appeal | Patent Protection Grant | Compatibility with Bundled GPL FFmpeg | Recommendation for OpenTake |
|---|---|---|---|---|---|
| **MIT** | Permissive (Zero copyleft). Anyone can close the code. | Very High (Universally understood). | None. Silent on software patents. | Compatible, but a proprietary vendor could take OpenTake, bundle their own FFmpeg, and sell it. | Rejected as primary license. |
| **Apache-2.0** | Permissive with explicit patent protection. | High in enterprise and tooling. | **Explicit, robust patent license & retaliation clause.** | One-way compatibility with GPLv3; cannot be used with GPLv2-only binaries. | Considered, but lacks copyleft protection against closed forks. |
| **GPL-3.0-or-later** | **Strong Copyleft.** Derivative works must remain free and open source. | High in desktop creative tools (Blender, Kdenlive, OBS Studio, Audacity). | **Explicit patent grant (Section 11).** | **100% Native Harmonization** with GPL FFmpeg and `libx264`. | **RECOMMENDED (PROPOSED).** |

---

## 3. Recommended Project License: GNU General Public License v3.0 (GPL-3.0-or-later)

### 3.1 Rationale
1. **Precedent in Creative Desktop Software:** The world's most successful free desktop creative tools—**OBS Studio, Blender, Audacity, and HandBrake**—are licensed under the GNU GPL. This establishes trust with content creators that OpenTake will never be converted into a paid commercial subscription.
2. **Total Legal Harmony with FFmpeg & libx264:** Because FFmpeg with `--enable-libx264` is governed by the GPL, licensing OpenTake under GPL-3.0-or-later eliminates legal friction when packaging desktop installers.
3. **Patent Defense:** GPLv3 Section 11 includes an explicit, irrevocable patent license granted by every contributor to every downstream user.

### 3.2 Distinction Between Source Code License and Third-Party Binaries
- **OpenTake Source Code:** Governed by **GPL-3.0-or-later**.
- **Rust Dependencies (`tokio`, `serde`, `tauri`):** Dual-licensed MIT / Apache-2.0. Permissively licensed dependencies are 100% compatible with inclusion inside a GPL project.
- **FFmpeg Binaries:** Governed by the upstream FFmpeg copyright holders under LGPL/GPL terms.

---

## 4. Third-Party Attribution & `NOTICE` File Blueprint

To ensure transparent compliance, OpenTake will maintain a top-level `NOTICE` file in STEP 26:

```text
OpenTake
Copyright (c) 2026 OpenTake Contributors

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This software includes third-party libraries and tools:
- FFmpeg (https://ffmpeg.org) licensed under LGPLv2.1+ / GPLv2+
- Tauri 2 (https://tauri.app) licensed under MIT / Apache-2.0
- Tokio (https://tokio.rs) licensed under MIT
- React (https://react.dev) licensed under MIT
```

### Action Status:
- **Project License Status:** **PROPOSED** (Awaiting formal developer authorization before generating the root `LICENSE` file).
