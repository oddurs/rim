---
id: ab0599d2-dae6-450a-8cf0-9b3076d331d6
title: Do room properties come from the boundary?
type: spike
status: done
milestone: building
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: s
layer: engine
area: map
---

## Question

Should a room's field behaviour derive from what encloses it, or stay a
constant on the field def?

Today `leak_per_hour = 0.06` lives on `[[field]] temperature` and applies
to every room on the map. `light` is `indoor = "none"`, so indoors is dark
except for emitters. Neither knows whether the room is walled in stone,
wood, or mostly glass.

Windows force the question. A window is not a special case to hard-code;
it is the first boundary piece whose material matters.

## Options

- **Field constants (today).** Cheapest. Windows become a hack: a flag the
  engine special-cases, which is the thing DESIGN.md exists to prevent.
- **Per-room, derived from the boundary.** Walk the room's boundary cells,
  sum each thing's contribution, cache per room and recompute when walls
  change (rooms already rebuild only then). Materials become meaningful
  everywhere, and 0188 and 0190 fall out of the same mechanism.
- **Per-cell.** Exact and expensive. Belongs with stock fields (0186) if
  ever.

## What it decides

- Whether `[[thing]]` gains boundary contributions (`leak`, `daylight`) or
  whether windows get a bespoke code path.
- Whether 0188 (wind shelter) and 0190 (drafty rooms) are content or
  engine work.

## Acceptance criteria

- [x] Cost of a boundary walk measured on a map with many small rooms
- [x] Decision recorded in DESIGN.md

## 2026-09-23

Decided: room properties come from the boundary. Measured on a 200x200 map in release, best of 6: a pass over wall cells adding each wall's contribution to every distinct adjacent room costs 0.078-0.093 ms, against a room rebuild of 0.190-0.224 ms -- about +40%. Flat from 16 to 400 huts, because the pass scales with map area rather than room count, and it runs only when rooms rebuild (walls changed), never per tick. So 0216 goes ahead as planned and windows are a real def rather than a flag. Follow-up for 0097: the pass scans all cells; narrowing it to the changed set is the same work as incremental regions. Benchmark kept as tests/boundary_spike.rs so the number can be re-measured.
