---
id: 70edf863-73a1-4a4d-b284-84e9803d2252
title: 'Save format: component-keyed, versioned, mod-aware'
type: spike
status: done
milestone: persistence
depends_on:
- be8174f0-ff41-44fe-b788-2ffff60e0d19
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
priority: p0
api: none
effort: s
layer: engine
area: save
pillar:
- plugin-first
---

## Question

What does a save look like so it survives mod updates and removals?

## Options

- Serde of typed components keyed by string
- Snapshot + command log
- Binary with a schema table

## Decision

A snapshot, not seed + command log (replays stay a separate file). The
header holds the format, engine and API versions, seed, tick and the mod
lockfile. Entities are maps of named components (`engine:pawn`,
`mood:thoughts`), with def references as qualified ids and save-local entity
numbers. Nothing derivable is saved. The Luau VM isn't saved: scripts
re-run, and state that must survive lives in script data. The encoding is
self-describing (CBOR or MessagePack, chosen by measurement in 0061) plus
zstd. The CI guarantee: save, load and continue matches the state hash of a
game that never saved. Full ruling: DESIGN.md §7a.

## Acceptance criteria

- [x] Decision recorded in DESIGN.md

Constraint from DESIGN.md §10: def and component keys are namespaced (`mod:id`, 0138), saves record every mod's version so 0139 can migrate, and the mod list is the lockfile (0152).

## 2026-09-23

From the Weather sprint (0179): saves must carry the calendar, the regime queue and blend, active ambient pushes, stock field grids (i16/i32 arrays, compress well), plant Growth components, and the wind exposure octant (or recompute it on load). Stock fields are mod-declared, so they save keyed by field id like components.
