---
id: 198
title: Balance a year, and prove core stands alone
type: chore
status: done
milestone: weather
depends_on:
- 57
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] First-week numbers within noise of the §4a table (40 seeds)
- [x] Winter survival with and without heating measured and recorded
- [x] Core-only runs in CI

## 2026-09-23

examples/year.rs (the climate report), balance --core / --start-day / --cold-snap / per-season and founder reports / parallel seeds, headless --core with a per-system profile. First week with weather vs core alone (80 seeds): founder froze after night one 26/80, 2.0 h vs 30/80, 3.1 h; no shelter 80/80, 10.4 h vs 40/40, 12.9 h; deaths 10/80 both. Winter (start day 38, 20 days, 40 seeds): hut only 0/40 colonies survive, heated hut 24/40. CI: core alone 3 days, and a year's climate report. Numbers and reasoning in DESIGN.md §4c.
