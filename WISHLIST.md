# FilmCraft: our wishlist

Our priorities for this fork, in order. **Ian owns this file.** The nightly agents read it at the start of every run and never edit it; they suggest changes in their log under "Needs Ian". Upstream's own roadmap is in the code branch (`ROADMAP.md`, `docs/roadmap.md`, `docs/gaps.md`, `docs/hardware-parity.md`, `docs/file-format-parity.md`), not here.

Use: editing videos for a YouTube channel. Source footage is mostly **SD DVD rips** (720x480 / 720x576, anamorphic 16:9 or 4:3 pixel aspect, 23.976 / 29.97, possibly interlaced or telecined) in **H.264 MP4**; new edits move to **H.265**. Delivery: YouTube (upscaled to 1080p, H.264 / H.265). Hardware: AMD Strix Halo laptop (Ryzen AI Max+ 395, 16 cores / 32 threads, Radeon 8060S iGPU with unified memory, Linux / SteamOS, Mesa RADV Vulkan, VA-API through radeonsi), plus an Apple-Silicon Mac.

*Last changed: 2026-10-12 (after the first night: more upstream PRs to build on in item 1, a speed note in item 3, and general bugs allowed under item 4; later the same day: export shape fixed and an interlaced H.264 note in item 3; then the Export dialog follow-ups, fork PRs 8 and 9).*

## 1. AMD / Strix Halo acceleration

Decode, effects, playback and export should run on the AMD GPU and its media engine (VCN), not the CPU. **H.265 export through VA-API is the most wanted piece** (there is no software HEVC encoder; only `crates/h264enc`).

- Upstream issues: #30 (hardware acceleration tracker), #632 (silent software-adapter fallback), #1216 (cap export cores), #848 / #1039 (decode thread scaling and HEVC per-core speed)
- Upstream PRs to build on, not duplicate: #408 (VA-API VP9 / AV1 decode + H.264 / H.265 encode), #409 (AV1 VA-API encode). Both conflict with main
- Also in flight upstream, so don't redo them: #1201 (rendering capabilities and fallback diagnostics, for #632), #1076 (HEVC picture threads, for #1039), #883 / #884 (H.264 SIMD, for #848)
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

- Pixel aspect: anamorphic 720x480 / 720x576 sources display and export at the right shape. **Export is fixed on `ours`** (2026-10-12, fork PR 7: `Geometry::new` in `crates/export/src/pipeline.rs` now uses the sequence's pixel aspect, Match Source keeps it, and it is written as `pasp`). Don't redo it. The Export dialog's preview and Summary line follow it since fork PR 8 (`preview_rects` in `crates/ui-egui/src/panels/export_mode.rs`, `ExportSettings::summary_for`). The dialog's Aspect drop-down follows it since fork PR 9 (`unset_par_label` in the same file, `ExportSettings::unset_pixel_aspect`): it reads "Match Source (1.19)" when a Match Source export keeps an anamorphic sequence's pixels, and "Square Pixels (1.0)" otherwise. Nothing is left open in the Export dialog for this. All of this exists only on `ours`: start from `origin/ours`, not `upstream/main`
- Interlaced and telecined sources: detect field order / telecine pattern, deinterlace and inverse telecine options
- **On hold, don't start:** `crates/h264` can't decode interlaced-coded H.264 at all (field pictures, PAFF, MBAFF: `frame_mbs_only_flag = 0` returns unsupported; see its README and `docs/gaps.md`), so such a clip imports but is unreadable. It's a multi-night decoder project and may not be needed here. Ian decides after checking the real footage
- SD -> 1080p upscaling quality for YouTube export (a GPU scaler)
- Fast SD H.264 decode and seek on long-GOP MP4. Measured 2026-10-12: SD decode is already fast on CPU (about 175 fps on one cloud core), so speed work goes to the export path first (scaling, colour conversion, `crates/h264enc`), then seek latency
- Code areas: `crates/h264`, `crates/isobmff`, `crates/media`, `crates/render` / `crates/gpu` (scaling), `crates/export`

## 4. Help upstream's goals

Upstream's ranked list (`ROADMAP.md` "Upcoming", `docs/gaps.md`): no launch crashes and CI on PRs, `.prproj`, 10-bit / 4:2:2 decode (#626, #629: low priority for us), parity measurement, hardware acceleration.

- **When no open bug fits items 1 to 3, the bug-fix pass may take general bugs that reproduce on CPU.** Order: crashes, hangs and data loss; then wrong behaviour in the editor; then test fixes that keep upstream's CI green on stock runners (old ffmpeg, no GPU, missing fonts)
