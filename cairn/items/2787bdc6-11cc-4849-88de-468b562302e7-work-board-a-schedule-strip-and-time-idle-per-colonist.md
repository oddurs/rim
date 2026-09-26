---
id: 2787bdc6-11cc-4849-88de-468b562302e7
title: 'Work Board: a schedule strip and time idle per colonist'
type: feature
status: backlog
milestone: mood
depends_on:
- f1b96df4-ff22-4c7c-88b4-15bd0c6391a2
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: additive
effort: m
layer: engine
area: ai
---

## Why

DESIGN.md §4d gives each Work Board row a 24-hour schedule strip and the time a colonist has spent idle. There are no schedules in the engine yet, and idle time isn't tracked, so the board (f1b96df4) shows only the current job.

## What

- A per-colonist schedule (work, sleep, anything) by hour, set by commands; the AI honours it.
- Idle time tracked per colonist over the last day.
- The board's rows show both.

## Acceptance criteria

- [ ] A schedule changes what a colonist does at that hour (test), and survives a save
- [ ] Idle time is counted and shown on the row
