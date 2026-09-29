---
id: 682
uid: d4854e59-b814-4ea7-bb99-836ca9c5f301
title: A text size of 0 or below in a node or theme crashes or freezes the client
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
layer: client
area: ui
---

## What

Sizes from a mod reach cosmic-text and the glyph code unchecked: a node's
numeric `size`, `gap`, `pad`, `w`… (`node.rs` `size()` multiplies any
number by the scale) and every `[text]`/`[space]`/`[shape]` value in a
mod's `ui/theme.toml` (`Theme::load` takes any float).

## How it fails

Measured with a probe mod in the UI test harness:

- `size = -12` (a node, or `[text] body = -12` in a theme): the frame
  never returns. cosmic-text's `Buffer::shape_until_scroll` loops
  forever with a negative line height (sampled stack). The client
  freezes.
- `size = 0`: cosmic-text panics, "line height cannot be 0"
  (buffer.rs:384). The client crashes.
- `[text] body = 1e9`: an overflow panic in cosmic-text's glyph cache.
- `[space] m = 1e30`: an overflow panic in `Text::quads`
  (crates/rim_ui/src/text.rs:404, `x.round() as i32 + gx + ...`).

One typo in one mod's UI takes the game down. A UI mod is meant to fail
into an error box (DESIGN.md §11).

## Fix

A numeric size is finite, at least 0 and at most a limit; a text size is
above 0 and at most a limit. The same holds for theme tokens, where a bad
value is the warning a bad colour already is. Glyph positions add in f32,
so a far-off node can't overflow.
