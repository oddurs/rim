---
id: 0811b7b6-8d38-4b41-98fa-a5f42850fc54
title: 'a_slow_mod_is_named_in_the_warnings fails under load: core crosses the wall-clock budget too'
type: bug
status: done
milestone: scale
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p2
api: none
effort: s
layer: engine
area: tests
---

## What happens

## What should happen

## Reproduction

Seed:
Mods:
Tick:

1.

`scripting::a_slow_mod_is_named_in_the_warnings` asserted that core is not
named slow. The budget is wall-clock, so on a loaded machine (load
average ~105 on 2026-09-27) core's own calls crossed it too and the test
failed with nothing wrong; it passed on rerun.

## Acceptance criteria

- [x] The test still fails if the slow mod isn't named
- [x] It no longer depends on core staying under a wall-clock budget
