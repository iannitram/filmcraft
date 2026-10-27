# FilmCraft (our fork): spec

**Approval:** 2026-10-26, the owner: "The specs/roadmaps were developed by someone else. Nomalize them to our system" (spec = upstream roadmap + our delta, R18).

## Base: upstream's roadmap

FilmCraft is a fork of [storytold/filmcraft](https://github.com/storytold/filmcraft). Upstream's plan is the spec for everything we do not list below:

- [ROADMAP.md](https://github.com/storytold/filmcraft/blob/main/ROADMAP.md): parity with Adobe Premiere Pro, stage and headline numbers
- [docs/roadmap.md](https://github.com/storytold/filmcraft/blob/main/docs/roadmap.md): milestones and current focus
- [docs/gaps.md](https://github.com/storytold/filmcraft/blob/main/docs/gaps.md): the ranked work list (G1-G20)
- [docs/target-app-parity.md](https://github.com/storytold/filmcraft/blob/main/docs/target-app-parity.md): how parity is measured

Upstream's work is upstream's: we do not rebuild what upstream has open (check `gh pr list -R storytold/filmcraft` first). The daily fork sync brings it in.

## 1.0 for our fork

Cutting a real episode in FilmCraft, with its titles made in EffectCraft, using our builds instead of Adobe: a practice episode first, on the Linux AMD handheld (owner, 2026-10-15; acceptance in #59). *To confirm with the owner:* the exact 1.0 sentence (#59) and the 95% feature list that gates it (#46).

## Our delta

What this fork has or wants that upstream does not. "Send upstream" follows the owner's "send what's solid" rule (small fixes with a failing test; features need the owner's yes).

| # | Delta | Status | Where | Upstream? |
|---|---|---|---|---|
| D1 | **AMD / Strix Halo acceleration:** VA-API decode on Mesa radeonsi that stays in hardware (open-GOP seek, concealed pictures), deeper decode queue, zero-copy VA surface to Vulkan, more effects on the GPU, unified-memory tuning | partly built (`crates/platform`) | #47-#53 | send upstream, piecewise; help upstream's open VA-API PRs first (#49) |
| D2 | **Open our old Premiere projects and relink the footage:** the `.prproj` importer (carried from another contributor's upstream PR, authorship kept), import gaps, relink by episode number, relink across machines, media following a moved project folder | importer and folder-follow built; gaps open | #25-#27, #38-#41, #45 | importer: upstream's own PR; fixes: send upstream |
| D3 | **Round trip with Adobe:** save back to `.prproj` and report what cannot be written | not built | #57 | to confirm with the owner |
| D4 | **Live link with EffectCraft:** an EffectCraft title in a timeline re-renders when saved; one written contract both apps follow | engine side on main as work in progress | #56 | upstream-ready, offered only on the owner's word |
| D5 | **Self-logging** (`craft-log`, `craft-stage`): app and CLI log files, secret masking, diagnostics | built; Windows gaps | #64 | keep on our fork |
| D6 | **Data safety:** save, auto-save and recovery never lose newer work (FX1-FX11); export never silently overwrites | FX1-FX7, FX11 built; FX8 open | #21 and the FX8 issue | send upstream |
| D7 | **Linux desktop fixes:** Wayland window fit at fractional scale or rotated panels; drag and drop on Wayland | fit built, verify; drop open | #54, #55 | send upstream |
| D8 | **Releases through the Craft launcher:** signed, offered, never self-installed; Windows, Linux, Mac | not built for this fork | new issue | keep on our fork |
| D9 | **Tests and speed for our footage:** private-corpus ratchet, synthetic Premiere fixtures, the SD export path | not built | #58, #60 | fixtures: send upstream; corpus: keep |

## Won'ts (owner rulings)

- No work on rare old video formats (Camtasia, DivX/XviD, Sorenson 3, DV, VC-1) (2026-10-13).
- Licensed fonts are never installed or copied; free look-alikes only after the owner approves them by eye (2026-10-13).
- Nothing is relinked silently: every suggested link is confirmed by a person (2026-10-13).
- No plugin hosting work on this fork unless the owner decides (upstream's G11 says "owner decision needed").
