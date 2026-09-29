---
id: 385
uid: 97a12814-63c8-4096-9009-c5c45712cbf7
title: 'Colony readings and standing orders: rules that switch themselves'
type: feature
status: done
milestone: work
assignee: Oddur Sigurdsson
created: 2026-09-26
updated: 2026-09-26
closed_at: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: ai
---

## Why

"Whenever food runs low, forage sooner" is impossible to say today: rules read the hour, the season, the stance and a colonist's need, not the colony's stores. DESIGN.md §4d: standing orders are rules on a colony reading with a band, and readings are numbers scripts publish, so the engine names no content.

## What

- Readings: `rim.set_reading(id, value)` from sim scripts, a fixed-point number per id, saved and hashed. `rim.reading(id)` reads one. An id is `mod:name`.
- `when.reading = "core:food_days"` with `below` and/or `above`, and `until`, which ends the band on the other side (on under 5, off at 8). The rule's on/off state is world state, saved and hashed, because with a band it depends on history.
- `update_rules` re-evaluates only when a reading crosses one of its rules' marks, so it stays a key compare per tick.
- `SetRuleEnabled { rule, on }`: a colony switches a rule off. Disabled rules are listed and saved.
- Events `rule_started { rule }` and `rule_stopped { rule }` for news and scripts.
- `explain` labels a reading rule with its label, like any rule.

## Acceptance criteria

- [x] A rule with `below = 5, until = 8` turns on at 4.9, stays on at 6, and turns off at 8 (test)
- [x] Setting a reading that crosses no mark doesn't re-evaluate rules (counted, like `Rules::evaluations`)
- [x] A disabled rule never applies and survives save and load (test)
- [x] `rule_started` and `rule_stopped` fire once per crossing (test)
- [x] Determinism test passes with a scripted reading
