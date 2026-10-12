# State

## Review cursor

Second code review run done 2026-10-12 (finished within budget, ~15 min used). Reviewed the 4 PRs the first run didn't reach (429, 1076, 884 are ready; 1201 needs a rebase). 408/409/495/629 unchanged since the first run, carried forward without re-review. Next run: re-check any of the 8 whose head commit changed; if all 8 are unchanged, there's nothing to re-review — consider scanning the wider open-PR list again for new WISHLIST-area matches (checked once this run, nothing beat the current 8).

## Open threads

| PR | Title | Verdict (2026-10-12, 2nd review) | Head commit reviewed |
|---|---|---|---|
| 408 | VA-API hardware decoding + H.264/H.265 export encoding | needs work (test robustness bug + conflicts) — carried forward, not re-reviewed | 41aa5b0cb9612e8d9baadb710670907d1a02852b |
| 409 | AV1 export format, VA-API on Linux | needs work (inherits 408's bug + conflicts; duplicates 408) — carried forward, not re-reviewed | 39909bc6973cc8cd031beb23dcf72ca44a2e5534 |
| 495 | Premiere FX & Projects (.prproj importer) | needs work (rebase only; code itself looks solid) — carried forward, not re-reviewed | 006807e35816e7631ed6dccc1e64e7b61a82acd7 |
| 629 | H.264 High 10 / 4:2:2 decode, bit-exact | needs work (unexplained eac3_oracle test failure; see Needs Ian) — carried forward, not re-reviewed | 8641e7d6cf5afa7913d2faf3bf6d5bb5a4eda1e2 |
| 429 | Real-world media round-trip corpus manifest | **ready** | b505987ffb4b394ac437e8b4b67f6bfefd6412f9 |
| 1201 | M7: rendering capabilities / fallback diagnostics | needs work (rebase: conflicts in export/lib.rs, hardware_encode.rs, several i18n .tsv, ui-egui/lib.rs; code itself careful and well-tested) | 70193ecb6fc145d15c1c8707d7eaa2d3f1fc87ca |
| 1076 | #1039: HEVC multi-threaded picture decode | **ready** | 2098f2a426f5f49b890dad5ca7b4c78dae7b27e4 |
| 884 | h264: safe-SIMD weighted pred/chroma/IDCT (#848) | **ready** | 5351d9e1762c923e63115bd4ee4783b4e062c488 |

Full findings: `nightly/2026-10-12.md` → Code review and Code review (second run) sections.

## Bug-fix pass cursor

2026-10-12: looked at `upstream issue` 1171 (couldn't reproduce, needs the reporter's file), 1127 (probably fixed upstream; AVI multi-audio gap), 1088 (fixed, PR 2). Next run: don't redo these; check whether 1171 or 1127 got new comments.

## Bug-fix cursor (second run)

2026-10-12: 1171 not reproducible on Linux (likely macOS/Windows only); 1203 already covered by an engine test; 1153 waiting on the maintainer. Don't redo these.

## Optimization pass cursor

2026-10-12: SD H.264 decode, deblocking filter. Two bit-exact attempts (per-MB transpose of vertical edges; 64-bit word-swap transpose), both within noise, nothing kept. Don't retry deblock transposes. Next target in rotation: **H.264 export path incl. SD->1080p scaling with pixel-aspect correction** (CPU scaler / colour conversion / h264enc), then seek / scrub latency on long-GOP MP4. Skip anything `upstream PR` 883 / 884 (H.264 MC/IDCT SIMD) or 1076 (HEVC threads) cover.
## Bug-fix cursor (third run)

2026-10-12: fixed 1241 (PR 4). Skipped because upstream PRs cover them: 1158, 1191, 1163, 1183, 1184, 1187, 1188, 1189, 1190. Skipped as features or too vague: 1176, 1205, 1117, 1186. Skipped as OS-specific: 1235, 1234, 1157, 1134, 1229, 1212. Next run: start from issues newer than 1243, or recheck 1205 if it gets steps to reproduce.

## Bug-fix cursor (fourth run)

2026-10-12: fixed 1249 (PR 5). 1250 is already fixed upstream by `4ccc1a8` (verified, don't redo). 1251 was not reproducible here: the sandbox checker refused hiding the system Japanese fonts. A patch is proposed in the log; Ian decides. Skipped 1244, 1246 (architectural), 1247, 1248, 1252, 1253. Next run: issues newer than 1253, or 1251 if Ian approves.

## Design notes

(none)

## Branches

| Branch | Purpose | Status |
|---|---|---|
| fix/upstream-1088-transform-shutter | Transform shutter blur leaked into still frames / missing on GPU path (`upstream issue 1088`) | Merged into fork main 2026-10-12 (PR 2 = fix, PR 3 = extra GPU-plan test). Not yet upstream, so the nightly fast-forward sync will skip until upstream has it or main is merged instead |
| `fix/upstream-1088-shutter-blur-frame-edges` | Transform shutter GPU-path fix (`upstream issue 1088`), second run | Merged into fork main 2026-10-12 (PR 2 = fix, PR 3 = extra GPU-plan test). Not yet upstream, so the nightly fast-forward sync will skip until upstream has it or main is merged instead |
| `fix/upstream-1241-select-all-keyframes` | Cmd/Ctrl+A in Effect Controls selects keyframes, not Timeline clips (`upstream issue` 1241) | Merged into fork main 2026-10-12 (PR 4, merge commit a6e27e7). Not yet upstream; upstream candidate |
| `fix/upstream-1249-apv-oracle-old-ffmpeg` | APV oracle tests skip instead of failing when ffmpeg < 8.0 (`upstream issue` 1249) | PR 5 open on the fork, ready to merge; upstream candidate |
