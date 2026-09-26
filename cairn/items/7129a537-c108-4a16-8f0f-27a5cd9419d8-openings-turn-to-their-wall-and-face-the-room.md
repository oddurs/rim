---
id: 7129a537-c108-4a16-8f0f-27a5cd9419d8
title: Openings turn to their wall and face the room
type: feature
status: backlog
milestone: houses
depends_on:
- 3fe8c3cb-dcba-4882-b623-0468ea9fe697
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: client
area: render
---

## Why

A door is a rectangle with a bar in it, and it can't say which way it opens. A window is a wall with a blue square. In a plan, both take their direction from the wall they sit in, and a door shows the arc it swings through into the room. DESIGN.md §6c.

## What

- `look.orient = "run"`: layers are written for an east–west run, and the renderer turns them 90° when the joined neighbours run north–south.
- `into = "room"` on a layer mirrors it toward the enclosed side. If both sides are enclosed it goes toward the smaller room, and if neither is, toward the south or east.
- An `arc` primitive: a stroked arc about a point, `r`, `from` and `to` in degrees.
- Core's door is two jambs, a leaf and its swing. Core's window is a pane band across the mass.

## Acceptance criteria

- [ ] A door in a north–south wall draws turned, with no facing set (screenshot)
- [ ] A front door's leaf swings into the house; an inner door's swings into the smaller room
- [ ] `arc` is validated like the other primitives: unknown or out-of-range fields are load errors
