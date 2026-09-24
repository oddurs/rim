---
id: 2a2f2f48-2e45-4b75-8b29-0fb868857990
title: 'Founder trait: combat bonus and recruitment pull'
type: feature
status: done
milestone: shelter
assignee: Oddur Sigurdsson
created: 2026-09-22
updated: 2026-09-24
closed_at: 2026-09-24
priority: p2
api: none
effort: s
layer: core
area: combat
pillar:
- survival
- growth
---

## Why

The founder matters most when alone. Their death is an event, not game over.

## Acceptance criteria

- [x] Founder damage bonus from start def
- [x] Founder death recorded as a colony event

## 2026-09-24

The damage bonus was already wired (StartDef.founder_damage_bonus, applied in ai::swing, core's warrior start sets 5) but nothing tested it; ai::melee_base and melee_bounds now expose the pre-roll value so the test asserts the founder's bounds beat a plain colonist's. A founder's death now says so in its own message, notes a founder_died event for the UI, and pawn_died carries founder = true for scripts; colony_lost is unchanged, so a founder dying with others left is an event and alone is still the end. The title's recruitment pull has no criterion here and is not done; it belongs with recruitment when there is any.
