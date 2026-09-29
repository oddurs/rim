---
id: 638
uid: 77fa896d-1821-427c-a75c-4b81f85c9bc8
title: Fields deplete, and rotation restores them
type: feature
status: backlog
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
effort: m
layer: plugin
area: building
---

## Why

Farming (0110) grows the same crop on the same soil forever. Its note lists rotation and depletion as not done.

## Acceptance criteria

- [ ] Harvesting a crop lowers the cell's fertility by a per-crop amount declared in data
- [ ] A different crop, or a fallow season, restores it
- [ ] Tests with fixed conditions
