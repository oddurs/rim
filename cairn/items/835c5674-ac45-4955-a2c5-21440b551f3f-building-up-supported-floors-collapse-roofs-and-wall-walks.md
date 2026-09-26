---
id: 835c5674-ac45-4955-a2c5-21440b551f3f
title: 'Building up: supported floors, collapse, roofs and wall walks'
type: feature
status: backlog
milestone: defense
depends_on:
- acd85584-7f3d-4348-8d56-5242a0bdb620
- ba8253df-9f73-4196-87ac-2924e3143627
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

Second storeys, lookouts and walks along the top of a wall for archers. It comes after digging because it needs rules digging doesn't: support, collapse and coverage (DESIGN.md §6c). It sits in Defense because wall walks are what it's first for.

## What

- Core's level range gains +1 and +2.
- A floor over air needs a wall or rock directly below, or a supported floor within its def's `span` (wood 2, stone 3). The check runs on placement and when a support is removed, only within span of the change.
- An unsupported floor falls: it drops as items into the cell below and hurts what is there. `rim.on("collapse", fn(x, y, z))`.
- A cell is covered when the cell above has a floor or is solid. Rain, snow and daylight respect coverage, so §4's explicit roof is a floor on the level above. Shelter stays enclosure.
- Wall walks: a floor laid along the top of a wall run.

## Acceptance criteria

- [ ] A two-storey hut builds, and removing its supporting wall drops the floor above (scene test)
- [ ] A covered room keeps snow off its cells with the weather plugin on
- [ ] Support checks cost O(span²) per change, not O(map), measured and recorded here
- [ ] Determinism test passes
