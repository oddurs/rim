---
id: 99398b15-0556-4997-a8e8-51e4dcf82511
title: Priority rules on alerts and Luau predicates
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 0cb48faf-617a-4a64-922a-d6ebb842145a
created: 2026-09-25
updated: 2026-09-25
priority: p3
api: additive
effort: m
layer: engine
area: ai
---

## Why

Rules and stances (0cb48faf) shipped the conditions data can say: hours, seasons, the stance, a colonist's need. DESIGN.md §4d also names alerts and, for anything a curve can't say, a cached Luau predicate.

## What

- `when.alert`: once the engine has alerts (it has none yet; they arrive with the storyteller or the alerts UI).
- `when.script = "mod:fn"`: a Luau predicate, colony-wide, cached. It is re-run when an input it declares changes, or on a cadence it declares, never per tick, so `update_rules` stays a key compare.

## Acceptance criteria

- [ ] A mod gates a rule on a Luau predicate, and it runs only when its declared inputs change (measured)
- [ ] Two runs with a scripted rule hash identically
