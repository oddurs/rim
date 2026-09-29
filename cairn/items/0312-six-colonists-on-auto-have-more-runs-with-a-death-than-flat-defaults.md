---
id: 312
uid: 1aad094f-15ce-4c28-8adb-70f048f393dd
title: Six colonists on Auto have more runs with a death than flat defaults
type: spike
status: done
milestone: work
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: m
layer: core
area: ai
---

## Why

The balance run for c59d67ed (80 seeds, 8 days, six colonists) had 29/80 runs with a death on Auto against 24 flat and 23 tuned. Colonies lost were equal (none). That's about one standard deviation, so it may be noise. It may also be Auto spreading colonists over more ground (each on the job they fit best) when a threat arrives, where flat defaults keep them near the same work.

## What

- Rerun at 200+ seeds, all three modes, and record where and how each death happened (threat, distance from the colony centre, job at the time).
- If Auto is worse, find the planner change that closes the gap without losing its churn and coverage gains.

## Acceptance criteria

- [x] Deaths per mode at 200+ seeds, with causes, recorded in DESIGN.md §4d
- [x] Either Auto's rate is within noise of flat's, or a planner change brings it there

## 2026-09-27

200 seeds, six colonists: runs with a death Auto 63, flat 57, tuned 60; under one standard deviation. Causes and distances match across modes (wolves most; 22-23 cells out; mostly idle or harvesting). No planner change. The harness now reports deaths by cause and by job, with the mean distance from home.
