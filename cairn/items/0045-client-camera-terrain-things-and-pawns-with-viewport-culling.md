---
id: 45
title: 'Client: camera, terrain, things and pawns with viewport culling'
type: feature
status: review
milestone: castaway
created: 2026-09-22
updated: 2026-09-22
priority: p0
api: none
effort: l
layer: client
area: render
pillar:
- performance
---

## Why

See the world. Only draw what is on screen.

## Acceptance criteria

- [ ] Pan with WASD/drag, zoom with wheel
- [ ] Pawns interpolate between cells
- [ ] Blueprints, designations and regrowth visible

## 2026-09-22

Renders on macOS (screenshot verified: terrain, trees, water, founder, needs panel, def-driven toolbar). Input paths not yet exercised by hand.
