---
id: 213
title: Material factors on built things
type: feature
status: backlog
milestone: building
depends_on:
- 212
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: additive
effort: m
layer: engine
area: building
---

## Why

Materials have to change something, and the engine must not be the thing
that knows what. Nothing in `rim_sim` should contain the word "insulation".

## What

- `stuff.factors = { hp = 0.7, work = 0.8, insulation = 1.2, ... }` on an
  item def: named multipliers, meaning nothing to the engine.
- A stat is `base * factor` where base comes from the buildable's def and
  the factor from its `MadeOf`. Unknown names are carried, not rejected --
  a plugin may read a factor the engine never heard of.
- Readable from Luau (`rim.stat(thing, name)`) and the UI, so a mod can
  act on its own factors.
- The first concrete slice of 0078; 0078 generalises it beyond buildings.

## Acceptance criteria

- [ ] A stone wall has more hp and takes longer to build than a wooden one,
      entirely from defs
- [ ] `grep -ri insulation crates/` finds nothing
- [ ] A factor no engine code knows about survives to a script that reads it
