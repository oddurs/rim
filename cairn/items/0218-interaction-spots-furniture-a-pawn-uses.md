---
id: 218
title: 'Interaction spots: furniture a pawn uses'
type: feature
status: backlog
milestone: building
created: 2026-09-23
updated: 2026-09-23
priority: p1
api: additive
effort: m
layer: engine
area: ai
---

## Why

A bed is already furniture a pawn goes to and is better off for using.
`BedDef { rest_rate }` is that idea hard-coded for one case. A chair at a
table is the same idea, and so is every workstation later.

## What

- `[[thing]]` gains `spots = [...]`: cells, relative to the thing, a pawn
  stands or sits in to use it.
- A spot is reserved while occupied, so two pawns do not share a chair.
- A job can name the furniture it wants; `BedDef` becomes the first user of
  the general thing rather than its own mechanism.
- Chairs face the table they serve: a spot may require an adjacent thing by
  category, which is how a chair knows it is at a table and not alone in a
  field.

## Acceptance criteria

- [ ] Sleeping uses spots, with no behaviour change
- [ ] Two pawns cannot use one spot
- [ ] A pawn eats at a table when one is reachable, and on the spot when not
