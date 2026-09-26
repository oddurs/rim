---
id: ca22f222-1b4c-4705-b106-afbb8a282b43
title: 'Bills pick their ingredients: one material per order, and a filter'
type: feature
status: backlog
milestone: crafting
created: 2026-09-25
updated: 2026-09-25
priority: p2
api: additive
effort: m
layer: plugin
area: building
---

## Why

A recipe's tag input (`knappable`) takes whatever lies nearest, and a work order doesn't care whether its pieces match. A hand axe can be knapped from one flint and one bone, and it comes out made of whichever was brought first. So a flint axe can cost half its flint, or a bone axe can use up a flint. The player can't steer it: bills have no ingredient filter, and bone dropped at a kill near the spot beats flint in a stockpile.

## What

- **An order that takes a tag input by several** takes one material for all of them. Once the first piece is in, the rest must match.
- **A bill's ingredient filter** says which things may fill a tag input (flint only, or anything knappable), with the bills panel showing it.
- **The stall reason names the shortage**: "1 of 2 flint" rather than counting every knappable.

## Acceptance criteria

- [ ] A hand axe never mixes flint and bone
- [ ] A bill can be told to use flint only
- [ ] A bill waiting for a matching second piece says so
