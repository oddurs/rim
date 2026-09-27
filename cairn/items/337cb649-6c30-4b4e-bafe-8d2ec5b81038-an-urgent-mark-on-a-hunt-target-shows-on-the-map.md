---
id: 337cb649-6c30-4b4e-bafe-8d2ec5b81038
title: An urgent mark on a hunt target shows on the map
type: feature
status: backlog
milestone: mood
created: 2026-09-27
updated: 2026-09-27
priority: p3
api: none
effort: s
layer: client
area: render
---

## Why

Urgent marks (bc9b9a91) work on a creature marked for hunting: work choice lifts it a level. But the map draws the mark only on things, because pawns aren't in the chunk mesh. A player who marks a deer urgent sees nothing change.

## What

- The pawn layer draws the same urgent disc over a marked creature, following it as it moves.
- If chalkline's breathing urgent ring (bf3079fb) lands first, the creature gets that ring instead of the disc.

## Acceptance criteria

- [ ] A marked deer wears the mark in a client autotest screenshot
- [ ] The mark goes when the hunt is cancelled or done

## 2026-09-27

Chalkline's bf3079fb makes the urgent mark a breathing ring on things. This item puts the same mark on creatures; whichever lands second should match the first.
