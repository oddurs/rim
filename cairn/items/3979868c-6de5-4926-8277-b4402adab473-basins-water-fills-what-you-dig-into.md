---
id: 3979868c-6de5-4926-8277-b4402adab473
title: 'Basins: water fills what you dig into'
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- 3f90e043-bf62-48c8-ac67-d043dc755b6e
- ba8253df-9f73-4196-87ac-2924e3143627
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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
- Depth is in sevenths of a cell. What depth does to who can pass (wading, swimming, drowning, doors that hold water) is 695ef115; drawing it is 202b16c4. Core has no drains.
- `rim.on("breach", fn(x, y, z, source))`.
- Rain runoff and puddles stay with `1415721a` on the surface.

## Acceptance criteria

- [x] Digging beside the river floods the dug space to the brim (scene test)
- [x] An aquifer breach at −2 fills −3 before −2
- [x] Cost at 250 × 250 with three open levels: flooding mean and worst tick, and at rest, recorded here and in §6d
- [x] Determinism test passes; basins are iterated in id order

## 2026-09-27

Split: depth's effect on passability, rising water and drowning went to 695ef115; drawing water to 202b16c4 (after the view, 5689930d). This item is the water itself: basins, volumes, sources, falling, the front, the breach event and the cost.

## 2026-09-27

Cost (examples/flood.rs, 250 × 250, three 120 × 120 halls joined by holes, breached into a river at −1; measured at load average 100+, so upper bounds): flooding to full took 28,274 ticks at 0.0005 ms mean; at rest ~0.1 µs a tick; the breach tick (top level rebuilt) 3.1 ms; a dig beside the flooded hall 0.59 ms min, 1.57 ms median of 30. First build of all four levels 7–90 ms by load. Rings used a BTreeSet per walk and overlap a BTreeMap per cell (122–396 ms a level under load); a visit stamp and per-basin lists took that out. A level below reads only whether cells above are air or pouring water (ground_rev), so mining rock to floor at −1 doesn't rebuild −2.
