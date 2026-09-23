---
id: 163
title: 'System UI font: discovery, shaping and a glyph atlas'
type: feature
status: planned
milestone: interface
depends_on:
- 162
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: client
area: ui
---

## Why

The UI uses the operating system's own font, found at runtime and never shipped, so it looks native and costs nothing to license.

## What

Resolve the system UI font per platform (San Francisco on macOS, Segoe UI on Windows, fontconfig `sans-serif` on Linux) with a fallback chain for scripts the primary lacks. Shape and lay out text with cosmic-text; rasterise glyphs into a cached atlas texture. A theme can name a different family.

## Acceptance criteria

- [ ] Resolves the system font on macOS, Linux (CI) and Windows (by path table, verified in 0156's Windows CI)
- [ ] Falls back per glyph: accented Latin, Greek, CJK and Arabic samples all render
- [ ] Glyph atlas caches by (font, size, glyph); a steady frame uploads nothing new
- [ ] Crisp at 1x and 2x DPI
- [ ] `font` in the theme overrides the family; a missing font falls back with a warning
