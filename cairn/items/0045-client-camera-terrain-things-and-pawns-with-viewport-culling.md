---
id: 793e0a96-d4af-49d4-ad58-ff7493ddfdae
title: 'Client: camera, terrain, things and pawns with viewport culling'
type: feature
status: done
milestone: castaway
created: 2026-09-22
updated: 2026-09-23
closed_at: 2026-09-23
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

- [x] Pan with WASD/drag, zoom with wheel
- [x] Pawns interpolate between cells
- [x] Blueprints, designations and regrowth visible

## 2026-09-22

Renders on macOS (screenshot verified: terrain, trees, water, founder, needs panel, def-driven toolbar). Input paths not yet exercised by hand.

## 2026-09-23

Verified by rim --autotest (crates/rim_client/src/autotest.rs), which drives the real client through the same Actions keyboard and mouse produce, checks state and saves screenshots; 56/56 checks pass, screenshots reviewed by eye. Runs in CI on macOS. Camera: pan, zoom about the cursor, zoom clamp; interpolation confirmed mid-step. Screenshots showed berry bushes unreadable at normal zoom and regrowing bushes looking like trees: bushes now larger with bigger berries, picked bushes small and brown. Name labels and the sleep 'z' now draw above the night overlay; only colonists are always named (others on hover/select) to stop labels overlapping.
