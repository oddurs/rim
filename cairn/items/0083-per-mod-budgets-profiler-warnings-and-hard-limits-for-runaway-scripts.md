---
id: c65db254-2e6d-4b1a-a213-a69e07762146
title: 'Per-mod budgets: profiler warnings and hard limits for runaway scripts'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-22
updated: 2026-09-24
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

- [x] Budget in ms per mod
- [x] Warning in the profiler when exceeded
- [x] Hard instruction limit per call via the Luau interrupt; the offending mod is named and its hook disabled
- [ ] Memory cap per mod
- [x] Limits are counted in instructions, not wall time, so they are deterministic

## 2026-09-24

Done: a counted step budget per call (100M interrupts, #19) stops a runaway, names the mod, and now switches that hook or handler off for the game; a mod whose calls average over 0.5 ms (MOD_BUDGET_US) is named once in the load warnings the profiler shows (wall-clock, so warning only). Tests: an_endless_loop_is_stopped_and_the_game_goes_on, a_slow_mod_is_named_in_the_warnings. Open: a memory cap per mod. The VM has a 256 MB cap overall; per-mod accounting needs Luau memory categories (lua_setmemcat / lua_totalbytes), which mlua 0.12 only exposes through heap dumps, too slow per call.
