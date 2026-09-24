---
id: fdafcb10-6fcf-4a63-b798-fbd8a5e32ade
title: Custom jobs and work givers from Luau
type: feature
status: backlog
milestone: plugin-api
depends_on:
- 0e73145a-39c4-4f1d-88a4-59d803a2f535
created: 2026-09-22
updated: 2026-09-24
priority: p0
api: additive
effort: l
layer: engine
area: scripting
pillar:
- plugin-first
---

## Why

New verbs, not just new nouns.

## Acceptance criteria

- [ ] Script can offer jobs to idle colonists
- [ ] Budgeted per tick

## 2026-09-24

Luau work givers post into the work pools (0e73145a) rather than being polled per pawn, within a per-tick budget: scripts say what work exists, the engine says who does it (DESIGN.md §4d).
