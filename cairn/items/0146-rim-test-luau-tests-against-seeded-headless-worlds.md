---
id: acaa16f4-f165-4ddf-a5aa-da860100abcb
title: 'rim test: Luau tests against seeded headless worlds'
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-24
closed_at: 2026-09-24
priority: p0
api: additive
effort: m
layer: tooling
area: tests
pillar:
- determinism
- plugin-first
---

## Why

The engine is deterministic and headless (DESIGN.md §7). Modders should get that for free: set up a scene, advance ticks, assert. Mood is the first customer.

## What

`tests/*.luau` in a mod:

```luau
test("stampede spawns boars", function(t)
  local w = t.world({ seed = 7, mods = { "core", "wildlife_plus" } })
  w:run_incident("wildlife_plus:boar_stampede", { points = 300 })
  t.expect(w:count_pawns("hostile")).to_be_at_least(2)
end)
```

## Acceptance criteria

- [x] Test API: build a world from seed and mod set, place things and pawns, issue commands, advance ticks, read state
- [x] Runs headless; exit code and a JUnit-style report for CI
- [x] Failures print seed and tick so they reproduce exactly
- [x] core and wildlife_plus ship tests

## 2026-09-24

The runner is rim_sim::modtest; the CLI is `rim test` in the client binary (a plain fn main dispatches it before macroquad opens a window, so it runs on headless CI: the new CI step runs it on Linux without Xvfb and on macOS, which has no OpenGL). Test code runs in its own Luau VM; each t.world() is a real Sim with its own sim VM. Expectations are a Luau prelude so error(msg, 3) points at the test's own line, and the runner appends the last-touched world's seed and tick. w:call runs a module export inside the world like a hook (ScriptHost::call_export); core's storyteller now exports fire(id, overrides) and ids(). Shipped: 7 core tests, 4 weather, 2 wildlife_plus, all in 0.4 s, also run by cargo test (tests/mod_tests.rs). Follow-ups worth filing: types for the test API (a types/test.d.luau), and scripts/check-luau.sh covering tests/.
