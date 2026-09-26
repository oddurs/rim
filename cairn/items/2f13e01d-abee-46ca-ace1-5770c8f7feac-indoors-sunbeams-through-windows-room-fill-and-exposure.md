---
id: 2f13e01d-abee-46ca-ace1-5770c8f7feac
title: 'Indoors: sunbeams through windows, room fill, and exposure'
type: feature
status: backlog
milestone: lighting
depends_on:
- 6fd6b13b-1186-46f4-876e-743d173e03d7
- 8f4f1de8-5784-4377-8cee-25bcf223275e
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: client
area: render
---

## Why

A torch in a hut and a torch in a field should look different, and a window should visibly let the day in. This is §6c's daylight fan and warm wash, done as light. DESIGN.md §6e.

## What

- Under a roof, a sky ray passes only through a window between sill and lintel: sunbeams, long at dusk, a strip at noon, none through a north wall.
- Room fill: `0.3 · Σ(I·r²) / area` per room, flat over its cells, in each light's channel. Computed when rooms rebuild.
- A room's diffuse sky share from its boundary `daylight`, which room rebuild already sums.
- Exposure follows the sky and eases over a second; `max(sky, fire)` goes.
- Contact shadows at wall bases from four occluder reads (`medium` and up).

## Acceptance criteria

- [ ] A hut with a west window shows a beam on the floor at 18:40 and none at 12:00 (autotest screenshots)
- [ ] One brazier lights a 3×3 hut to its corners and leaves the corners of a 12×10 hall dim (screenshots)
- [ ] Room fill rebuilds only with rooms (test)

## 2026-09-26

Agreed with Houses: the window daylight fan and the fire's warm wash belong to this item. 4791e24b drops both and keeps only the gap marker and the open-sky hatch. The plan draws nothing for either, so there's no second fan to replace.
