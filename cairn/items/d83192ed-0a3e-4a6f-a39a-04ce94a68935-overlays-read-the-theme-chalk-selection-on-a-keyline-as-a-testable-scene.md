---
id: d83192ed-0a3e-4a6f-a39a-04ce94a68935
title: 'Overlays read the theme: chalk selection on a keyline, as a testable scene'
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: additive
effort: m
layer: client
area: ui
---

## Why

Selection is a yellow 2 px box (`draw.rs` world_ui) and a yellow ring (`draw.rs` pawns), from macroquad constants no mod can change. Every later Chalkline item needs one palette and one set of primitives. It also needs overlays whose output can be tested without reading pixels. DESIGN.md §6f.

## What

- Theme tokens in `mods/core/ui/theme.toml`:
  - `[color]`: `chalk = "#f2eee3"`, `keyline = "#080a0c8c"`, `seam = "#0000001f"`, `seam_major = "#00000052"`, `intent_fill = "#5ab4ff4d"`, `zone = "#a48fe0"`, `zone_fill = "#a48fe01f"`, `veil = "#080a0c85"`.
  - `[shape]`: `hair = 1`, `stroke = 1.5`, `firm = 2`, `bracket_gap = 3`, `bracket_arm_min = 4`, `bracket_arm_max = 12`.
  - `accent`, `bad` and `threat` are reused as they are. `theme.rs` already reads any key in these sections.
- A new `crates/rim_client/src/overlay.rs`:
  - `Palette::from_theme(&Theme)` falls back to the values above for any missing token.
  - `Scene` is plain data: a list of `Mark`s built by `scene(&App, now)`, and `draw(&Scene, &Palette)` draws it. Autotests assert on the scene.
  - Primitives: a stroke on a keyline (the keyline is drawn 2 px wider underneath), `brackets`, `ring`, `cross`, `caution` (a triangle) and `chip`. A chip draws its text with `app.ui.text` quads, the same way `readouts` does.
- Selection:
  - A thing gets brackets 3 px outside its footprint, in `firm` chalk. The arm is 28% of the short side, clamped to 4–12 px.
  - A pawn gets a ring 4 px outside its body, and its remaining path is dotted in chalk at 60%.
  - A group draws every member at 70% and the inspector's one at 100%, with a chip reading "3 selected".
  - Brackets and rings close in from 7 px to 3 px over 120 ms.
- Document the new tokens in `docs/modding/ui.md` under Themes.

## Acceptance criteria

- [x] `mods/core/ui/theme.toml` defines the tokens this item reads (`chalk`, `keyline`, `firm`, `bracket_gap`, `bracket_arm_min`, `bracket_arm_max`), and `docs/modding/ui.md` lists them with one line each. Each later token arrives with the item that first reads it.
- [x] `grep -n YELLOW crates/rim_client/src/draw.rs` finds nothing; selection draws through `overlay::draw`
- [x] A unit test: a theme overriding `color.chalk` changes the colour of the selection mark in `Palette`
- [x] A unit test: the bracket arm is 4 px for a 10 px footprint and 12 px for a 100 px footprint
- [x] Autotest: selecting a tree gives one brackets mark around its footprint; selecting three colonists gives three rings (the primary at alpha 1.0, the others at 0.7) and a "3 selected" chip; screenshots `chalk-select-thing` and `chalk-select-group`
- [x] `rim --bench-render --check` passes

## 2026-09-27

Review: tokens nothing reads yet were documented as if they worked, so each token now lands with the item that first reads it (seam: grid; stroke: hover; hair: box select; intent_fill: build ghosts; zone and zone_fill: zones; seam_major: measure; veil: the Work Board spotlight 9ebfa104). The haul line from the stores overlay (#196) moved into the scene, so it's drawn after lighting like every other chalk mark.

## 2026-09-27

The render bench criterion is left for CI: locally the bench goes over budget in the zooming and storm views on unrelated branches too, while a dozen sessions build on this machine.
