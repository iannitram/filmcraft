# Rules for agents in this repo (our pipeline)

Upstream's `AGENTS.md` and `CLAUDE.md` are upstream's: read them and follow them for code. These are our pipeline rules on top. Full pipeline: the owner's private pipeline repo (`PIPELINE.md`, `RULES.md`, `RUBRIC.md`); this is the short copy.

1. **Find the stage** in `PIPELINE.md`. Spec: `SPEC.md` (upstream's roadmap + our delta). Code map: `docs/code-map.md`.
2. **Work from an issue.** Take one that is not `in-progress` and not blocked; add `in-progress` before you start. Its body is your brief; an Easy issue has a "Qwen brief" section.
3. **Your own platform's issues first.** Mac, Windows and Linux all matter: say which you built on.
4. **Check that it builds**, then open the PR against `main`. CI and a review by another satellite with another model do the rest.
5. **Label the PR** `by:<machine>` and `model:<model>`, and write `Closes #N`.
6. **A failed job** gets a comment on the issue saying what happened, and the issue moves up one size.
7. **No private details** in code, commits, issues or PR text: no names, e-mail addresses, home paths, machine names, project files or footage. Issues and reports carry counts only. Commit as the no-reply identity; never add the owner to contributors, AUTHORS or a changelog credit.

## This project

- **Licence wall:** MIT/Apache. GPL code is reference only and never enters the tree. **Clean-room for `.prproj`:** learn the format only from files we own and author; no Adobe code, assets or plug-ins.
- **Upstream:** do not rebuild what upstream has open. Solid small fixes (a test that fails without them, suites green rebased on upstream `main`, nothing private) may be offered upstream and reported afterwards; features need the owner's yes. Never comment on, review or close anything upstream. Write `upstream PR 953`, not `storytold/filmcraft#953`, in our text.
- **The owner's library of original Premiere, Photoshop and After Effects projects is read-only as a whole:** work on copies; if the app wants to write beside a library file, stop and ask.
- **Logs first:** when anything fails, read the app's own log (`<data folder>/Logs/filmcraft.log`; `FILMCRAFT_DATA_DIR` sets the folder) and quote it in the issue, or say it was silent.
- **Tests:** release profile with `FILMCRAFT_NO_HARDWARE=1`; keep to fixed `-p` sets (a new mix rebuilds many crates). NVENC tests run only on the NVIDIA PC. The full test notes are in issue #66.
- **Starting `filmcraft.exe --help` on old Windows builds starts the app;** use `filmcraft-cli` for anything outside a window test.
