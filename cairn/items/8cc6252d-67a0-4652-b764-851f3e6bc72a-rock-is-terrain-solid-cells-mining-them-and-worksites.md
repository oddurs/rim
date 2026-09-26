---
id: 8cc6252d-67a0-4652-b764-851f3e6bc72a
title: 'Rock is terrain: solid cells, mining them, and worksites'
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

- [ ] No rock entities after map generation; the entity count on seed 1 before and after is recorded here
- [ ] Mining a solid cell yields, leaves its floor, and rebuilds regions and rooms
- [ ] Mining progress survives an interrupted worker and a save and load
- [ ] Stone-age gating still holds: the `stone_age` sweep's quarry step passes at its old rate
- [ ] Bench (`examples/bench.rs`) mean and p99 before and after, recorded here
- [ ] A room dug into rock is roofed by the rock's support span
- [ ] Determinism test passes
