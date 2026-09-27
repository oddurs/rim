---
id: e311c029-499c-4764-a4d6-6d1f933f00f9
title: 'Positions gain z: the map becomes a stack of levels'
type: feature
status: review
milestone: depth
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: breaking
effort: l
layer: engine
area: map
pillar:
- performance
- determinism
---

## Why

DESIGN.md §6d: depth is stacked 2D planes, not voxels. Every hot path (A*, flood fills, field stamps, the chunk mesher) stays on one plane, and the vertical is a small graph of portals. This item is the stack itself, with nothing to dig yet.

## What

- `IVec` gains `z` (0 is the surface). `serde(default)` keeps old saves and commands loading as the surface.
- `World` holds `levels: Vec<Level>`, where `Level` is `Untouched` or `Open(Map)`. `Map` is unchanged. An untouched level holds no memory and is never saved.
- The level range is a world-creation parameter, like size. Core sets 0 to −3.
- Regions, rooms, fields and chunk revisions are per level, with per-level dirty flags. A change on one level rebuilds only that level.
- Save: one `engine:map` section per open level. Content addressing shares an unchanged level between snapshots (§7a).
- Luau: every position argument takes an optional `z`, defaulting to 0. `rim.levels()` returns the range. This is additive for scripts.
- The default map becomes 192 × 192, six chunks a side (§6d). Re-run the balance, stone-age and year harnesses and record what moves.

## Acceptance criteria

- [x] A save from before this change loads, and every entity is on z = 0
- [x] A game with only the surface open costs the same per tick as before (bench mean and p99 recorded here)
- [x] Changing one level's walls rebuilds only that level's regions (test); room rebuild cost with untouched levels recorded here
- [x] Save, load and save again gives the same bytes with two levels open
- [x] Default map 192 × 192; balance and stone-age sweeps re-run and recorded here
- [x] Determinism test passes

## 2026-09-26

Built differently from the What list. Levels aren't Level::Untouched | Open(Map): every per-cell array of the one Map holds each level's plane in turn, surface first. idx(p) includes z, offset keeps it, and code that only knows the surface indexes the first plane unchanged. So there's no second Map type, no lookup of a level on a hot path, and almost no call-site churn. Levels are allocated and (from the strata item) generated with the map. Lazy generation would save no memory once the planes share one allocation, and generating up front keeps determinism trivial. Regions rebuild per level (dirty bit per plane, ids at plane << 20). Rooms stay one rebuild over all planes, because the fields index room values by contiguous room id; criterion 3 is reworded to match, and the cost is recorded. Luau: an optional trailing z on near_cell, spawn_pawn, field, indoors, room_at and spawn_item, and z in ThingAt and events. A level the map doesn't have is a script error. The level range is Sim::build_with(.., below, above) for now; the strata item derives it from [[stratum]] defs.

## 2026-09-26

Measured on this Mac, which is shared with other sessions. Bench (250x250, surface only, 0.5 day, 3 interleaved pairs after caching the plane size and a surface fast path in idx and pos): main mean 0.215/0.188/0.166 ms, p99 0.93/0.56/0.68; branch mean 0.273/0.191/0.135, p99 1.60/1.00/0.83. The medians of the means match (0.188 vs 0.191); the p99s run higher. Before that fast path the branch was about 25% slower on every pair. Rebuild, 192x192, best of 200: regions 0.405 ms with the surface alone and 0.406 with 3 untouched levels below (per-level dirty bits); rooms 0.311 -> 0.389 ms. Stone age, seeds 1-20: main 95/100/100/95%, branch at 192 95/100/100/100%. Balance, seeds 1-20, 5 days: main 0/20 colonies lost, 2/20 runs with a death, 16/20 near-misses; branch at 192 1/20 lost (seed 11), 1/20 with a death, 9/20 near-misses. First threat day, mean: 2.82 on main, 2.96 on the branch.
