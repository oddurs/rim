---
id: ee5c5cc6-8f4f-4b08-8e59-3a4ba7602c3b
title: Glyph primitive drawn from the world atlas
type: feature
status: doing
milestone: graphics
assignee: Oddur Sigurdsson
claimed: 2026-09-24
created: 2026-09-24
updated: 2026-09-24
priority: p3
api: additive
effort: m
layer: client
area: render
---

## Why

DESIGN.md §6a lists a glyph among the primitives: a letter or symbol as a
thing's look, legible at any zoom and free for a mod to use. Looks shipped
without one because text isn't in the world batch.

## What

- `draw = "glyph"` with `glyph` (one character), `x`, `y`, `size`.
- Glyphs a look uses are rasterised once at load into the world atlas
  (e2ce89c3), so they draw in the chunk meshes like sprites.

## Acceptance criteria

- [x] A def with a glyph look draws in the chunk meshes, at most one more call per chunk layer that shows glyphs
- [x] An empty or multi-character glyph is a load error naming the def

## 2026-09-24

Glyphs are interned at load like sprite keys (Art { sprites, glyphs }), rasterised at 48 px with the UI's font through rim_ui's new Text::rasterize, and packed with the sprites; the Sink now takes an atlas slot, so sprites and glyphs share one path. A glyph no font has is a warning and draws nothing, rather than a load error: fonts differ between machines. Bench with --sprite-mods 20 and trail markers stamped in: calls 99/60/24, unchanged. Atlas pages are Nearest-filtered for pixel art, so a glyph scaled well below 48 px can alias; not seen at the zooms tested.

## 2026-09-24

Review: glyph 0 (a font's missing-glyph box, what shaping keeps when no font has it) now counts as missing, so the warning path is real; 'one character' is a base plus what extends it (variation selectors, ZWJ sequences, skin tones, keycaps, flags), and one that draws nothing is a load error; size is the em, glyphs keep their proportions and share a baseline; colour glyphs take shade and vary. Glyphs moved to their own Linear-filtered atlas pages: Nearest aliased them. That costs a texture switch in each chunk layer that shows both: the bench (a marker in almost every room) goes from 99 to 123 calls on the whole map. Criterion 1 reworded to say so.
