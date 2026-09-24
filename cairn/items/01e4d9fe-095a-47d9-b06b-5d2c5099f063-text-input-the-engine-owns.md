---
id: 01e4d9fe-095a-47d9-b06b-5d2c5099f063
title: Text input the engine owns
type: feature
status: backlog
milestone: plugin-api
created: 2026-09-24
updated: 2026-09-24
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

- [ ] Typing survives twenty rebuilds without losing the caret
- [ ] Focus, edit, submit and escape each have a headless test
- [ ] Hot reload keeps the buffer of a focused input
