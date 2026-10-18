*Machine-drafted by Qwen 2026-10-16, reviewed and corrected by Claude 2026-10-17.*

# Goal 7 — UI fidelity

**Goal in one paragraph:** Make FilmCraft's panels, monitors and on-screen controls look and behave like
Premiere Pro 26.5.2: transform handles on the Program monitor, a polished Effect Controls with zoom bar
and divider, draggable panel docking rearrangement, filmstrip thumbnails, marquee selection, track renaming,
transition controls, and platform-specific window chrome.

## What exists

### Transform handles — `crates/ui-egui/src/panels/transform_handles.rs`
- **8 handles** (4 corners + 4 edge mids) + **anchor point** (Premiere's ⊕) — `transform_handles.rs:30-41`
- **Grab** enum (Move, Handle, Rotate, Anchor) — `transform_handles.rs:51-58`
- **Placement** struct with `Affine` transforms (outer, to_screen, quad, handles) — `transform_handles.rs:61-81`
- **Drag** struct with angle tracking for multi-turn rotation — `transform_handles.rs:86-98`
- `handle_points` / `within` / `unscale` from `graphics` module — `transform_handles.rs:24`
- `changes` function: drag logic for position, anchor, scale (uniform/off), rotation
- `resize_cursor` / `rotate_pointer` drawing
- Merge key `"programTransform"` for single-undo-step drags — `transform_handles.rs:45`
- **Automation IDs:** `program.transform.box`, `program.transform.handle.<0-7>`, `program.transform.rotate.<n>`, `program.transform.anchor` — doc header `transform_handles.rs:13-15`
- **Trigger:** selecting Motion in Effect Controls or double-clicking a clip's picture in Program monitor — doc header `transform_handles.rs:1-4`

### Effect Controls — `crates/ui-egui/src/panels/effect_controls.rs`
- **Split layout** (effect list + keyframe area) with divider — `effect_controls.rs:128-150`
- **MIN_LIST_W = 260.0**, **MIN_LANE_W = 60.0** — `effect_controls.rs:21-23`
- **Keyframe existence check** — `effect_controls.rs:57-64`
- **Select all keyframes** (⌘A) — `effect_controls.rs:68-91`
- **selected_clips** with linked audio — `effect_controls.rs:93-126`
- **Save Preset / Clear** buttons
- **custom_setup_row, param_row, graph_rows**
- **Maskable effect rows** integration
- **Pill tabs** (Source / Sequence), time ruler, playhead handle — `effect_controls.rs:24-28`
- **Value / velocity graphs** with Y curve (`Color32(0x60, 0xc0, 0x80)`) — `effect_controls.rs:29-35`
- **Keyframe navigator** (◀ ◆ ▶) with `NAV_STEP = 16.0` — `effect_controls.rs:27-33`
- **Shift-click keyframe selection** — `effect_controls_selection.rs`
- **State:** `collapsed_fx`, `effect_controls_split`, `keyframe_selection` — `state.rs`
- **Transition controls** host in Effect Controls — `effect_controls.rs:135-137`

### Docking — `crates/ui-egui/src/dock.rs`
- **28 PanelKind** variants (Project through ProjectNotes) — `dock.rs:14-43`
- **DockNode** (Split / Tabs) with `SplitSize` (Ratio / FixedA / FixedB) — `dock.rs:136-149`
- **Workspaces** serialized as trees — `dock.rs`
- **DockLayout** — `dock.rs`
- **Legacy migration** (`migrate_legacy_effects` removes duplicate Effect Controls panels)
- **saved_layout** / **saved_docking**
- **Compact panels** (Tools, AudioMeters) — `dock.rs:130-133`
- **Window shortcuts** (Shift+1..9) — `dock.rs:116-128`
- **Floating panel groups** — `dock_ui.rs`
- **`window.dockPanel` / `window.floatPanel`** actions — `dock_shared.rs`

### Monitor view — `crates/ui-egui/src/panels/monitor_view.rs`
- **Magnification:** 10%–1600% (10 levels) — `monitor_view.rs:32-33`
- **Display modes:** Composite, Alpha, Red, Green, Blue, Multi-Camera, Waveform, Comparison — `monitor_view.rs:49-50`
- **Resolution:** full / half / quarter / eighth / sixteenth — `monitor_view.rs:41-47`
- **Rulers** (16 px) — `monitor_view.rs:35`
- **Guides** (dragged out, Add/Clear/Lock, templates in preferences) — doc header `monitor_view.rs:5-7`
- **Snap** (6 px) — `monitor_view.rs:38-39`
- **Automation IDs** for zoom, settings, rulers, guides — doc header `monitor_view.rs:13-17`

### Monitor — `crates/ui-egui/src/panels/monitor.rs`
- **Source / Program** monitors with frame fetching from `FrameServer` — `monitor.rs:1-8`
- **Prefetch** in priority order during playback — `monitor.rs:4-8`
- **Multi-camera layout** with grid/video area splitting — `monitor.rs:80-230`
- **Scrubbing** with nearest cached frame fallback (no black flashes) — `monitor.rs:6-8`

### State — `crates/ui-egui/src/state.rs`
- `keyframe_selection: Vec<KeyframeRef>` — shift-click selection
- `collapsed_fx: HashSet<String>` — collapsed effect sections
- `effect_controls_split: f32` — divider ratio

## What's missing

### Monitor
| Issue | Gap | File |
|---|---|---|
| #431 | Program monitor **Button Editor (+)** does nothing | `monitor_view.rs` |

(#639, the on-screen transform handles, is done: see "What exists". `docs/gaps.md` still lists it.)

### Effect Controls
| Issue | Gap | File |
|---|---|---|
| #641, #645 | **Keyframe zoom bar** (magnifier over keyframe area) | `effect_controls.rs` |
| #448 | **Position curves** (X/Y value graphs for Position param) | `effect_controls.rs` |

(Done, though `docs/gaps.md` still lists them: the draggable divider, #643, `effect_controls.rs:20,150,187`;
row lines, #640, `effect_controls.rs:330` and `transition_controls.rs:110`.)

### Docking
| Issue | Gap | File |
|---|---|---|
| #493 | **Panel rearrangement** via drag-and-drop in dock tree (reorder tabs, move between groups) | `dock.rs` |
| #466 | **Multi-monitor floating windows** — float a panel to another monitor | `dock.rs`, `dock_ui.rs` |

### Project panel
| Issue | Gap |
|---|---|
| #578 | **Marquee selection** in Project panel |
| #579 | **Freeform bins** and **middle-mouse pan** |
| #587 | **Bins cannot be left on Windows** |

### Timeline
| Issue | Gap | File |
|---|---|---|
| #614 | **Continuous filmstrip thumbnails** on clips | Timeline panel |
| #654 | **Track renaming** | Timeline panel |

### Transitions
| Issue | Gap | File |
|---|---|---|
| #577 | Transition editing controls: the transition's own timeline in Effect Controls exists (drag to slide or resize, `transition_controls.rs:1-12`); the **A/B previews with Start/End sliders** remain | `transition_controls.rs` |

### Platform
| Issue | Gap |
|---|---|
| #637 | **Window controls** (min/max/close) on macOS |
| #457 | **UI scale** on X11 |

### Misc
| Issue | Gap |
|---|---|
| #683 | **Clicking empty space** to deselect: partly done (Timeline empty space, a text layer being edited, the Program monitor around the picture; `crates/ui-egui/tests/deselect_ui.rs`); check the other panels |
| #648, #668 | **Gap delete** in timeline |
| #653 | **Clip stretch** in timeline |
| #483 | **New tracks by dropping** |
| #680, #404 | **Delete in timeline** |
| #400 | **Wheel scroll in audio area** |
| #481 | **Audio clip mixer values cannot be typed** |
| #519, #611 | **Clipboard paste** of images/files |

## Small first steps

1. **Deselect on empty click, remaining panels** (`#683`): extend what `tests/deselect_ui.rs` covers to the panels that still keep their selection. 2–3 h.
2. **Track renaming** (`#654`): add an inline text edit on double-click in the timeline track header. 3–5 h.
3. **Panel rearrangement** (`#493`): implement drag-to-reorder in the dock tab strip (swap tabs within a group). 8–12 h.
4. **Position curves** (`#448`): extend the existing value/velocity graph rows to draw X/Y curves for Position parameters. 4–6 h.
5. **Keyframe zoom bar** (`#641`): add a magnified keyframe region above the keyframe lane, synced with scroll position. 6–10 h.
6. **Transition A/B previews** (`#577`): Start/End sliders with the A and B pictures. 4–8 h.

## Effort estimate

Total: **120–200 h** (matches `docs/gaps.md` line 167). The small first steps above are ~27–44 h, leaving 80–160 h for the remaining items (docking rearrangement, filmstrip thumbnails, marquee selection, transition controls, platform chrome).

## Related docs

- `docs/gaps.md` line 157–167: G8 definition
- `docs/ui-parity.md`: UI/UX parity table (~60% ready, breadth ~88%)
- `docs/keyboard.md`: keyboard shortcuts
- `docs/monitors.md`: monitor view options
- `crates/ui-egui/src/panels/transform_handles.rs`: on-screen handles (#639)
- `crates/ui-egui/src/panels/effect_controls.rs`: Effect Controls panel
- `crates/ui-egui/src/dock.rs`: docking system
- `crates/ui-egui/src/panels/monitor_view.rs`: monitor view options
- `crates/ui-egui/src/panels/monitor.rs`: source/program monitors
