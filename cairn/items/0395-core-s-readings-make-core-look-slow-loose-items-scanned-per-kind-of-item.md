---
id: 395
uid: af2f8428-a53d-4554-90fd-18d52b783d0b
title: 'Core''s readings make core look slow: loose items scanned per kind of item'
type: bug
status: done
milestone: work
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p1
api: none
effort: s
layer: core
area: perf
---

## Why

`a_slow_mod_is_named_in_the_warnings` started failing on macOS CI and locally with "mod 'core' is slow: 0.84 ms". The profiler's smoothed cost takes its first sample whole, and in that test's 1,300 ticks core makes only a couple of calls. The first is now `readings.luau`'s hourly publish at tick 0, which called `rim.stock` once per kind of item for the loose count.

## What

- Count loose items per root of the item category tree (a lookup per branch; every item is under one root).

## Acceptance criteria

- [x] `a_slow_mod_is_named_in_the_warnings` passes
- [x] `core:loose_items` still matches a hand count (default_orders test)
