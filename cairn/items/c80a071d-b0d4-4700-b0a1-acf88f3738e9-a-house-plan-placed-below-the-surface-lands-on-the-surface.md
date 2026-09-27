---
id: c80a071d-b0d4-4700-b0a1-acf88f3738e9
title: A house plan placed below the surface lands on the surface
type: bug
status: done
milestone: houses
created: 2026-09-27
updated: 2026-09-27
closed_at: 2026-09-27
priority: p1
api: none
effort: s
layer: engine
area: building
---

## Why

`Command::PlacePlan` built each piece at `IVec::new(x, y)`, which is the surface, whatever level the plan was placed on (827b2421). Once levels can be viewed and built on, a plan placed at z −1 would put its walls up at z 0. Reported by the Depth work.

## What

- Pieces keep the placement's level.

## Acceptance criteria

- [x] A plan placed at z −1 plans every piece at z −1 and nothing on the surface (test)
