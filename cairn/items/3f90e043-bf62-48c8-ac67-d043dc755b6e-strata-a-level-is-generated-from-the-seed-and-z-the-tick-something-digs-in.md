---
id: 3f90e043-bf62-48c8-ac67-d043dc755b6e
title: 'Strata: a level is generated from the seed and z the tick something digs in'
type: feature
status: backlog
milestone: depth
depends_on:
- 8cc6252d-67a0-4652-b764-851f3e6bc72a
- e311c029-499c-4764-a4d6-6d1f933f00f9
created: 2026-09-26
updated: 2026-09-26
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

An untouched level costs nothing only if it can be produced on demand, exactly, from the seed. Depth also has to be content: which rock is where, and why a level is worth reaching, belongs in defs and mods, not the engine (DESIGN.md §6c).

## What

- `[[stratum]]` defs: a level, a fill of solid terrains with weights and noise bands, veins, and terrain conditions such as "near surface water". The engine reads them; core owns them.
- Generation runs inside a tick, from `(seed, z)` and the engine's fixed-point noise, when a dig-down designation first reaches a level. A designation can trigger it early so the dig never waits.
- A script may take over a stratum's generation (`rim.on_generate_level(z, fn)`), as §6a allows for the surface.
- `rim.on("level_opened", fn(z))` fires once per level.
- Core content: −1 soil, clay near water, chalk with flint; −2 limestone and aquifer rock in wet biomes; −3 granite. A new core tool tag `mining`. `mods/primitive` gates −1 behind `digging` and −2 behind `pounding`. Nothing gates −3 until metal picks exist (Crafting).
- A one-cell ring of unminable bedrock at every underground level's edge: only the surface has a map edge.

## Acceptance criteria

- [ ] The same seed generates the same level whether it is opened on day 2 or day 40 (test)
- [ ] Generating a level costs under 10 ms on the reference machine, recorded here and in DESIGN.md §6c
- [ ] A Luau mod replaces −2's generation in a fixture test
- [ ] Core alone digs to −3; with `primitive`, −1 needs `digging` and −2 needs `pounding`
- [ ] Determinism test passes
