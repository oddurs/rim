---
id: fbab2877-1a33-46ed-96c9-223673df6d03
title: Core's looks in the plan style
type: content
status: done
milestone: houses
assignee: Oddur Sigurdsson
depends_on:
- 3fe8c3cb-dcba-4882-b623-0468ea9fe697
- 7129a537-c108-4a16-8f0f-27a5cd9419d8
- 7c53ec62-85bb-4752-8bed-9b1271d0eef3
created: 2026-09-26
updated: 2026-09-27
closed_at: 2026-09-27
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

- [x] The autotest gallery screenshot shows every core building in the plan style
- [x] docs/modding/looks.md documents joins, orientation, patterns and weights with core's defs as its samples

## 2026-09-26

Stairs get a plan look (treads, break line, UP/DN arrow) once Depth's portals exist; not in this item.

## 2026-09-27

Four named weights (heavy, medium, light, hair) live in rim_sim::look::Weight, sized with the zoom in Weight::px and nowhere else; outline, arc, edges and line take `line = <weight>` as well as `width` in points. New layers: `box` (a rounded body outlined in one ink, rim_sim::look::INK) and `line`. Core's furniture is now plan symbols; walls and windows take the heavy contour, the door's swing the light one. Sizes are unchanged: the prototype's 1x2 bed and 2x1 table would change play, which is a separate call. Primitive's pallet is converted too; the timber mod's pieces are left for their author.
