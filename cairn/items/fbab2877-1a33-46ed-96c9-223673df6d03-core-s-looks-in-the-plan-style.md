---
id: fbab2877-1a33-46ed-96c9-223673df6d03
title: Core's looks in the plan style
type: content
status: backlog
milestone: houses
depends_on:
- 3fe8c3cb-dcba-4882-b623-0468ea9fe697
- 7129a537-c108-4a16-8f0f-27a5cd9419d8
- 7c53ec62-85bb-4752-8bed-9b1271d0eef3
created: 2026-09-26
updated: 2026-09-26
priority: p1
api: none
effort: m
layer: core
area: render
---

## Why

Once the primitives exist, every core def should use them: walls as masses, openings along the run, furniture as outlined plan symbols in four line weights. This is the art pass under "no sprites" (§6a).

## What

- Wall, door, window, floor, bed, table, chair, stove, campfire, and crafting's spot and workbench, drawn as in the prototype.
- Line weights come from four named sizes in one place, so a mod can match them.

## Acceptance criteria

- [ ] The autotest gallery screenshot shows every core building in the plan style
- [ ] docs/modding/looks.md documents joins, orientation, patterns and weights with core's defs as its samples

## 2026-09-26

Stairs get a plan look (treads, break line, UP/DN arrow) once Depth's portals exist; not in this item.
