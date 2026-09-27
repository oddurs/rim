---
id: 845db9fb-c3a6-4e48-8bd0-d071a578f2b4
title: rim replay --bisect and a hash per tick when platforms disagree
type: feature
status: backlog
milestone: proving-ground
created: 2026-09-27
updated: 2026-09-27
priority: p1
api: none
effort: s
layer: tooling
area: save
---

## Problem

When two runs diverge, `rim replay` reports the first logged checkpoint that disagrees (every 600 ticks), and the crosscheck prints one hash a day, so finding the tick and system that differ is done by hand. Co-op's desync work (fa6f0afb) needs the same tool.

## Proposal

- `rim replay --bisect A B` (two saves, or a save and a platform's crosscheck output) narrows to the first tick where section hashes differ and prints the sections.
- The crosscheck prints a hash per tick for the day that disagreed, when `agree` fails.

## Acceptance criteria

- [ ] A deliberately introduced platform-style divergence (a changed draw at tick N) is found at tick N with its section named (test)
- [ ] `agree`'s failure output names the first differing tick (shown)
