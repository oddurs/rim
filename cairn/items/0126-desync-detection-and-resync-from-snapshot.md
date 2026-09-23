---
id: 126
title: Desync detection and resync from snapshot
type: feature
status: backlog
milestone: co-op
depends_on:
- 61
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: l
layer: engine
area: net
pillar:
- determinism
---

## Why

Desyncs must be caught and fixed.

## Acceptance criteria

- [ ] State hash exchanged every N ticks
- [ ] Resync from a save snapshot
- [ ] State hash broken down per mod (script state and components), so a desync names the mod that caused it
