---
id: 36d3ea49-e9dd-40fa-b6aa-9dd67eee7485
title: 'UI stack: taffy layout, cosmic-text and a second Luau VM on macroquad'
type: spike
status: done
milestone: interface
assignee: Oddur Sigurdsson
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: none
effort: s
layer: client
area: ui
pillar:
- plugin-first
- performance
---

## Question

Can the planned stack draw a realistic HUD inside the 1 ms budget on macroquad, with crisp system-font text at any DPI?

## Options

- taffy (flexbox) + cosmic-text (shaping, fallback) + glyph atlas in macroquad textures + a separate mlua Luau VM
- egui via egui-macroquad, styled to our tokens (fallback if text or layout costs too much)
- Our own minimal layout (row/column only) if taffy's cost or API is a poor fit

## Decision

**Go: taffy 0.14 + cosmic-text 0.19 + a second Luau VM**, in a renderer-independent `rim_ui` crate that emits a draw list and a glyph atlas for the client to replay.

Measured on the reference machine (`cargo run --release -p rim_ui --example spike`):

| Stage | Cost |
|---|---|
| Font load, once (SF Pro from `/System/Library/Fonts/SFNS.ttf`, 883 fallback faces) | 224 ms |
| 300-node tree per frame: Luau build / convert / taffy layout | 0.014 / 0.020 / 0.179 ms = **0.21 ms** |
| Shape 300 labels: cold / cached | 15.4 / 0.02 ms |
| Glyph quads for 6,790 glyphs: cold (rasterise) / cached | 0.87 / 0.25 ms |
| Fallback: accented Latin, Greek, Japanese, Arabic, emoji | all produce glyphs |

Risk found: shaping a new string costs ~50 µs, and the cache grows with every distinct string (the clock). Mitigations built into the engine: evict shaped entries unused for a few seconds, and refresh fast-changing numeric readouts (profiler) a few times a second rather than every frame. Crispness at 2x DPI is verified with the renderer in 0163.

## Acceptance criteria

- [x] Prototype builds a 300-node tree from Luau, lays it out and draws it with shaped text
- [x] Measured per frame: Luau build, layout, text, draw (numbers recorded here)
- [x] Text is crisp at 1x and 2x DPI (screenshots attached as notes)
- [x] Decision recorded in DESIGN.md §11

## 2026-09-23

Decision recorded in DESIGN.md §11 ('How it's built'). The prototype became rim_ui itself; it draws in the client with shaped system-font text. Crisp at 2x confirmed on the reference machine's screenshots; 1x is checked on CI's Linux screenshots before ticking.

## 2026-09-23

Crisp at 1x confirmed on CI's Linux screenshots (Xvfb, 1600x960, DejaVu Sans via fontconfig): run 35936043817, artifact autotest-screenshots. 2x confirmed locally (SF Pro, 3200x1920).
