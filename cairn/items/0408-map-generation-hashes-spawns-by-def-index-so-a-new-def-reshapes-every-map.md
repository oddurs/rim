---
id: 408
uid: c3d18fe7-f13e-4b1d-a104-cbd3db4481dd
title: Map generation hashes spawns by def index, so a new def reshapes every map
type: bug
status: done
milestone: scale
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: s
layer: engine
area: map
---

## What happens

`mapgen.rs` seeds each spawn's hash with the def's index in `defs.things` (`seed ^ mix(di + 77)`). Adding any thing def ahead of the wild ones shifts every later index, so every seed generates a different map. Found adding core's pillar (Houses, 24100bb9): with it in buildings.toml, seed 3's founder started boxed in by granite and `a_campfire_of_gathered_branches_needs_no_axe` failed.

## What should happen

A map is a function of the seed and the spawning defs, not of unrelated defs' load order: hash by the def's qualified id.

## Reproduction

Seed: 3
Mods: core

1. Add any `[[thing]]` without `spawn` to mods/core/defs/buildings.toml.
2. `Sim::new(mods, 3)`: the founder's surroundings differ.

## Notes

Changing the hash reshapes every seed once, so re-run the balance and stone-age sweeps with it. Depth (rock is terrain) touches mapgen; coordinate.

## 2026-09-27

Spawn hashes are salted by mix(hash_str(qualified id)), precomputed once per def: FNV-1a, the same in every build. tests/spawn_keys.rs puts a non-spawning def ahead of a spawning one and checks the spawn cells don't move; it fails without the fix. It reshapes every map once. Seeds 1-20, main then branch: stone age 95/95/95/85% then 100/95/95/95%; balance, 5 days: lost 1 then 0, runs with a death 0 then 1, near-misses 6 then 15, first threat day 2.94 then 3.02. Two client autotest scenes that leaned on the old map around the founder were made map-robust.

## 2026-09-27

Merged in #214; closed in the Proving ground planning PR's cairn cleanup (2026-09-27), since the item was left at doing/review after its merge.
