---
id: 827b2421-129a-4995-b475-1398dc91d2cb
title: 'House plans as data: an ASCII grid placed with one command'
type: feature
status: backlog
milestone: houses
depends_on:
- ba18a8e4-6893-4e2e-8e30-9d8040b41109
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: additive
effort: m
layer: engine
area: building
---

## Why

Minecraft structures and Factorio blueprints make a good house something you place again. It's text, with no binaries, and the balance bots could build from plans instead of hand-coded huts. DESIGN.md §6c.

## What

- A `[[plan]]` def: `grid` (text), `legend` (character to thing), and an optional material per character.
- `Command::PlacePlan { plan, at, facing, stuff }` places every piece as a blueprint, skipping cells where it can't.
- Primitive ships a branch hut and a cob house. The stone_age bot uses them.
- The client saves a selection as a plan file, in the same format.

## Acceptance criteria

- [ ] A placed plan turned east matches the grid turned east (test)
- [ ] The stone_age sweep builds its hut from a plan and holds its numbers
