---
id: a519d507-a1de-42eb-ac50-314a1dc93f50
title: Storyteller takes offered beats within one tension budget
type: feature
status: backlog
milestone: story
depends_on:
- fddc451d-17f9-4c84-ac60-a6e756f91782
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: core
area: storyteller
pillar:
- growth
- wealth-gravity
---

## Why

Threads never fire events; they offer them, and the storyteller paces offers
and its own incidents with one budget (DESIGN.md §4g).

## Acceptance criteria

- [ ] `storyteller.offer({ incident =, weight =, expires =, source = })`
- [ ] Offers and incidents draw from one tension budget
- [ ] An offer can be declined or expire; the thread hears which
- [ ] Five threads offering raids don't raise the raid rate (a test)
