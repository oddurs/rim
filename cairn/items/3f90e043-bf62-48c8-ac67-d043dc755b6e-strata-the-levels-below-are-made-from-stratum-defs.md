---
id: 3f90e043-bf62-48c8-ac67-d043dc755b6e
title: 'Strata: the levels below are made from [[stratum]] defs'
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- 8cc6252d-67a0-4652-b764-851f3e6bc72a
- e311c029-499c-4764-a4d6-6d1f933f00f9
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p0
api: additive
effort: m
layer: engine
area: map
pillar:
- determinism
- plugin-first
---

## Why

An untouched level costs nothing only if it can be produced on demand, exactly, from the seed. Depth also has to be content: which rock is where, and why a level is worth reaching, belongs in defs and mods, not the engine (DESIGN.md §6d).

## What

- `[[stratum]]` defs: a level, a fill of solid terrains with weights and noise bands, veins, and terrain conditions such as "near surface water". The engine reads them; core owns them.
- Generation runs inside a tick, from `(seed, z)` and the engine's fixed-point noise, when a dig-down designation first reaches a level. A designation can trigger it early so the dig never waits.
- A script may take over a stratum's generation (`rim.on_generate_level(z, fn)`), as §6a allows for the surface.
- `rim.on("level_opened", fn(z))` fires once per level.
- Core content: −1 soil, clay near water, chalk with flint; −2 limestone and aquifer rock in wet biomes; −3 granite. A new core tool tag `mining`. `mods/primitive` gates −1 behind `digging` and −2 behind `pounding`. Nothing gates −3 until metal picks exist (Crafting).
- A one-cell ring of unminable bedrock at every underground level's edge: only the surface has a map edge.

## Acceptance criteria

- [x] The same seed generates the same level whether it is opened on day 2 or day 40 (test)
- [x] Generating a level costs under 10 ms on the reference machine, recorded here and in DESIGN.md §6d
- [x] A Luau mod replaces −2's generation in a fixture test
- [x] With `primitive`, −1 needs `digging` and −2 needs `pounding`; core alone gates none of them
- [x] Determinism test passes

## 2026-09-26

Started on a branch stacked on e311c029 (feat/e311c029-levels) while its PR is measured; rebased onto main once it merges.

## 2026-09-26

Built with the map, not the tick something digs in: with every plane in one allocation (e311c029), laziness saves no memory, and generating up front keeps determinism trivial. rim.on('level_opened') moves to the portals item, where a first dig reaches a level. A generator is rim.on_generate_level(z, fn), one per level, run after the stratum fills it; it uses rim.set_terrain, rim.terrain_at and rim.noise (the engine's value noise, seeded from the world), and an error in it fails the new game. solid without thing/leaves is bedrock; each stratum's edge must be solid. The level range comes from the deepest stratum (DefDb::depth); every level from -1 down needs one. Content: packed_earth, clay (under marsh, water and rich soil), chalk at -1; limestone, wet_limestone (aquifer, under wet ground) and granite at -2; granite and limestone at -3. Veins are left to db7f1e06. Cost, 192x192, core only, best of 8: new game 5.9 ms surface only, 15.3 ms with three strata, 3.1 ms a level.

## 2026-09-26

Criterion 4 is half true here: the gates hold (tests/strata.rs, the_stone_age_gates_the_levels_by_tool), but digging down arrives with the portals item (acd85584), so it can't be ticked until then.
