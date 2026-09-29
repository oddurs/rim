---
id: 199
uid: 1415721a-a7b6-4c3c-87ff-9269ec49b883
title: 'Water flow: runoff, puddles and floods'
type: feature
status: backlog
milestone: scale
depends_on:
- 186
created: 2026-09-23
updated: 2026-09-26
priority: p3
api: additive
effort: l
layer: engine
area: sim
pillar:
- performance
---

## Why

Stock fields have no lateral flow (DESIGN.md §4c, "should water flow?"). Water that runs downhill would bring puddles in hollows, rivers that flood in a spring melt, and drainage ditches worth digging.

## What

- Elevation kept from map generation as a terrain-independent grid.
- A flow step for stock fields that declare `flow = { ... }`: stable with staggered updates, or run on the native tier (WASM).
- Floods as a script incident that raises water levels along rivers.

## Acceptance criteria

- [ ] Water gathers in low ground after heavy rain
- [ ] Stable under staggered updates at 6× speed
- [ ] Cost at 250×250 within the §8 budget, recorded here

## 2026-09-26

DESIGN.md §6d (Depth) splits water in two. Flooding of dug space is basins, an engine mechanism with the water table and aquifers in core (3979868c-6de5-4926-8277-b4402adab473). This item stays rain runoff and puddles on the surface, as a stock-field plugin.
