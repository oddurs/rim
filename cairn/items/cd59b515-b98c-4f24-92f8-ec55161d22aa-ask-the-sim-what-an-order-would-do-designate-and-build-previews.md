---
id: cd59b515-b98c-4f24-92f8-ec55161d22aa
title: 'Ask the sim what an order would do: designate and build previews'
type: feature
status: planned
milestone: chalkline
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: none
effort: m
layer: engine
area: building
---

## Why

A drag preview is only worth drawing if it matches what the order does. `command::apply` decides which things a Designate marks (it checks `harvest_for`, respects `Planned`, and wakes rock) and which cells a Build plans (it clears natural things, stays off water and skips impassable cells). If the client copies those rules, the copy will drift. DESIGN.md §6f, "who decides what a preview shows".

## What

- In `crates/rim_sim/src/command.rs`, two read-only functions over `&World`:
  - `designate_preview(w, designation, a, b) -> Vec<Target>`, where `Target` is `Thing(Entity)` or `Rock(IVec)` (rock that would be woken to take the mark) or `Creature(Entity)`. It lists only what the order would newly mark.
  - `build_preview(w, thing, stuff, a, b, facing) -> Vec<(IVec, Place)>`, where `Place` is `Open`, `Clears(Entity)` (a natural fixture, or a `Rock(IVec)` cell to be mined first) or `Blocked(Blocker)`. `Blocker` is `Water`, `OutOfBounds`, `Occupied(Entity)` or `NoMaterial`.
- `apply` for `Designate` and `Build` is rewritten to walk the preview and act on each entry, so the two can't disagree.
- No content ids. No change to `Command`, saves or determinism.

## Acceptance criteria

- [ ] Neither function takes `&mut World`, and a test calls both on a world and checks its hash is unchanged
- [ ] A test over seeds 1–20, with random rectangles and every designation: the things marked by applying the order are exactly the preview's targets
- [ ] A test over seeds 1–20, with random rectangles and a wall and a 1×2 bed at each facing: the cells planned are exactly the `Open` and `Clears` cells
- [ ] A cell of deep water is `Blocked(Water)`, a tree is `Clears(tree)`, and a built wall is `Blocked(Occupied(wall))` (unit tests)
- [ ] `cargo test -p rim_sim` passes, including the determinism test
