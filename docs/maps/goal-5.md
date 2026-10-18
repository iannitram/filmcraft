*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

# Goal 5 — Real-world media corpus and workflow acceptance tests

> Maps G6 from `docs/gaps.md` (line 120–136): "No real-world media corpus or workflow acceptance tests."

## 1. Goal summary

Build a **pinned, sha256-verified corpus** of ≥ 50 real-world media files (phone, camera, screen
recordings, damaged files, odd containers, multi-track audio) and run **end-to-end import → edit →
export round-trip tests** against ffprobe / ffmpeg oracles, gated in CI (nightly).

Done when:
- A corpus fetcher (`cargo xtask corpus`) downloads files pinned by URL + sha256, verifies them,
  and places them in `<workspace>/` `target/fixtures/corpus/` (new, never committed).
- ≥ 50 files covering the categories below.
- Workflow tests import each file, apply edits (extract, trim, gain, title, dissolve, colour
  matte), export, and verify streams / frame counts / durations / frame placement / A-V sync /
  pixel values / audio levels via oracles.
- A nightly CI job runs the full corpus + workflows.

## 2. Existing code we build on

| What | Where | What it does today |
|------|-------|--------------------|
| Workflow acceptance tests | `apps/filmcraft-cli/tests/workflows.rs` | Two scripted MCP sessions: a phone-like VFR H.264 + AAC MP4 and a camera-like long-GOP H.264 Matroska with two audio tracks (AAC, AC-3). Each imports, edits (extract, audio gain, title, ripple trims, colour matte, cross dissolve), exports H.264 MP4, saves a project that a new process exports as ProRes, and checks streams, frame count, duration, frame placement, A/V sync, title pixels, dissolve mix, audio levels. |
| Fixture generation utilities | `crates/testkit/src/fixtures.rs` | `fixtures_root()`, `fixtures_dir(sub)`, `temp_path(out)`, `generate(out, make)` for race-free on-the-fly generation. Reports `FIXTURE <made|cached|skipped> <name>`. |
| CLI test harness | `apps/filmcraft-cli/tests/workflows.rs:69–87` | `Scratch` struct for per-test temp dirs; `Mcp` struct for JSON-RPC over stdio to `filmcraft-cli mcp`. |
| Oracle helpers | `apps/filmcraft-cli/tests/workflows.rs:162–249` | `probe()`, `streams()`, `frames()`, `diff()`, `assert_shows()`, `pcm()`, `onsets()`, `rms()`, `tone()` — all use ffprobe / ffmpeg as external oracles. |
| `cargo xtask fixtures` | `xtask/src/main.rs:406–456` | Runs codec conformance fixture generators (h264, hevc, isobmff, matroska, prores, dnx, mpeg2v, ac3, mpa, ogg) via ignored `generate_fixtures` tests. |
| `cargo xtask ci` | `xtask/src/main.rs:458–466` | Runs fmt, clippy, test, layers, assets, wasm. Does not yet run corpus or workflow acceptance tests. |
| G6 definition | `docs/gaps.md:120–136` | Documents done/missing/done-when for the corpus + workflow tests. |

## 3. Concrete gaps — code paths that must change

### 3.1 No pinned corpus fetcher

**Gap:** Fixtures in `workflows.rs` are generated at test time via ffmpeg (`phone_vfr_mp4`,
`camera_long_gop_mkv`, lines 30–66). No mechanism to fetch pre-recorded files from a URL and
verify sha256.

**New file needed:** `xtask/src/corpus.rs` (new) — a `corpus()` command that:
- Reads a manifest `xtask/fixtures/corpus-manifest.toml` listing files by URL + sha256 + category.
- Downloads missing files to `<workspace>/target/fixtures/corpus/` (new) or `$FILMCRAFT_CORPUS_DIR`.
- Verifies sha256 on every download.
- Reports `CORPUS <downloaded|verified|skipped> <name>`.

**Reference:** PhotoCraft's `cargo xtask corpus` (craftrules `standards/test-corpora.md`).

### 3.2 Corpus manifest does not exist

**Gap:** `xtask/fixtures/corpus-manifest.toml` (new) must list ≥ 50 files with:
- `url = "https://..."`
- `sha256 = "..."`
- `category = "phone|camera|screen|damaged|container|audio|other"`
- `description = "..."`

**Categories to cover:**
- **phone** — iPhone HEVC/H.264, Android VP9/WebM, variable frame rate clips
- **camera** — long-GOP H.264, ProRes RAW, multi-angle shoots
- **screen** — OBS WebM, desktop recordings with alpha, screen-capture VP9
- **damaged** — truncated MP4, corrupted Matroska, partial downloads
- **container** — AVI, FLV, OGG, M2TS, MXF, 3GP
- **audio** — multi-track (5.1, 7.1), hidden subtitle streams, non-standard sample rates
- **other** — odd codecs (Theora, Dirac), unusual aspect ratios, interlaced

### 3.3 Workflow tests do not use the corpus

**Gap:** `workflows.rs` generates its own fixtures. Tests must be extended to accept corpus file
paths and run round-trips on real files, not just generated ones.

**Changes needed:**
- `apps/filmcraft-cli/tests/workflows.rs` — add `corpus_roundtrip(name, corpus_file, expected)`
  test function that imports a corpus file, applies edits, exports, and verifies with oracles.
- Parameterize existing tests to run on both generated and corpus fixtures.

### 3.4 No CI gating for corpus + workflows

**Gap:** `xtask/src/main.rs:ci()` (line 458) does not call a corpus download or workflow tests.

**Changes needed:**
- `xtask/src/main.rs` — add `corpus()` function called before `ci()`, or a separate `nightly()`
  function that runs corpus + workflows.
- `.github/workflows/` — add a nightly workflow (or extend existing) that runs `cargo xtask
  corpus` then `cargo test -p filmcraft-cli --test workflows`.

### 3.5 Oracle coverage gaps

**Gap:** Current oracles check streams, frame count, duration, frame placement, A/V sync, title
pixels, dissolve mix, audio levels. Missing:
- Interlaced field order verification.
- Alpha channel preservation.
- Multi-track audio routing (which track maps to which output channel).
- Damaged file graceful error handling (not a crash).

**Changes needed:**
- `apps/filmcraft-cli/tests/workflows.rs` — add oracle functions for interlaced fields, alpha
  compositing, multi-track routing.
- Ensure `filmcraft-cli mcp` returns structured errors for damaged files (no panics).

## 4. Small first steps (3–6)

1. **Create `xtask/fixtures/corpus-manifest.toml` (new)** — start with 5–10 files from public
   sources (e.g., sample-videos.com, Big Buck Bunny, public domain camera originals) with
   verified sha256 sums. Categories: phone (2), camera (2), damaged (1).

2. **Add `cargo xtask corpus` to `xtask/src/main.rs`** — read the manifest, download missing
   files, verify sha256, place in `target/fixtures/corpus/` (new). Reuse `generate()` pattern from
   `crates/testkit/src/fixtures.rs` for atomic writes.

3. **Extend `workflows.rs` with a corpus test** — add `#[test] fn corpus_phone_vfr()` that
   imports a corpus phone clip, applies extract + title + export, and verifies with existing
   oracles. Run locally with `FILMCRAFT_CORPUS_DIR` pointing to a manual download.

4. **Add nightly CI job** — `.github/workflows/nightly-corpus.yml` (new) that runs `cargo xtask
   corpus` then `cargo test -p filmcraft-cli --test workflows --release`. Skip on PRs; run
   nightly at 02:00 UTC.

5. **Expand corpus to ≥ 50 files** — collect real-world files from contributors, verify sha256,
   add to manifest. Cover all categories in §3.2.

6. **Add missing oracles** — interlaced field order, alpha preservation, multi-track routing
   checks in `workflows.rs:162–249`.

## 5. Risks and mitigations

- **Corpus availability:** URLs rot. Pin by commit in a dedicated corpus-hosting repo or use
  sha256-pinned mirrors. Fallback: regenerate from ffmpeg if download fails (with a warning).
- **License compliance:** Every corpus file needs an `.attribution` sidecar (rule 1.3). Prefer
  public domain / CC0 / creator-donated files. No Adobe recordings.
- **Test flakiness:** Corpus tests are slower. Run them only in nightly CI, not on every PR.
  Use `#[ignore]` until corpus is available locally.
- **Damaged file safety:** Ensure no panics on corrupt input (rule 0). Test damaged files
  separately to verify graceful error returns.

## 6. Acceptance metrics

| Metric | Target | How measured |
|--------|--------|--------------|
| Corpus file count | ≥ 50 | `cargo xtask corpus --count` |
| Corpus sha256 verification | 100% pass | `cargo xtask corpus --verify` |
| Workflow round-trips passing | ≥ 90% of corpus files | `cargo test -p filmcraft-cli --test workflows` |
| CI nightly green | 5 consecutive nights | `.github/workflows/nightly-corpus.yml` |
| No panics on damaged files | 0 crashes | Corpus includes ≥ 5 damaged files |
