---
id: e311c029-499c-4764-a4d6-6d1f933f00f9
title: 'Positions gain z: the map becomes a stack of levels'
type: feature
status: backlog
milestone: depth
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

- [ ] A save from before this change loads, and every entity is on z = 0
- [ ] A game with only the surface open costs the same per tick as before (bench mean and p99 recorded here)
- [ ] Changing one level's walls rebuilds only that level's regions and rooms (test)
- [ ] Save, load and save again gives the same bytes with two levels open
- [ ] Default map 192 × 192; balance and stone-age sweeps re-run and recorded here
- [ ] Determinism test passes
