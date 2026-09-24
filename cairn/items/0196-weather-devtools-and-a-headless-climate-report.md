---
id: 196
title: Weather devtools and a headless climate report
type: chore
status: planned
milestone: weather
depends_on:
- 184
- 187
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: none
effort: s
layer: tooling
area: tests
pillar:
- plugin-first
---

## Why

Tuning a year by playing it is hopeless. Modders and we need to force weather, scrub time and read a year in seconds.

## What

- F12 devtools "Weather" tab: force any regime, scrub hour and day of year, pause the sim clock while the weather runs, a 3-day graph of every channel, and the breakdown under the cursor.
- `rim --climate-report [--seed N] [--years N]`: runs headless and writes `target/climate/*.csv` (hourly channels, daily ground means, regime spans, plant counts) and a markdown summary.
- CI runs a one-year report and uploads it as an artifact.

## Acceptance criteria

- [ ] Forcing a regime and scrubbing the hour work from devtools
- [ ] The climate report runs a year in under 30 s in release
- [ ] CI uploads the report
