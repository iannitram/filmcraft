# State

*Tidied 2026-10-12 by a daytime session after the first night: several runs of each routine had left one cursor section per run. From here on there is **one section per routine**: add your new line at the end of your own section, and don't add new sections.*

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

**Upstream has its own PRs for three of our fixes** (checked 2026-10-12 15:55 UTC; all opened after our runs that night, all still open):

| Upstream issue | Our fix on `ours` | Upstream's PR | What happens when upstream merges it |
|---|---|---|---|
| 1241 (Select All in Effect Controls) | fork PR 4 | `upstream PR` 1271, same approach | merging `upstream/main` into `ours` conflicts in `crates/ui-egui/src/panels/effect_controls.rs` |
| 1249 (APV tests on old ffmpeg) | fork PR 5 | `upstream PR` 1257 and `upstream PR` 1314 | conflicts in `crates/apv/tests/oracle.rs` (and `docs/testing.md`) |
| 1088 (Transform shutter blur) | fork PRs 2 and 3 | `upstream PR` 1287: the same `FxCtx::frame_rate` field, but it leaves the GPU frame plan alone | conflicts in 5 files under `crates/render` and `crates/gpu` |

For the bug-fix pass: when the merge of `upstream/main` into `ours` conflicts in those files, that is this, and the prompt's rule holds (abort, push nothing, list the files under Needs Ian). A daytime session then drops our copy and takes upstream's. Of the shutter fix only the `gpu_chain` check in `crates/render/src/plan.rs` and its test are ours to keep. **Before starting any issue, list upstream's open PRs again**: other contributors fixed all three of these within hours.

Open threads in our own code (nobody is working on these; Ian decides whether a run takes one):

- `crates/h264enc`: `EncoderConfig::threads = 1` still runs on rayon's global pool, so it uses every core (found by the optimization pass; relevant to `upstream issue` 1216).
- Effect Controls: Deselect All (Cmd/Ctrl+Shift+A) still clears the Timeline clip selection instead of only the keyframes (found while fixing `upstream issue` 1241).
- `upstream issue` 1251 (a language test that needs a Japanese font): a patch is written down in `nightly/2026-10-12.md` (fourth bug-fix run) but was never built or tested.
- Export dialog: the Aspect drop-down still reads "Square Pixels (1.0)" when a Match Source export will keep an anamorphic sequence's pixels. The Summary line beside it is right since fork PR 8. Changing the label needs new translated strings.

## Bug-fix pass cursor

Write a claim here before starting an issue: `claimed: upstream issue <N>, <UTC time>`.

- 2026-10-12, run 1: looked at `upstream issue` 1171 (couldn't reproduce, needs the reporter's file), 1127 (probably fixed upstream; AVI multi-audio gap), 1088 (fixed, PR 2). Don't redo these; check whether 1171 or 1127 got new comments.
- 2026-10-12, run 2: 1171 not reproducible on Linux (likely macOS/Windows only); 1203 already covered by an engine test; 1153 waiting on the maintainer. Don't redo these.
- 2026-10-12, run 3: fixed 1241 (PR 4). Skipped because upstream PRs cover them: 1158, 1191, 1163, 1183, 1184, 1187, 1188, 1189, 1190. Skipped as features or too vague: 1176, 1205, 1117, 1186. Skipped as OS-specific: 1235, 1234, 1157, 1134, 1229, 1212.
- 2026-10-12, run 4: fixed 1249 (PR 5). 1250 is already fixed upstream by `4ccc1a8` (verified, don't redo). 1251 was not reproducible here: the sandbox checker refused hiding the system Japanese fonts. A patch is proposed in the log; Ian decides. Skipped 1244, 1246 (architectural), 1247, 1248, 1252, 1253.
- **Next run:** issues newer than 1253; recheck 1205 if it gets steps to reproduce; 1251 only if Ian approves.

## Optimization pass cursor

- 2026-10-12, run 1: SD H.264 decode, deblocking filter. Two bit-exact attempts (per-MB transpose of vertical edges; 64-bit word-swap transpose), both within noise, nothing kept. Don't retry deblock transposes. Skip anything `upstream PR` 883 / 884 (H.264 MC/IDCT SIMD) or 1076 (HEVC threads) cover.
- 2026-10-12, run 2: H.264 export encoder: SATD (8/4-wide) + fixed-width MC rows, −8.9%, kept (PR 6). Don't redo SATD 16-wide in plain Rust (already near its limit).
- **Next target in rotation:** seek / scrub latency on long-GOP MP4, then the SD->1080p scaler (CPU) and h264enc transform/quant (26% of encode).
- **Working on `crates/h264enc` again?** `ours` already has PR 6 in `dsp.rs` and `picture.rs`, and upstream doesn't. Measure against `origin/ours`, not `upstream/main`, or the baseline misses that speed-up and the PR may conflict.

## Design notes

(none)

## Branches

**Fork layout since 2026-10-12:** `main` is a clean mirror of upstream (fast-forward only). `ours` = upstream main plus our merged fixes (PRs 2 to 8). New PRs go against `ours`, never `main`.

**Work that builds on one of our own fixes** (anything touching the lines the PRs below changed) has to start from `origin/ours`: `upstream/main` doesn't have that code. Say so in the PR.

**Done locally on 2026-10-12** (a daytime session on a Windows desktop with an NVIDIA GPU, after the first night; don't redo): every merged branch below was re-read and re-tested on `ours` with `upstream/main` (0.7.0 release prep) merged in. `filmcraft-export` (111 + 1 tests), `filmcraft-h264enc` (26), `filmcraft-apv` (6, with ffmpeg 8.1.1, so the three comparisons run for real), `filmcraft-render` (189 + 3), the engine's export tests (59) and the `keyframes_ui` (16) and `export_ui` (8) UI tests all pass. Clippy from Rust 1.95 (the workspace's `rust-version`) stops on 4 lints in `crates/ui-egui` (`panels/file_dialogs.rs:134`, `panels/timeline.rs:254` and `:2399`): they are upstream's own lines, the same on `upstream/main`, and the third bug-fix run's clippy (Rust 1.97) didn't report them. `ours` was 2 commits behind `upstream/main` again by the end (the 0.7.1 release commits): the next bug-fix run merges them.

| Branch | Purpose | Status |
|---|---|---|
| `fix/upstream-1088-transform-shutter` | Transform shutter blur leaked into still frames / missing on GPU path (`upstream issue` 1088) | Merged into `ours` 2026-10-12 (PR 2). **Checked on a real GPU 2026-10-12** (exports with GPU rendering on and off, 60 fps, 360° shutter): upstream 0.6.0 drops the blur with the GPU on; `ours` blurs the move, keeps the frames at rest sharp, and its GPU and CPU exports match. **No longer an upstream candidate as it stands:** `upstream PR` 1287 covers the frame-rate half. The GPU frame plan half (`gpu_chain`, 3 lines and a test) is still only ours |
| `fix/upstream-1088-shutter-blur-frame-edges` | The second run's duplicate of the fix above | Merged into `ours` 2026-10-12 (PR 3): its only extra is a test that the frame plan and the CPU render match. Not an upstream candidate |
| `fix/upstream-1241-select-all-keyframes` | Cmd/Ctrl+A in Effect Controls selects keyframes, not Timeline clips (`upstream issue` 1241) | Merged into `ours` 2026-10-12 (PR 4). **Not an upstream candidate any more:** `upstream PR` 1271 fixes the same issue the same way |
| `fix/upstream-1249-apv-oracle-old-ffmpeg` | APV oracle tests skip instead of failing when ffmpeg < 8.0 (`upstream issue` 1249) | Merged into `ours` 2026-10-12 (PR 5). **Not an upstream candidate any more:** `upstream PR` 1257 and `upstream PR` 1314 cover it |
| `perf/h264enc-satd-mc` | h264enc: faster narrow-block SATD + fixed-width MC rows, byte-identical output | Merged into `ours` 2026-10-12 (PR 6). Re-measured on a 24-thread desktop (7 alternating pairs; another build was running, so it is noisy): median encode time −9% at 1080p and −13% at 720x480, every 720x480 pair faster, output the same size to the byte. Not yet upstream; upstream candidate |
| `fix/2026-10-12-anamorphic-export-squeezed` | Exports of anamorphic sequences came out squeezed and Match Source dropped the pixel aspect (no upstream issue) | Merged into `ours` 2026-10-12 (PR 7). Not yet upstream; upstream candidate together with the branch below |
| `fix/2026-10-12-anamorphic-export-preview` | Follow-up to PR 7: the Export dialog's preview and Summary line showed the old squeezed shape and "1.0"; AV1 (hardware-only, like H.265) keeps a matched pixel aspect in the track only | Merged into `ours` 2026-10-12 (PR 8). Branched from `ours` (it needs PR 7) |
