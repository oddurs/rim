---
id: 171
title: Port the HUD into mods/core/ui and delete the Rust HUD
type: feature
status: done
milestone: interface
depends_on:
- 169
- 170
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: none
effort: l
layer: core
area: ui
pillar:
- plugin-first
---

## Why

Dogfooding: if core's own HUD can't be written on the UI system, neither can a mod's.

## What

Top bar (clock, wealth, outdoor temperature, overlay name), colonist bar, toolbar (still generated from defs), pawn inspector, message feed as toasts, hover readout, F3 profiler panel, overlay legend, colony-lost banner. The client's HUD drawing in `draw.rs` is deleted; the client draws the world and the UI tree.

## Acceptance criteria

- [x] Every HUD part is a Luau component in `mods/core/ui/` with a namespaced id
- [x] The Rust HUD code is deleted (only world rendering remains in `draw.rs`)
- [x] The autotest drives the UI by node id, not screen coordinates, and passes on CI
- [x] Screenshots reviewed against the current HUD: nothing lost
- [x] Whole UI under 1 ms per frame with 30 colonists

## 2026-09-23

The HUD is mods/core/ui/hud.luau (top bar, clock, status, colonists, toolbar, messages, inspector, hover readout, order hint, profiler, colony lost). draw.rs keeps only the world and a draw-list renderer. The autotest now clicks UI by node id and drives world input through the same routing as the mouse. Nothing lost against the old HUD; gained: system font, crisp at 2x, need bars, drag size at the cursor, profiler shows UI time. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
