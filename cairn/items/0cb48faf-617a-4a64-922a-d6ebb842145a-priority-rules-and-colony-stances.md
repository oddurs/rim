---
id: 0cb48faf-617a-4a64-922a-d6ebb842145a
title: Priority rules and colony stances
type: feature
status: backlog
milestone: colony
depends_on:
- 03ad3e3a-efe1-4186-b764-8dcd1d9344f1
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: additive
pillar:
- growth
effort: m
layer: engine
area: ai
---




## Why

Players rewrite the whole grid for winter and again for a siege. Rules change priorities when something holds, and stances switch a set of rules with one click (DESIGN.md §4d).

## What

- `[[priority_rule]]`: `when` (hour range, season, alert, need, stance, or a cached Luau predicate) and `shift` or `set` per work type.
- `[[stance]]`: a named set of rules. Core ships Normal, Harvest, Winter prep and Siege; mods add more.
- Effective priority = base + active rules, with a breakdown per cell.
- A stance bar on the Work Board, and `rim.set_stance` for scripts and incidents.

## Acceptance criteria

- [ ] Effective priorities explain themselves, and the parts sum to the value
- [ ] Rules re-evaluate only when their inputs change (measured)
- [ ] A mod adds a stance with data alone
- [ ] Two runs with stance changes hash identically
