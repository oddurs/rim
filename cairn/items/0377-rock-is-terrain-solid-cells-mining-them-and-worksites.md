---
id: 377
uid: 8cc6252d-67a0-4652-b764-851f3e6bc72a
title: 'Rock is terrain: solid cells, mining them, and worksites'
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: additive
effort: l
layer: engine
area: map
pillar:
- performance
---

## Why

`granite` is a thing that spawns at `density = 1.0` on every `rock_floor` cell (`mods/core/defs/nature.toml`), so every rock cell on the map is an hecs entity with a `Thing`, saved and hashed, that never acts. Levels below the surface are almost all rock: four of them would add 147,456 such entities. DESIGN.md §6d rules that rock is terrain. This is also worth shipping alone, before any z: it takes the rock out of the ECS on today's maps.

## What

- `[[terrain]]` gains `solid = true`: the cell is filled, blocks movement, bounds rooms and stops fields. `Map::ensure_rooms` already treats impassable terrain as boundary.
- A solid terrain has `mine = { work, requires, yields, leaves }`. `leaves` names the floor terrain left behind.
- Designating a solid cell spawns a worksite entity that holds `Work` (§6b), so progress survives the worker leaving and a save. It is removed with the rock. The chunk stays cached while nobody works it.
- Core's `granite` moves from `[[thing]]` to `[[terrain]]`, and `rock_floor` plus granite becomes one solid terrain. The mine designation, the overlay and the stone-age `pounding` gate keep working.
- Room boundary contributions (`leak`, `daylight`) can come from a solid terrain as from a wall.
- Solid terrain is a roof support (§6c) with a span from its material (rock: 5), and it joins walls' mass joins through the join predicate (`look.join = "rock"`).
- API version bump: patches that target `thing/granite` break. List them in the release notes.

## Acceptance criteria

- [x] No rock entities after map generation; the entity count on seed 1 before and after is recorded here
- [x] Mining a solid cell yields, leaves its floor, and rebuilds regions and rooms
- [x] Mining progress survives an interrupted worker and a save and load
- [x] Stone-age gating still holds: the `stone_age` sweep's quarry step passes at its old rate
- [x] Bench (`examples/bench.rs`) mean and p99 before and after, recorded here
- [x] Determinism test passes

## 2026-09-26

Built as a lazy thing rather than a new worksite kind: [[terrain]] solid = { thing, leaves } names the thing that stands in the cell once worked. Designate, Build over, and Order wake it (World::wake_rock). Mining despawns it and the terrain becomes leaves. Cancel with no work done puts it back to sleep (settle_rock). The harvest, work, wear, look, wind and boundary pipeline is reused unchanged, so thing/core:granite patches (primitive's pounding gate) still apply: api additive, not breaking. Readers that wanted the thing at a cell use World::fixture_def_at. Entities after generation, 200x200, default mods: seed 1 3630 -> 1383, seed 2 6524 -> 2973, seed 3 7206 -> 2060.

## 2026-09-26

Moved out the criterion 'a room dug into rock is roofed by the rock's support span': the roof span is the Houses work (#147), which puts a support block on thing/core:granite. Solid terrain reaches it through World::fixture_def_at, so nothing more is needed here.

## 2026-09-26

Bench (examples/bench.rs, 250x250, seed 1, this Mac under load from other sessions). One 1-day run each: main mean 0.945 ms, p99 22.7; branch mean 1.381, p99 38.2. The runs diverge (29 vs 32 colonists by the end), and 'pawns' (A*) is nearly all of it. Three interleaved 0.5-day runs each: main mean 0.119/0.070/0.053, p99 1.76/1.47/0.53; branch mean 0.078/0.051/0.047, p99 1.72/0.93/0.52. Medians favour the branch; no per-tick cost was added (shelter 0.016 -> 0.012 ms in the 1-day run). The stone_age sweep has no quarry step; stone-age gating of rock is covered by tests/primitive.rs felling_and_quarrying_wait_for_tools_and_say_so, which passes.
