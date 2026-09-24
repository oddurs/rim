---
id: e2ce89c3-de39-43ac-a302-d2dcc325aedf
title: Sprites from mods, packed into one world atlas at load
type: feature
status: backlog
milestone: graphics
depends_on:
- b25b27e0-b43b-478f-b50b-825fedbf62e3
- 3dd22b0d-a723-4acb-ae23-4095dc784e7e
created: 2026-09-22
updated: 2026-09-24
priority: p0
api: additive
effort: m
layer: client
area: render
pillar:
- plugin-first
---

## Why

A look can name a sprite key, but nothing loads the sprites. If each mod
brought its own atlas texture, every switch between two mods' things on
screen would end a draw call, and twenty mods would mean hundreds of calls
a frame. The number of mods must not change the number of draw calls.

## What

- A mod ships loose PNGs under `sprites/`; a look's sprite key is
  `mod:name`. rim packs every loaded mod's sprites into one world atlas at
  load, with a white texel, so primitives and sprites draw from the same
  texture in the same batch.
- The atlas is at most 4096², the largest size integrated GPUs of the
  reference class all support; past that a second page, and the load log
  says so with the mods that filled it.
- Sprite keys resolve to atlas rectangles once, at load. The draw path
  never looks up a string.
- Tinting: a sprite is multiplied by the look's colour (or its material's),
  so one grey plank sprite serves every wood.
- An unknown key is a load-time error naming the mod (from the Looks item);
  a PNG that fails to decode is too.

## Acceptance criteria

- [ ] The example plugin ships sprites and a def that uses one
- [ ] Sprites from several mods on screen draw in one call (bench counts calls)
- [ ] Draw calls do not grow with the number of mods (test with a generated set of 20)
- [ ] Atlas overflow spills to a second page with a load-log line, not a crash

## 2026-09-24

Graphics milestone: rewritten from a two-line stub. The per-mod atlas in the Looks item becomes one shared atlas packed at load, since per-mod textures would break batching.
