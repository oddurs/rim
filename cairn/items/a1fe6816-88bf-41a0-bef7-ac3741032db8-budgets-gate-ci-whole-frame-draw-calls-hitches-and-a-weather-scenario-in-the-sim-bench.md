---
id: a1fe6816-88bf-41a0-bef7-ac3741032db8
title: 'Budgets gate CI: whole frame, draw calls, hitches, and a weather scenario in the sim bench'
type: perf
status: backlog
milestone: bare-metal
assignee: rapid-cloud
created: 2026-09-28
updated: 2026-09-28
priority: p0
api: none
effort: m
layer: tooling
area: perf
---

## Why

The budgets in the Bare Metal milestone mean nothing unless CI fails when they're broken. Today the sim bench has no weather, which is how 50 to 127 ms ticks from snow and mud went unseen (a708c037).

## What

- The render bench fails the queue lane over budget on the whole frame, draw calls or hitches, not only world CPU. Budgets are scaled per runner class.
- The sim bench gains a winter scenario with snow, mud and hauling, gated on mean, p99 and max tick.
- A PR run posts the numbers it moved.

## Acceptance criteria

- [ ] The queue lane fails when a planted regression breaks a frame, draw-call or hitch budget
- [ ] The sim bench's weather scenario runs in CI and fails a planted slow path search
- [ ] The budgets live in one data file that the bench and the docs both read
