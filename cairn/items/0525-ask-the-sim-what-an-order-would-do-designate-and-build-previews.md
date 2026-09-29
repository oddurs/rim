---
id: 525
uid: cd59b515-b98c-4f24-92f8-ec55161d22aa
title: 'Ask the sim what an order would do: designate and build previews'
type: feature
status: done
milestone: chalkline
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
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
  - `build_preview(w, thing, stuff, a, b, facing) -> Vec<(IVec, Place)>`, where `Place` is `Open`, `Clears(Target)` (a natural fixture, or a `Rock(IVec)` cell to be mined first) or `Blocked(Blocker)`. `Blocker` is `Terrain(IVec)`, `Solid(IVec)`, `Occupied(Entity)`, `OutOfBounds`, `Overlap`, `NoMaterial` or `NotBuildable`.
- `apply` for `Designate` and `Build` is rewritten to walk the preview and act on each entry, so the two can't disagree.
- No content ids. No change to `Command`, saves or determinism.

## Acceptance criteria

- [x] Neither function takes `&mut World`, and a test calls both on a world and checks its hash is unchanged
- [x] A test over seeds 1–20, with random rectangles and every designation: the things marked by applying the order are exactly the preview's targets
- [x] A test over seeds 1–20, with random rectangles and a wall and the 2×1 woodpile at each facing: the cells planned are exactly the `Open` and `Clears` cells
- [x] A cell of deep water is `Blocked(Terrain(cell))`, a tree is `Clears(tree)`, and a built wall is `Blocked(Occupied(wall))` (unit tests)
- [x] `cargo test -p rim_sim` passes, including the determinism test

## 2026-09-27

Blocker names no content: water is Terrain(cell), and the client shows the terrain's label. Core has no 1×2 buildable yet, so the facing test uses primitive's 2×1 woodpile. A plan placed earlier in the same order covers its footprint for later cells (Overlap), which is what apply did implicitly. Two behaviour changes, both only when a cell is refused: rock that blocks a plan is no longer woken and left standing, and grass in a multi-cell anchor is no longer removed when the footprint can't fit. A differential test (apply_does_what_it_did_before_the_previews) runs the old apply beside the new one over seeds 1–10 and finds no difference with the shipped mods.

## 2026-09-27

Rebased over #223 (plan a building bigger than a cell over grass, trees and rock). build_preview now mirrors World::plan_footprint for a multi-cell thing: every footprint cell on the map and open, or holding a natural thing it can clear that no other plan has claimed; Clears names the first thing in the way, and the rest of the footprint is marked with it (Planned.at). apply calls plan_footprint for each anchor the preview accepts. Cancel keeps #223's unplan.

## 2026-09-27

Rebased over #229 (house plans placed whole): Build and PlacePlan both walk build_preview now, and one function, realize, carries out each place it accepts; build_at is gone. A check that decides whether a cell can be built (such as #225's dig check) belongs in build_preview, so the preview and every order agree.
