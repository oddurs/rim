---
id: b6d0a4cc-3335-4323-88bb-c77f207199e9
title: A selection off screen leaves a chevron at the edge
type: feature
status: planned
milestone: chalkline
depends_on:
- d83192ed-0a3e-4a6f-a39a-04ce94a68935
created: 2026-09-27
updated: 2026-09-27
priority: p2
api: none
effort: s
layer: client
area: ui
---

## Why

Pan away from a selected colonist and nothing says where they went. DESIGN.md §6f.

## What

- For each selected thing whose centre is off screen: a chalk chevron on a keyline, 16 px inside the screen edge, on the line from the screen's centre to the thing, pointing at it.
- A chip beside it reads "Name · N cells".
- Clicking the chevron centres the camera on the thing.

## Acceptance criteria

- [ ] A unit test of the edge clamp: a target left of the screen puts the chevron at x = 16 and points it left
- [ ] Autotest: select a colonist, pan 40 cells away, and a chevron mark is in the scene; clicking it brings the colonist on screen
- [ ] Screenshot `chalk-offscreen`
