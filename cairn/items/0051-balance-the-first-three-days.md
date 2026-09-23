---
id: 51
title: Balance the first three days
type: spike
status: done
milestone: castaway
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-22
closed_at: 2026-09-22
priority: p1
api: additive
effort: s
layer: core
area: storyteller
pillar:
- survival
---

## Question

Is the first hour tense but fair for a lone unarmed warrior?

## Options

- Longer grace period
- Weaker early predators
- Starting knowledge: berries visible

## Decision


## Acceptance criteria

- [x] Play 5 seeds to day 3 and note deaths and near-misses
- [x] Adjust need rates, animal stats and grace period
- [x] Decision recorded in DESIGN.md

## 2026-09-22

Decision recorded in DESIGN.md §4b. Built examples/balance.rs (bot plays the opening on N seeds through Commands only) and examples/duel.rs (N vs M fight symmetry). Baseline, 20 seeds: 9 colonies lost, 16/20 runs with a death, food never below 23%. Root causes: fights to the death; retreat only worked for raiders (43 colonist deaths vs 1 raider in 100 1v1 duels). Added creature retreat_below + plural (defs), Job::Leave, a mercy rule (no new targets that are wounded or retreating, no chasing), defend radius 7->20, rim.leave_after + pawn_left event (API additive), POINTS_PER_RAIDER 55->80, predators 70->90, raids leave after 0.6 days. After, 40 seeds: 1 lost, 5/40 with a death, 28/40 near-misses. Need rates and the 2-day grace period left unchanged on the data.
