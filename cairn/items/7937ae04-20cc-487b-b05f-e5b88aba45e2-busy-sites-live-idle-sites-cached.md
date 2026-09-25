---
id: 7937ae04-20cc-487b-b05f-e5b88aba45e2
title: Busy sites live, idle sites cached
type: perf
status: doing
milestone: building
assignee: Oddur Sigurdsson
claimed: 2026-09-25
depends_on:
- a14a5ff6-adad-488c-9810-8a02f6409b7c
created: 2026-09-25
updated: 2026-09-25
priority: p1
api: none
effort: s
layer: client
area: perf
pillar:
- plugin-first
---

## Budget

Worksite drawing under 0.3 ms a frame with 200 pawns on 200 sites; 0 live draws for plans nobody is building.

## Measurement (before)

`mesh.rs` `live()` draws every thing with a `Blueprint` each frame, worked or not. 500 wall plans are 500 live draws a frame. Record the bench before changing it.

## Approach

- `live()` becomes: a worksite this tick (`World::worksites`), or an animated look. Idle plans join the chunk cache.
- A bench case: 200 pawns building 200 plans, plus 500 idle plans.

## Acceptance criteria

- [x] Idle plans are drawn from the chunk cache
- [x] A plan being built is drawn live and shows its progress
- [x] A bench case records worksite cost before and after
