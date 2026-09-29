---
id: 374
uid: 835c5674-ac45-4955-a2c5-21440b551f3f
title: 'Building up: supported floors, collapse, roofs and wall walks'
type: feature
status: backlog
milestone: defense
depends_on:
- 393
- 399
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: additive
effort: l
layer: engine
area: building
pillar:
- wealth-gravity
---

## Why

Second storeys, lookouts and walks along the top of a wall for archers. It comes after digging because it needs rules digging doesn't: support, collapse and coverage (DESIGN.md §6d). It sits in Defense because wall walks are what it's first for.

## What

- Core's level range gains +1 and +2.
- One span rule, shared with §6c's roofs: a built floor at `(z+1, c)` is allowed only where `span_covered(z, c)` holds or a support stands directly below it. The span comes from the support's material (`support.span`), not from the floor. The check runs on placement and when a support is removed, only within span of the change.
- An unsupported floor falls: it drops as items into the cell below and hurts what is there. `rim.on("collapse", fn(x, y, z))`.
- `roofed(z, c) = span_covered(z, c) || solid_or_floor(z+1, c)`. The implicit roof over a room is where a second storey can go, and the floor laid there becomes its explicit roof. Roofs are drawn from the topmost storey.
- Wall walks: a floor laid along the top of a wall run.

## Acceptance criteria

- [ ] A two-storey hut builds, and removing its supporting wall drops the floor above (scene test)
- [ ] A covered room keeps snow off its cells with the weather plugin on
- [ ] Support checks cost O(span²) per change, not O(map), measured and recorded here
- [ ] Determinism test passes
