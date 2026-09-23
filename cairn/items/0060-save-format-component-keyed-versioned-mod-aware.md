---
id: 60
title: 'Save format: component-keyed, versioned, mod-aware'
type: spike
status: backlog
milestone: persistence
created: 2026-09-22
updated: 2026-09-22
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
