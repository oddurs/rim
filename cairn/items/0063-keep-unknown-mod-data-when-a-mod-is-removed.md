---
id: b4ad855e-8a8a-49ca-8d2e-b7bc8eb2859a
title: Keep a removed mod's data, and give it back when the mod returns
type: feature
status: done
milestone: persistence
assignee: Oddur Sigurdsson
depends_on:
- d0524477-919e-4bd6-952e-95ff0d6bb58d
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
priority: p1
api: none
effort: m
layer: engine
area: save
pillar:
- plugin-first
---

## Why

Removing a mod must not corrupt a save; re-adding it should restore its data.

## What

At the epoch boundary (DESIGN.md §7a), the sections of a mod that is no
longer installed are parked, untouched, in the new root, and written into
every later snapshot. When the mod comes back, they are unparked. Entities
whose def came from the removed mod are dropped, and the load report lists
them.

## Acceptance criteria

- [x] A removed mod's sections are parked and survive further saves
- [x] Re-adding the mod restores its data
- [x] Entities of a removed mod's defs are dropped and listed in a load report

## 2026-09-24

Snapshot::restore_noting maps every def id across a change of defs by qualified id (Remap over engine:defs), dropping what's gone and noting it: entities whose thing or creature def is gone, blueprints needing a removed material, MadeOf/Designated/needs/carried items that no longer resolve, pending events; terrain from a removed mod becomes the first terrain. Field state is matched by field id. Hooks switched off for running away are only restored under the same scripts. The plain restore() refuses to drop anything. A removed mod's script data needs no parking mechanism: it stays in world.data, keyed by its prefix, so every later snapshot carries it as that mod's section and the mod finds it when it returns. LoadReport.dropped lists the notes.

## 2026-09-24

Review: several drops went unnoted (materials, designations, needs, carried items, pending events), so a plain restore could drop them quietly; each is now noted. And room values are indexed by room id, which a removed mod's walls renumber: under other defs the fields carry values over cell by cell on the first update, as after any room rebuild. Test builds a ring of a mod's walls and a ring of core walls made of the mod's material, removes the mod, and checks both notes and that the surviving room keeps its value (it took its neighbour's before).
