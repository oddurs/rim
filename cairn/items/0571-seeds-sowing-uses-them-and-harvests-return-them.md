---
id: 571
uid: 15316b12-48be-402c-8a5f-e00999acd8b9
title: 'Seeds: sowing uses them, and harvests return them'
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

Farming (0110) sows for free, so a field costs nothing but time. Its note lists no seeds as not done.

## Acceptance criteria

- [ ] Sowing a crop uses a seed of that crop; with no seed, the cell waits
- [ ] A harvest yields seeds as well as food
- [ ] Deterministic across a save and load (test)
