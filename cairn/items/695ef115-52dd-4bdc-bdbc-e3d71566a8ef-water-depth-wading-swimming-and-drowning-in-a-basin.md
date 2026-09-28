---
id: 695ef115-52dd-4bdc-bdbc-e3d71566a8ef
title: 'Water depth: wading, swimming and drowning in a basin'
type: feature
status: backlog
milestone: depth
depends_on:
- 3979868c-6de5-4926-8277-b4402adab473
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: additive
effort: m
layer: engine
area: sim
---

## Why

Split from 3979868c (basins): water that fills a mine is only a hazard, or a moat, once its depth changes who can pass (DESIGN.md §6d).

## What

- `[[fluid]]` def: wade, swim and no-air depths in sevenths.
- Passability changes only when a basin crosses one, and that level's regions rebuild at most once every 60 ticks while it rises. Colonists leave rising water, and no air drowns.
- Doors pass water unless `holds_water`.

## Acceptance criteria

- [ ] A flooded trench blocks non-swimmers (scene test)
- [ ] Colonists leave water that is rising past wading depth; nobody drowns in the scene test
- [ ] Determinism test passes
