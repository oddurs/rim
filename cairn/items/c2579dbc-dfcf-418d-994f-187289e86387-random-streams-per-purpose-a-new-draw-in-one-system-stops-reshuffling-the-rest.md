---
id: c2579dbc-dfcf-418d-994f-187289e86387
title: 'Random streams per purpose: a new draw in one system stops reshuffling the rest'
type: feature
status: backlog
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-27
priority: p0
api: additive
effort: m
layer: engine
area: sim
---

## Problem

The world has one RNG and 29 draw sites in seven files share it (terms 6, systems 5, script 5, ai 4, world 3, mapgen 3, map 3). A new draw anywhere changes every roll after it: the map's animals, a raid's target, and what every mod's `rim.random()` returns. Tests written against a seed's behaviour break for no reason (#207, #211), and installing one mod changes another's dice. DESIGN.md §7b.

## Proposal

- `rng.rs`: `World::stream(name)` returns a stream derived as `Rng::new(mix(seed ^ hash_str(name)))`, created on first use and kept in a sorted map. Streams: `spawns`, `sim`, `ai`, `weather`, `story`, and `mod:<id>` for each mod's `rim.random()` and `rim.random_int()`.
- A counter-based draw for per-entity decisions: `draw(stream, entity, tick, k)`, a hash, independent of system order. Use it where a system iterates entities and draws.
- Every stream's state is saved and loaded with the snapshot; bump the save format.
- `docs/modding/scripting.md` says each mod has its own stream.

## Acceptance criteria

- [ ] A test: adding a mod that calls `rim.random()` every tick changes nothing else in a 3-day run on core (same spawns, same AI outcomes, same state hash outside that mod's data)
- [ ] A test: an extra draw in the `ai` stream leaves the map's spawns identical
- [ ] Save, load and continue matches never saving (the existing test, still passing), and the crosscheck agrees on all four platforms
- [ ] The save format version is bumped and an old save reports a version change, not a divergence
- [ ] Determinism test passes
