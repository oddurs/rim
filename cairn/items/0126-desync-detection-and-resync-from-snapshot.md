---
id: fa6f0afb-4220-4ee5-89c2-7fe675e0d99a
title: Desync detection and resync from snapshot
type: feature
status: backlog
milestone: co-op
depends_on:
- c5d185be-9bb8-491d-b73d-94f75e4018f0
created: 2026-09-22
updated: 2026-09-24
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

## 2026-09-24

The per-mod hash comes from the save format (DESIGN.md §7a): each snapshot section has a hash over its canonical bytes, and sections are grouped by owning mod. Resync sends a snapshot plus the log after it.
