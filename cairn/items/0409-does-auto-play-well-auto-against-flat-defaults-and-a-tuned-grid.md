---
id: 409
uid: c59d67ed-0603-43d0-ad4e-4baf62c646b2
title: Does Auto play well? Auto against flat defaults and a tuned grid
type: spike
status: done
milestone: work
assignee: Oddur Sigurdsson
depends_on:
- 401
- 431
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
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

- [x] The table is in DESIGN.md §4d
- [x] Auto loses no more colonies than `flat`, or the planner is changed until it doesn't
- [x] Churn is under one change per colonist per in-game day

## 2026-09-26

Auto passes: colonies lost 4/80 against flat's 5 (castaway) and 0/80 each with six; churn 0.91 and 0.58 a colonist-day after two planner changes (rest by skill alone; stickiness for colonists already on a job). Open: six colonists on Auto had 29/80 runs with a death against 24 flat, about one standard deviation; filed as 1aad094f. Idle hours were identical across modes (9.7, 11.3), which says the bot's opening, not priorities, sets idle time.
