# Changelog (our fork)

Newest first. One line per change of stage, release or decision that the next session needs. Upstream's history is upstream's.

- 2026-10-28 Workspace `unsafe_code` is now `deny` instead of `forbid`, matching `crates/platform` (whose FFI modules already opt in); outside that crate the single allowed function is `attach_parent_console` on Windows; owner ruling "forbid -> deny + one allow" (2026-10-28).
- 2026-10-26 Delta marks approved by the owner as written; "to confirm" notes removed 1.0 sentence set; .prproj round trip (#57) after 1.0.
- 2026-10-26 Normalized to pipeline v2 at Dev Loop: SPEC.md (upstream roadmap + our delta, R18), docs/code-map.md, AGENTS.fork.md, PIPELINE.md; open issues sized; missing build issues filed.
- 2026-10-26 Upstream main merged (2 commits). The `ours` branch is gone: `main` now carries upstream plus our work, and pull requests go against `main` (issue #66 still describes the old `ours` layout).
- 2026-10-19 Data-safety fixes FX1-FX7 and FX11 merged; `--version` prints to the terminal on Windows.
- 2026-10-15 Owner set the fork's two goals (AMD / Strix Halo; open the old Premiere projects and relink) and "done = real episode work" (issue #66).
