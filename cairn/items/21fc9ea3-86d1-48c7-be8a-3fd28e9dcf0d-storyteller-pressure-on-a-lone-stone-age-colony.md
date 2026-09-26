---
id: 21fc9ea3-86d1-48c7-be8a-3fd28e9dcf0d
title: Storyteller pressure on a lone stone-age colony
type: chore
status: backlog
milestone: colony
created: 2026-09-25
updated: 2026-09-25
priority: p1
api: none
effort: m
layer: core
area: storyteller
---

## Why

With the stone age on, a lone colonist gets the first days right: shelter by night one, an axe by day 2, a felled tree by day 3 (4bd94457). Then the colony dies anyway. Across crosscheck seeds 5, 6, 7, 9, 11 and 12, every colony was gone by day 45, and on seed 1 wolves killed the founder on day 3. The storyteller and the predators were tuned for a colonist with an axe on day one, not one knapping flint.

## What

- Measure deaths and their causes by day over a seed sweep with all shipped mods, extending `examples/stone_age.rs` or `examples/balance.rs`.
- Tune raid timing and size, and predator aggression, against the colony's real strength. The strength score should count its tools, if it doesn't already.

## Acceptance criteria

- [ ] The causes of death by day, recorded over 20 seeds
- [ ] At least half the lone colonies alive at day 30
