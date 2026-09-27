---
id: 1aad094f-15ce-4c28-8adb-70f048f393dd
title: Six colonists on Auto have more runs with a death than flat defaults
type: spike
status: backlog
milestone: work
created: 2026-09-26
updated: 2026-09-26
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

- [ ] Deaths per mode at 200+ seeds, with causes, recorded in DESIGN.md §4d
- [ ] Either Auto's rate is within noise of flat's, or a planner change brings it there
