---
id: 171
title: Port the HUD into mods/core/ui and delete the Rust HUD
type: feature
status: planned
milestone: interface
depends_on:
- 169
- 170
created: 2026-09-23
updated: 2026-09-23
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

- [ ] Every HUD part is a Luau component in `mods/core/ui/` with a namespaced id
- [ ] The Rust HUD code is deleted (only world rendering remains in `draw.rs`)
- [ ] The autotest drives the UI by node id, not screen coordinates, and passes on CI
- [ ] Screenshots reviewed against the current HUD: nothing lost
- [ ] Whole UI under 1 ms per frame with 30 colonists
