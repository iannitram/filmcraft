# State

## Review cursor

First code review done 2026-10-12 (run cut short by a 05:00 UTC hard stop; 4 of 8 selected PRs not reached). Next run: review 429, 1201, 1076, 884 first (not reviewed last time), then re-check 408/409/495/629 only if their head commit changed.

## Open threads

| PR | Title | Verdict (2026-10-12) | Head commit reviewed |
|---|---|---|---|
| 408 | VA-API hardware decoding + H.264/H.265 export encoding | needs work (test robustness bug + conflicts) | 41aa5b0cb9612e8d9baadb710670907d1a02852b |
| 409 | AV1 export format, VA-API on Linux | needs work (inherits 408's bug + conflicts; duplicates 408) | 39909bc6973cc8cd031beb23dcf72ca44a2e5534 |
| 495 | Premiere FX & Projects (.prproj importer) | needs work (rebase only; code itself looks solid) | 006807e35816e7631ed6dccc1e64e7b61a82acd7 |
| 629 | H.264 High 10 / 4:2:2 decode, bit-exact | needs work (unexplained eac3_oracle test failure; see Needs Ian) | 8641e7d6cf5afa7913d2faf3bf6d5bb5a4eda1e2 |
| 429 | Real-world media round-trip corpus manifest | not reviewed: out of time | b505987ffb4b394ac437e8b4b67f6bfefd6412f9 |
| 1201 | M7: rendering capabilities / fallback diagnostics | not reviewed: out of time | 70193ecb6fc145d15c1c8707d7eaa2d3f1fc87ca |
| 1076 | #1039: HEVC multi-threaded picture decode | not reviewed: out of time | 2098f2a426f5f49b890dad5ca7b4c78dae7b27e4 |
| 884 | h264: safe-SIMD weighted pred/chroma/IDCT (#848) | not reviewed: out of time | 5351d9e1762c923e63115bd4ee4783b4e062c488 |

Full findings: `nightly/2026-10-12.md` → Code review section.

## Bug-fix pass cursor

2026-10-12: looked at `upstream issue` 1171 (couldn't reproduce, needs the reporter's file), 1127 (probably fixed upstream; AVI multi-audio gap), 1088 (fixed, PR 2). Next run: don't redo these; check whether 1171 or 1127 got new comments.

## Bug-fix cursor (second run)

2026-10-12: 1171 not reproducible on Linux (likely macOS/Windows only); 1203 already covered by an engine test; 1153 waiting on the maintainer. Don't redo these.

## Design notes

(none)

## Branches

| Branch | Purpose | Status |
|---|---|---|
| fix/upstream-1088-transform-shutter | Transform shutter blur leaked into still frames / missing on GPU path (`upstream issue 1088`) | PR https://github.com/iannitram/filmcraft/pull/2 merged into fork main 2026-10-12; still needs Z13 GPU check and an upstream PR (Ian) |
| `fix/upstream-1088-shutter-blur-frame-edges` | Transform shutter GPU-path fix (`upstream issue 1088`), second run | PR https://github.com/iannitram/filmcraft/pull/3 open. **Duplicates PR 2** (PR 2 is the better fix); recommend closing |
