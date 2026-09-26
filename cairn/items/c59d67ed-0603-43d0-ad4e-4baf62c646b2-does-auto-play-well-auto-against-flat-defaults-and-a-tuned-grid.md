---
id: c59d67ed-0603-43d0-ad4e-4baf62c646b2
title: Does Auto play well? Auto against flat defaults and a tuned grid
type: spike
status: backlog
milestone: work
depends_on:
- bbc59ec3-10da-4c12-8e05-45f20dac8c75
- f5295002-f21b-4d73-a37a-308f49dc2487
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: tooling
area: tests
---

## Why

Auto is the default for every new player, so it has to be good, not merely plausible. DESIGN.md §4d records balance runs as tables; Auto needs one before it ships as the default.

## What

- The headless harness gains a priorities mode: `auto`, `flat` (every work type at the default) and `tuned` (a hand-written grid per start).
- 80 seeds, a castaway start and a six-colonist start, 8 days: colonies lost, runs with a death, day the shelter stands, idle hours per colonist-day, and planned-level changes per colonist-day (churn).
- The results go in DESIGN.md §4d as a table, with a ruling on whether Auto ships as the default.

## Acceptance criteria

- [ ] The table is in DESIGN.md §4d
- [ ] Auto loses no more colonies than `flat`, or the planner is changed until it doesn't
- [ ] Churn is under one change per colonist per in-game day
