---
id: 459ce1eb-7261-4e32-b4af-55fcd9a3f9a5
title: A load lays no water on the map until the next water pass, then every basin reads as rising
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
layer: engine
area: save
---

## What

Water reaches the map only every `WATER_EVERY` ticks. `World::apply_water` calls `Water::costs`, which puts each basin's step cost on `Map::water_cost` and footing, and notes whether it rose since last time (`Basin::rising`, `last_volume`). None of that is saved:
- `Snapshot::restore` rebuilds the map with no water cost, and `Water::restore` rebuilds basins with `last_volume = 0` and `rising = false`.
- `Water::stale` (cells of rebuilt basins still carrying a cost) is lost too.

## How it fails

The loaded game diverges from the one that kept running:
- Until the next multiple of `WATER_EVERY`, every flooded cell costs nothing and has footing. Deep water is walkable, and regions and paths go through it.
- A pawn in water past wading doesn't flee: `rising` is false.
- At the first `apply_water`, `last_volume` is 0, so every wet basin reads as rising. Any pawn standing in still, wading-deep water flees, where the live game's doesn't.

Most saves aren't on a multiple of 60, so most loads with water hit this. Found by the save/load review sweep. The water code came in with quiet-field's depth work (#283, #294, #296).

## Reproduce

In tests/basins.rs, from `water_is_saved_by_volume`: save at a tick with `tick % WATER_EVERY != 0` once a basin holds water past swimming depth. Then compare `map.passable` over its cells, and `water.rising`, live against loaded.

## Acceptance

- [x] A loaded game has the live game's water cost, footing, rising and pending stale cells, at any save tick
- [x] Older saves without the new data still load
- [x] A test that fails before the fix and passes after
