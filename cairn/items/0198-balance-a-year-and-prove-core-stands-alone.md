---
id: 198
title: Balance a year, and prove core stands alone
type: chore
status: planned
milestone: weather
depends_on:
- 57
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

Seasons change the temperature every need and incident was tuned on. The first week must play as it does today, and the first winter should be hard but survivable with preparation. And "remove every plugin and the game still works" should be tested, not hoped.

## What

- `examples/year.rs`: runs a year headless and prints daily temperature, weather and warmth hours (the climate report).
- Extend `examples/balance.rs` with `--days`, reporting per season.
- A test and CI step that load core alone and play a few days.
- Tables and reasoning recorded in DESIGN.md §4c.

## Acceptance criteria

- [ ] First-week numbers within noise of the §4a table (40 seeds)
- [ ] Winter survival with and without heating measured and recorded
- [ ] Core-only runs in CI
