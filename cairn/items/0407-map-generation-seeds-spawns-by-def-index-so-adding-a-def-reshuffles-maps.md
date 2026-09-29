---
id: 407
uid: c2b1b623-28e5-4dcd-b72d-c4f5f993f4ed
title: Map generation seeds spawns by def index, so adding a def reshuffles maps
type: bug
status: done
milestone: crafting
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: s
layer: engine
area: map
---

## Why

`mapgen.rs` seeds each spawnable thing with `hash2_f(x, y, seed ^ mix(di + 77))`, where `di` is the def's index in `defs.things`. Indices follow load order and file names, so any def loaded before a spawnable one (a new file sorting before `wild.toml`, a mod loading earlier) shifts every later index and changes every map for every seed. Found while adding primitive's storage buildings: the crosscheck scenario on seed 1 lost its reachable flint and stopped making a hand axe, and `shelter`'s enclosed-room test lost its open ground. The storage file was renamed to sort after `wild.toml` as a workaround.

## What

- Seed each spawnable thing by a stable key, such as a hash of its qualified id, so adding or reordering defs leaves other things' spawns alone.
- Re-tune anything that depended on today's maps (the stone-age sweep, seed-specific tests) in the same change, since every map will move once.

## Acceptance criteria

- [x] Adding a def before a spawnable thing leaves that thing's spawn cells unchanged (test)
- [x] The stone-age sweep still meets its targets
- [x] Determinism test passes

## 2026-09-27

Fixed by #214 (fedc9dc8): each spawning def's pattern is salted by its qualified id, so adding a def no longer reshapes maps. primitive's yard.toml no longer needs to sort after wild.toml; its comment saying so is removed.
