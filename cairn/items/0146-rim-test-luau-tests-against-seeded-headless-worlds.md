---
id: 146
title: 'rim test: Luau tests against seeded headless worlds'
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-23
updated: 2026-09-23
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

- [ ] Test API: build a world from seed and mod set, place things and pawns, issue commands, advance ticks, read state
- [ ] Runs headless; exit code and a JUnit-style report for CI
- [ ] Failures print seed and tick so they reproduce exactly
- [ ] core and wildlife_plus ship tests
