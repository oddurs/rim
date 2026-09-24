---
id: 163
title: 'System UI font: discovery, shaping and a glyph atlas'
type: feature
status: done
milestone: interface
depends_on:
- 162
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
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
- [x] Falls back per glyph: accented Latin, Greek, CJK and Arabic samples all render
- [x] Glyph atlas caches by (font, size, glyph); a steady frame uploads nothing new
- [x] Crisp at 1x and 2x DPI
- [x] `font` in the theme overrides the family; a missing font falls back with a warning

## 2026-09-23

SF Pro from /System/Library/Fonts/SFNS.ttf on macOS; fontconfig sans-serif (DejaVu) on Linux CI; 883 fallback faces. Fallback samples (accented Latin, Greek, Japanese, Arabic, emoji) produce glyphs. The atlas uploads only when dirty; shaped strings unused for ~10 s are evicted. The Windows criterion can't be verified without a Windows machine: moved to 0156 (Windows CI), which now checks that segoeui.ttf resolves.

## 2026-09-23

Theme font override tested (a_theme_can_name_a_font_and_a_missing_one_falls_back): a named family is used; a missing one falls back to the system UI font with a warning naming the mod and font, shown in F3.

## 2026-09-23

Crisp at 1x (CI Linux, DejaVu Sans) and 2x (macOS, SF Pro) confirmed from screenshots. Criterion 1's Windows part is tracked in 0156; macOS and Linux resolve and render.
