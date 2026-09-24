---
id: 166
title: UI layers and input routing
type: feature
status: done
milestone: interface
depends_on:
- 165
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

The UI and the world share the mouse and keyboard; each event must go to exactly one of them, top layer first.

## What

Layers, bottom to top: world, anchored, docked, windows, menus and tooltips, modal, toasts. Hit-testing walks top-down; hover, pressed, focused and disabled states are tracked per node; keyboard focus moves with Tab. Events the UI doesn't handle fall through to the world as today's `Action`s.

## Acceptance criteria

- [x] Clicking a panel never reaches the world; clicking empty screen does
- [x] Hover, pressed, focused and disabled states restyle nodes through tokens
- [x] Modal layer blocks everything beneath it
- [x] Tab and Shift+Tab move focus between focusable nodes; Enter activates
- [x] Tests for routing: panel over world, menu over panel, modal over all

## 2026-09-23

Layers bottom to top: anchored, docked, windows, cursor, modal, tooltip. Panels with a background swallow clicks; empty screen goes to the world (tests). Tab moves focus once a UI control has it (Tab otherwise stays the game's 'next colonist'); Enter activates; Escape gives the keyboard back. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
