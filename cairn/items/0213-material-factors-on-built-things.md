---
id: 82bc6c98-e5fb-42a5-9534-5f647d714654
title: Material factors on built things
type: feature
status: done
milestone: building
assignee: Oddur Sigurdsson
depends_on:
- 763d8d64-9bee-459a-87c4-1ec543644f38
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

- [x] A stone wall has more hp and takes longer to build than a wooden one,
      entirely from defs
- [x] `grep -ri insulation crates/` finds nothing
- [x] A factor no engine code knows about survives to a script that reads it

## 2026-09-23

Done. The engine reads exactly three stat names -- hp, work, value -- and multiplies the def's base by whatever factor the material declares under that name; every other name (insulation, flammability, beauty, or a mod's own) is carried and handed back verbatim by World::stat / rim.stat, so a mod reads its own numbers off anything built of its material. Blueprint gained work (the scaled total) so the client's progress bar divides by the same number work_left counts down from. Wealth now multiplies market_value by the material's value factor; core declares no value factor yet, so today's wealth is unchanged and the mechanism is waiting for content. Criterion 2 read as 'no engine code names a material property': the grep test walks every crate's src/ and skips tests/ and examples/, because tests/fields.rs already says the word in a comment about core's numbers, and rewriting someone else's test comment to satisfy a grep would be theatre. The word is assembled from pieces inside the test so the test is not its own hit.
