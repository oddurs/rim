---
id: 3979868c-6de5-4926-8277-b4402adab473
title: 'Basins: water fills what you dig into'
type: feature
status: backlog
milestone: depth
depends_on:
- 3f90e043-bf62-48c8-ac67-d043dc755b6e
- ba8253df-9f73-4196-87ac-2924e3143627
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: additive
effort: l
layer: engine
area: sim
pillar:
- performance
- determinism
---

## Why

Dig next to a river and the space floods; breach an aquifer and the levels below fill. DESIGN.md §6d compares two prototypes (192 × 192 × 5, one breached mine of 13,276 dug cells). Per-cell flow hadn't filled it after 200,000 ticks, and on a whole level it cost 0.40 ms a tick and settled 62% full in a slope. Volume per basin filled it in 332 ticks at 0.0014 ms a tick, and costs 9 ns a tick at rest.

## What

- A basin is a connected open area on one level, derived with the regions and rebuilt with them. Only volumes are saved.
- Surface water terrain is the water table: static, infinite, never simulated. Open space dug next to it floods. Aquifer rock seeps at its def's rate once a face is exposed.
- Water falls first, through air and stairwells, into the basin below until that one is full. It never climbs.
- A wet front grows one ring a tick from where water entered. Keep a per-basin list of holes: the prototype's worst tick (0.31 ms) was a map scan to seed the front.
- `[[fluid]]` def: wade, swim and no-air depths in sevenths. Passability changes only when a basin crosses one, and that level's regions rebuild at most once every 60 ticks while it rises. Colonists leave rising water, and no air drowns.
- Doors pass water unless `holds_water`. Core has no drains.
- `rim.on("breach", fn(x, y, z, source))`.
- Rain runoff and puddles stay with `1415721a` on the surface.

## Acceptance criteria

- [ ] Digging beside the river floods the dug space to the brim, and a flooded trench blocks non-swimmers (scene test)
- [ ] An aquifer breach at −2 fills −3 before −2
- [ ] Cost at 250 × 250 with three open levels: flooding mean and worst tick, and at rest, recorded here and in §6d
- [ ] Colonists leave water that is rising past wading depth; nobody drowns in the scene test
- [ ] Determinism test passes; basins are iterated in id order
