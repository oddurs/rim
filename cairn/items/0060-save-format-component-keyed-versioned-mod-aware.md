---
id: 60
title: 'Save format: component-keyed, versioned, mod-aware'
type: spike
status: backlog
milestone: persistence
depends_on:
- 138
created: 2026-09-22
updated: 2026-09-23
priority: p0
api: none
effort: s
layer: engine
area: save
pillar:
- plugin-first
---

## Question

What does a save look like so it survives mod updates and removals?

## Options

- Serde of typed components keyed by string
- Snapshot + command log
- Binary with a schema table

## Decision


## Acceptance criteria

- [ ] Decision recorded in DESIGN.md

Constraint from DESIGN.md §10: def and component keys are namespaced (`mod:id`, 0138), saves record every mod's version so 0139 can migrate, and the mod list is the lockfile (0152).
