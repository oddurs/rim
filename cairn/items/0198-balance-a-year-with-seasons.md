---
id: 198
title: 'Balance: a year with seasons'
type: chore
status: planned
milestone: weather
depends_on:
- 57
- 187
- 189
- 191
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: core
area: needs
pillar:
- survival
- growth
---

## Why

Seasons change the temperature every need and incident was tuned on. The first week must play as it does today, and the first winter should be hard but survivable with preparation.

## What

- Extend `examples/balance.rs` to run a full year and report per season: hours at zero warmth, deaths, food on hand, berry supply.
- Tune until: the first week matches today's numbers within noise; a colony with a heated hut and a food store survives winter in most seeds; one without either struggles visibly.
- Record the tables and reasoning in DESIGN.md §4c, like §4a and §4b.

## Acceptance criteria

- [ ] First-week numbers within noise of the §4a table (40 seeds)
- [ ] Winter survival with and without preparation measured on 40 seeds and recorded
- [ ] Tuning recorded in DESIGN.md §4c
