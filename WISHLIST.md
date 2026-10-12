# FilmCraft: our wishlist

Our priorities for this fork, in order. **Ian owns this file.** The nightly agents read it at the start of every run and never edit it; they suggest changes in their log under "Needs Ian". Upstream's own roadmap is in the code branch (`ROADMAP.md`, `docs/roadmap.md`, `docs/gaps.md`, `docs/hardware-parity.md`, `docs/file-format-parity.md`), not here.

Use: editing videos for a YouTube channel. Source footage is mostly **SD DVD rips** (720x480 / 720x576, anamorphic 16:9 or 4:3 pixel aspect, 23.976 / 29.97, possibly interlaced or telecined) in **H.264 MP4**; new edits move to **H.265**. Delivery: YouTube (upscaled to 1080p, H.264 / H.265). Hardware: AMD Strix Halo laptop (Ryzen AI Max+ 395, 16 cores / 32 threads, Radeon 8060S iGPU with unified memory, Linux / SteamOS, Mesa RADV Vulkan, VA-API through radeonsi), plus an Apple-Silicon Mac.

*Last changed: 2026-10-12 (first version).*

## 1. AMD / Strix Halo acceleration

Decode, effects, playback and export should run on the AMD GPU and its media engine (VCN), not the CPU. **H.265 export through VA-API is the most wanted piece** (there is no software HEVC encoder; only `crates/h264enc`).

- Upstream issues: #30 (hardware acceleration tracker), #632 (silent software-adapter fallback), #1216 (cap export cores), #848 / #1039 (decode thread scaling and HEVC per-core speed)
- Upstream PRs to build on, not duplicate: #408 (VA-API VP9 / AV1 decode + H.264 / H.265 encode), #409 (AV1 VA-API encode). Both conflict with main
- Code areas: `crates/platform/src/vaapi/`, `crates/platform/src/hardware_encode.rs`, `crates/platform/src/hybrid.rs`, `crates/export`, `crates/gpu`, `crates/render`
- Later: zero-copy VA surface -> DMA-BUF -> Vulkan texture (all `unsafe` stays in `crates/platform`), more effects on the GPU (Lumetri curves / wheels / HSL first), unified-memory-aware cache budgets
- No AMD GPU in the cloud: anything that needs the real GPU goes under "Needs Z13 testing" with exact steps

## 2. Open old Premiere `.prproj` projects and relink their media

Old Premiere projects (unknown Premiere versions; an inventory is being taken) must open with sequences, bins, cuts, nests, speed, transitions and basic effects, and their media must relink after moving from Windows / Mac drives to Linux paths.

- Upstream issues: #978 (open Premiere projects); `docs/gaps.md` G2 (the beta blocker)
- Upstream PR to build on: #495 ("Premiere FX & Projects", clean-room importer; conflicts with main)
- Code areas: `crates/interchange`, `crates/engine` (import commands, `media.relink`, Link Media), `crates/project`
- Wanted on top of #495: **batch relink with a path-mapping table** (e.g. `D:\Projects\` -> `/run/media/...`, `/Volumes/X/` -> ...; case-insensitive for Windows paths), fall back to search by name + duration + fingerprint; a clear report of everything that didn't import
- Ian's real projects are private and get tested on his machine. Use synthetic `.prproj` fixtures here; clean-room only (no GPL parsers, no Adobe code)

## 3. SD / DVD footage done right

- Pixel aspect: anamorphic 720x480 / 720x576 sources display and export at the right shape
- Interlaced and telecined sources: detect field order / telecine pattern, deinterlace and inverse telecine options
- SD -> 1080p upscaling quality for YouTube export (a GPU scaler)
- Fast SD H.264 decode and seek on long-GOP MP4
- Code areas: `crates/h264`, `crates/isobmff`, `crates/media`, `crates/render` / `crates/gpu` (scaling), `crates/export`

## 4. Help upstream's goals

Upstream's ranked list (`ROADMAP.md` "Upcoming", `docs/gaps.md`): no launch crashes and CI on PRs, `.prproj`, 10-bit / 4:2:2 decode (#626, #629: low priority for us), parity measurement, hardware acceleration.
