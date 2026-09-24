---
id: b25b27e0-b43b-478f-b50b-825fedbf62e3
title: Image node and a mod atlas
type: feature
status: done
milestone: plugin-api
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
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

- [x] A mod's PNG draws at 1x and 2x with no client change
- [x] A missing image is a named warning and a placeholder, not a crash
- [x] Tinted icons follow the theme's text colour

## 2026-09-24

Images are decoded once at load (png crate, already in the lock through macroquad) and kept as RGBA; the atlas places one the first time it is drawn, through the same shelf packer as glyphs, and re-places it after a clear like a glyph. An image node is a fixed-size leaf: its own logical size scaled, unless w/h say otherwise. Tint draws the quad as a mask in the text colour (the GlyphQuad color flag the client already honours), so no client change. A stem@2x.png is a second variant picked from 1.5x up. A missing src is a build error for that node only: a red placeholder naming the image and what does exist.
