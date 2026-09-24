---
id: 01e4d9fe-095a-47d9-b06b-5d2c5099f063
title: Text input the engine owns
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
area: ui
---

## Why

The tree is rebuilt twenty times a second. A caret that lives in Luau is
lost every rebuild, so edit state has to live in the engine, keyed by id.

## What

- `kind = "input"`: buffer, caret and selection in the engine keyed by node
  id; `on_change(text)` and `on_submit(text)`; click to focus, tab away.
- `Input` gains key events beyond tab and enter: characters, backspace,
  arrows, home/end, escape.
- Kit: `kit.input`, `kit.slider` (a bar that reports drag).

## Acceptance criteria

- [x] Typing survives twenty rebuilds without losing the caret
- [x] Focus, edit, submit and escape each have a headless test
- [x] Hot reload keeps the buffer of a focused input

## 2026-09-24

The buffer, caret and selection live in the engine (edit.rs) keyed by node id, lent to build (so the input shows them) and to paint (caret and selection), and never cleared by a rebuild or a reload. Keys reach the engine as Input.keys (chars plus backspace, delete, arrows, home, end, escape); a focused input takes every key and reports captured_keys, and the client then skips its own key map and camera pan. Enter submits and keeps focus; Escape blurs. on_drag is the general drag report (fractions across the node) that kit.slider is built on. A false placeholder child at [1] used to be read as text content; that broke a zero-width bar and is fixed here.
