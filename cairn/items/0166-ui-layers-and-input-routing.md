---
id: 166
title: UI layers and input routing
type: feature
status: planned
milestone: interface
depends_on:
- 165
created: 2026-09-23
updated: 2026-09-23
priority: p0
api: none
effort: m
layer: client
area: ui
---

## Why

The UI and the world share the mouse and keyboard; each event must go to exactly one of them, top layer first.

## What

Layers, bottom to top: world, anchored, docked, windows, menus and tooltips, modal, toasts. Hit-testing walks top-down; hover, pressed, focused and disabled states are tracked per node; keyboard focus moves with Tab. Events the UI doesn't handle fall through to the world as today's `Action`s.

## Acceptance criteria

- [ ] Clicking a panel never reaches the world; clicking empty screen does
- [ ] Hover, pressed, focused and disabled states restyle nodes through tokens
- [ ] Modal layer blocks everything beneath it
- [ ] Tab and Shift+Tab move focus between focusable nodes; Enter activates
- [ ] Tests for routing: panel over world, menu over panel, modal over all
