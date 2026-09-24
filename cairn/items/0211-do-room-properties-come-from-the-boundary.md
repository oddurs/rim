---
id: 211
title: Do room properties come from the boundary?
type: spike
status: backlog
milestone: building
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

- [ ] Cost of a boundary walk measured on a map with many small rooms
- [ ] Decision recorded in DESIGN.md
