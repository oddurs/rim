---
id: cc7114a3-cf60-4e9a-8f5a-3e86000bfaca
title: 'Typography and spacing pass: minimal, not ugly'
type: chore
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-24
updated: 2026-09-24
closed_at: 2026-09-24
priority: p2
api: none
---

## Why

The UI was cramped: 2 px vertical padding on buttons, 8 px in panels, 11 px
secondary text in a grey too dim to read, windows as translucent as the
bars behind them. Minimal should not mean ugly.

## What

A spacing and type pass on the theme and kit, judged from screenshots a
software rasteriser makes from the draw list. Tokens keep their names.

## Acceptance criteria

- [x] Screenshots of the HUD, palette, devtools, gallery and profiler can be made from a test without a window
- [x] Buttons, panels, windows and secondary text have room and contrast, and every engine test still passes

## 2026-09-24

Judged with a software rasteriser of the draw list (tests/common/raster.rs; cargo test -p rim_ui --test shots -- --ignored writes target/ui-shots/*.png), which is how this pass and any later one can be looked at without a window. The changes are spacing, not decoration: buttons and rows get a 4 px vertical pad instead of 2, panels and window bodies 12 instead of 8, bars and the palette 12 px side padding; small text 12 instead of 11 and the muted and faint greys a step lighter so secondary text reads; windows a step more opaque than docked bars with a lighter title bar; the devtools tree no longer reserves 320 px when empty; anchored labels keep 6 px apart. Tokens on the 4 px grid are unchanged, so themes still override the same names.
