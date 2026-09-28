---
id: 5e92cd70-a3d5-4c33-9440-6d31581b615b
title: The pawn panel shows wetness and what's worn
type: feature
status: backlog
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: additive
effort: s
layer: client
area: ui
---

## Why

Getting wet (0201) and apparel (d0362b0c) work in the sim, but the player can't see either. Pawn::wet and Worn aren't in the pawn panel, so a colonist losing warmth in the rain looks like a bug.

## Acceptance criteria

- [ ] The pawn panel shows how wet a colonist is when they're not dry
- [ ] It lists what they're wearing and each garment's warmth
- [ ] The UI API change is additive, and types/ui.d.luau and the docs are regenerated
