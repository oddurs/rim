---
id: ae5c3807-d02a-4cea-bfa1-7e17d4498d54
title: 'Lit edges: a mass catches the light on its top and left'
type: feature
status: backlog
milestone: houses
depends_on:
- 3fe8c3cb-dcba-4882-b623-0468ea9fe697
created: 2026-09-26
updated: 2026-09-26
priority: p2
api: none
effort: s
layer: client
area: render
---

## Why

Top-down plans are flat. A lit top-and-left edge gives a wall height without perspective, and tells a blocking mass from a floor at a glance. The lighting milestone's ruling on spike 0779def9 (PR #151) splits the job: the highlight is baked into the mesh here, and the down-right shadow is lighting's compose pass (8f4f1de8), a contact shadow that fades where direct sky reaches the cell.

## What

- A `mass` gets a thin highlight inside its exposed top and left contour edges, rounded with its outer corners.
- No baked shadow.

## Acceptance criteria

- [ ] The highlight is baked into the chunk mesh; nothing is added per frame
- [ ] It follows rounded corners and stops at joined sides (screenshot)

## 2026-09-26

Blocked on the lighting milestone's spike 0779def9 (PR #151): sun-driven shadows would conflict with a fixed down-right one. If the user picks contact shadows under direct sky light, this item becomes that.

## 2026-09-26

Rescoped to lighting's ruling on 0779def9: highlight here, shadow in lighting (8f4f1de8).
