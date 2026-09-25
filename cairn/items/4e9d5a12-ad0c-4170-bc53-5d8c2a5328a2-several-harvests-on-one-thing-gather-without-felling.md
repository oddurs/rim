---
id: 4e9d5a12-ad0c-4170-bc53-5d8c2a5328a2
title: 'Several harvests on one thing: gather without felling'
type: feature
status: backlog
milestone: stone-age
created: 2026-09-24
updated: 2026-09-25
priority: p0
api: additive
effort: s
layer: engine
area: sim
---

## Why

The early game takes branches from a standing tree, and later fells it for wood. Today `ThingDef::harvest` is `Option<HarvestDef>` (`crates/rim_sim/src/defs.rs:164`), so a thing has exactly one designation: `tree_oak` can be chopped or gathered, never both.

Everything else this needs already works. `berry_bush` (`mods/core/defs/nature.toml:19`) is `destroy = false, regrow_days = 2.0`, and `systems::regrow` implements it, so renewable gathering is proven. It just cannot coexist with felling. This is the keystone of the stone age: without it there is no "use your hands first".

## What

- `harvest` accepts one table or a list, so `harvest = { ... }` still parses and `harvest = [{ designation = "gather", ... }, { designation = "chop", ... }]` is new. A one-or-many deserializer keeps the change additive: no existing def is edited.
- A designation resolves to at most one harvest per thing. Two entries with the same designation is a load-time conflict, reported like any other (§6).
- Work, yields, `destroy`, `regrow_days` and `requires` (the tool gate, 7016d86b) stay per entry: gathering branches regrows, chopping destroys.
- `Regrow` readiness is per entry, so gathering branches doesn't block a later chop, and a chop still removes the tree.
- A patch can add an entry to another mod's list without replacing it (`harvest += [...]` or the patch system's list append), so the stone age adds gathering to `core:tree_oak` without owning it.
- Saves: `engine:regrow` keys readiness by entry.

## Budget

No per-tick cost. One extra lookup when a designation is placed, which happens at player rate, not tick rate.

## Acceptance criteria

- [ ] `harvest` parses as one table or a list, existing defs untouched
- [ ] Two harvests with the same designation is a reported load conflict
- [ ] A tree can be gathered (regrows) and chopped (destroys) in one def
- [ ] Regrow readiness is per harvest entry, and survives a save and load
- [ ] A patch appends a harvest entry to another mod's thing
- [ ] Determinism test passes
