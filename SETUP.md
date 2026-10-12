# Cloud sandbox facts

From the FilmCraft setup check on 2026-10-12 (Default Cloud Environment), plus the PhotoCraft / SoundCraft probes the same night. The setup check stalled waiting on a background build, so the first nightly run should add the build and test timings here.

| | |
|---|---|
| Machine | Ubuntu 24.04, 4 cores (Xeon 2.8 GHz), 15 GB RAM, no swap, ~30 GB free disk. No GPU, no audio device (`/dev/snd` absent) |
| Rust | 1.97 preinstalled with rustup; the workspace needs >= 1.95 (`rust-version`, edition 2024) and pins no toolchain |
| System packages | Missing by default; install each run (~13 s): `sudo apt-get update && sudo apt-get install -y libasound2-dev libgtk-3-dev libudev-dev libwayland-dev libxkbcommon-dev libvulkan-dev vulkan-tools mesa-vulkan-drivers` |
| Vulkan | After the install above, `vulkaninfo` shows a software device (lavapipe, Mesa 25.2): wgpu GPU-path tests can run on the CPU |
| ffmpeg | 6.1.1 preinstalled, with libx264 and libx265: the repo's oracle tests run |
| Gates | `cargo xtask ci` = fmt --check, clippy --workspace --all-targets --release -D warnings, test --workspace --release, layers, assets, wasm. On 4 cores the full set is long: test touched crates first |
| GitHub, our fork | `git fetch` / `push` work. Branches with prefixes `claude/` **and** `fix/` both pushed. Deleting a remote branch is refused (403 from the proxy), so throwaway branches stay until Ian deletes them. Push to `main` (fast-forward) worked for PhotoCraft |
| GitHub API | `gh auth status` says the token is invalid, but `gh api` works for repos attached to the session. Unattached repos are refused ("GitHub access to this repository is not enabled for this session"). Upstream `storytold/filmcraft` is attached to the nightly routines for this reason. Fallback: in-repo docs + `git log` |
| Upstream PR heads | `git fetch upstream pull/<N>/head:pr-<N>` works |
| Waiting | Never start a long build in the background and end the turn to wait: nothing wakes an unattended run. Run builds and tests in the foreground with a long timeout (<= 600000 ms), split per package |
| Build timings | 2026-10-12 bug-fix pass (cloud CPU, release): `cargo test --release -p filmcraft-render` cold build + run 2 min 50 s; warm incremental re-test ~1 min 50 s; `cargo clippy --release -p filmcraft-render -p filmcraft-gpu --all-targets` warm 39 s; `cargo test --release -p filmcraft-gpu` 2 min 47 s (34 tests on lavapipe) |
| Issue list | `gh api 'repos/storytold/filmcraft/issues?...' --paginate` fails on page 2 (proxy refuses numeric-ID URLs): add `&page=2` by hand |
| Build timings (second run) | Cloud CPU, 2026-10-12: release build of `filmcraft-cli` from cold 6 min 01 s; `clippy -p filmcraft-render` (warm) 21 s; all `filmcraft-render` tests (release, warm) 2 min 15 s |
| Attachments | Images on github.com/user-attachments can't be downloaded from the sandbox |
