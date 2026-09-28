---
id: 5c2229d9-bf5d-4120-a0ae-1ae9c11f1a7f
title: Body parts drawn as sprites and lines
type: feature
status: backlog
milestone: people
depends_on:
- 535a1fb9-2cd8-4798-b31c-04f89fb1aee0
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: additive
effort: s
layer: client
area: render
pillar:
- plugin-first
---

## Why

Bodies (535a1fb9) shipped `disc` and `box` parts. §6h promises a part is any
look layer, so a mod can paint a creature (a sprite pack) or draw thin
features (antlers, a spear, whiskers) as lines. Today antlers are thin
boxes, and nothing can be painted.

## What

- `draw = "sprite"`: a sprite key from the world atlas (as looks name them),
  drawn in the part's box and turned with it. The figure batch samples the
  atlas: the quad gains a UV rect and a flag, so sprites stay in the same
  single draw call.
- `draw = "line"`: `start` and `end` in the body's frame, at a weight or a
  width, drawn as a capsule quad in the same batch.
- docs/modding/bodies.md gains both, with a sample the guide test reads.

## Acceptance criteria

- [ ] A sprite part draws from the atlas, turned with the figure, still one draw call (autotest check and shot)
- [ ] A line part draws between its ends at its weight (unit test on the emitted quad)
- [ ] Unknown sprite keys and bad line ends are errors naming the file and part (tests)
- [ ] The guide documents both, and its samples read
