---
id: 54ffec74-a987-4300-aea8-15cdae4d469c
title: 'Event dispatch: handlers indexed by name, unheard events skipped, no allocation per call'
type: perf
status: backlog
milestone: plugin-api
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: s
layer: engine
area: scripting
pillar:
- performance
---

## Why

Story plugins raise and hear many more events. Today every event builds a Luau
table even when nobody listens, handlers are found by a linear scan with a
string compare, and each call allocates its profiler label (DESIGN.md §4g, Cost).

## Acceptance criteria

- [ ] Handlers indexed by event name
- [ ] An event with no listener builds no table
- [ ] Profiler labels interned per mod; no allocation per handler call
- [ ] Bench: cost per event and per listener recorded before and after
