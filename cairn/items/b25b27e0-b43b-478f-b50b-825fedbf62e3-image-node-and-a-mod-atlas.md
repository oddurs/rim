---
id: b25b27e0-b43b-478f-b50b-825fedbf62e3
title: Image node and a mod atlas
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-24
updated: 2026-09-24
priority: p1
api: additive
effort: m
layer: engine
area: render
---


## Why

There is no image node: the tree is boxes and text. Icons, swatches and
mod sprites all want a rectangle from an atlas.

## What

- `kind = "image"` referencing `mod:name`, from PNGs under a mod's
  `ui/img/`; packed into the atlas the glyphs already use; `tint = true`
  draws it in the current text colour (monochrome icons).
- Kit: `kit.icon(name, size)`.
- No SVG, no nine-slice, no animation. An icon that must scale ships twice.

## Acceptance criteria

- [ ] A mod's PNG draws at 1x and 2x with no client change
- [ ] A missing image is a named warning and a placeholder, not a crash
- [ ] Tinted icons follow the theme's text colour
