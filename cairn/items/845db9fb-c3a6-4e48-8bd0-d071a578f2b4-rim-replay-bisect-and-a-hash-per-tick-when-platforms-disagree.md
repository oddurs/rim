---
id: 845db9fb-c3a6-4e48-8bd0-d071a578f2b4
title: rim replay --bisect and a hash per tick when platforms disagree
type: feature
status: done
milestone: proving-ground
assignee: Oddur Sigurdsson
created: 2026-09-27
updated: 2026-09-28
closed_at: 2026-09-28
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

- [x] A deliberately introduced platform-style divergence (a changed draw at tick N) is found at tick N with its section named (test)
- [x] `agree`'s failure output names the first differing tick (shown)

## 2026-09-28

rim_sim::bisect: lockstep (two games stepped together, compared after every tick), logs (two saves of one game, log by log) and traces (two platforms' per-tick section hashes of one day). The crosscheck takes --trace-day D; when agree fails it now outputs the first differing day, a matrix job traces that day on all four platforms, and a compare job names the first tick and sections in the run summary. rim replay --bisect A B compares two saves and says which one this machine's replay agrees with. Criterion 1: tests/bisect.rs changes one draw from the spawn stream at tick 1234 in one of two games; lockstep finds tick 1234 and names engine:world, traces find 1235 (the first state after the draw), and two saves part between their logs at 1000 and 1500. A traced day costs about 110 s locally (a snapshot per tick).

## 2026-09-28

Criterion 2, from a queue run on a throwaway branch (deleted) whose crosscheck drew once more at tick 20321 on macOS only: https://github.com/oddurs/rim/actions/runs/36484543569. agree failed with 'Platforms disagree, first on day 2'; the four trace jobs replayed day 2 tick by tick, and the compare named it: 'trace-ubuntu-24.04-arm.txt parts from trace-macos-latest.txt at tick 20321: engine:world', the same for ubuntu-latest and windows-latest. An earlier run (36481863992) skipped the traces: under pipefail, diff's exit 1 on a difference ended agree before it set the day; the day is found with awk now.
