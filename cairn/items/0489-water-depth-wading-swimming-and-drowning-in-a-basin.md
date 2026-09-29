---
id: 489
uid: 695ef115-52dd-4bdc-bdbc-e3d71566a8ef
title: 'Water depth: wading, swimming and drowning in a basin'
type: feature
status: done
milestone: depth
assignee: Oddur Sigurdsson
depends_on:
- 330
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] A flooded trench blocks non-swimmers (scene test)
- [x] Colonists leave water that is rising past wading depth; nobody drowns in the scene test
- [x] Determinism test passes

## 2026-09-27

Built: [[fluid]] (core:water: wade 2, swim 4, no air 7, drown 2.0 an hour); water costs applied every 60 ticks (WATER_EVERY), which is also the most often regions rebuild for it; deep water takes footing unless a floor or span is over it; pawns in rising water past wading flee to dry ground on their level or the one above; drowning at no air; holds_water doors. Criterion 1 is met by a flooded tunnel rather than a trench: a trench is air, which nobody walks, so flooding it changes nothing until swimmers exist (3fea3b3d).
