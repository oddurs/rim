---
id: c65db254-2e6d-4b1a-a213-a69e07762146
title: 'Per-mod budgets: profiler warnings and hard limits for runaway scripts'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-22
updated: 2026-09-22
priority: p1
api: none
effort: m
layer: engine
area: perf
pillar:
- performance
- plugin-first
---

## Why

One slow mod should be visible and throttled, not a mystery. A runaway mod (`while true do end`) must be stopped and named, not hang the game (DESIGN.md §10).

## Acceptance criteria

- [ ] Budget in ms per mod
- [ ] Warning in the profiler when exceeded
- [ ] Hard instruction limit per call via the Luau interrupt; the offending mod is named and its hook disabled
- [ ] Memory cap per mod
- [ ] Limits are counted in instructions, not wall time, so they are deterministic
