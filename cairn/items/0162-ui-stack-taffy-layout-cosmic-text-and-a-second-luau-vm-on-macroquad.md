---
id: 162
title: 'UI stack: taffy layout, cosmic-text and a second Luau VM on macroquad'
type: spike
status: planned
milestone: interface
created: 2026-09-23
updated: 2026-09-23
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


## Acceptance criteria

- [ ] Prototype builds a 300-node tree from Luau, lays it out and draws it with shaped text
- [ ] Measured per frame: Luau build, layout, text, draw (numbers recorded here)
- [ ] Text is crisp at 1x and 2x DPI (screenshots attached as notes)
- [ ] Decision recorded in DESIGN.md §11
