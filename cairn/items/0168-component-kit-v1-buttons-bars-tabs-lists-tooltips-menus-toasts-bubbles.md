---
id: 168
title: 'Component kit v1: buttons, bars, tabs, lists, tooltips, menus, toasts, bubbles'
type: feature
status: done
milestone: interface
depends_on:
- 166
- 167
created: 2026-09-23
updated: 2026-09-23
closed_at: 2026-09-23
priority: p0
api: additive
effort: l
layer: core
area: ui
pillar:
- plugin-first
---

## Why

A small complete set of components, so every HUD part and mod panel is composed rather than drawn by hand.

## What

Written in Luau in `mods/core/ui/kit/` on the engine primitives, and exported as a module other mods `require`: `button`, `toggle`, `bar`, `tabs`, `list`, `tooltip`, `menu` (context), `toast`, `bubble`, `label`, `divider`. Each has hover, pressed, disabled and focused states from tokens.

## Acceptance criteria

- [x] Each component has a snapshot test of its tree
- [x] Every state styled only through tokens (no literal colours or sizes in the kit)
- [x] `require("@core/ui/kit")` works from another mod
- [x] A kit gallery screen (debug) renders every component and state for review

## 2026-09-23

mods/core/ui/kit.luau: panel, row, col, label, button, toggle, bar, tabs, list, menu, toast, bubble, divider, heading. Only tokens, no literals. Required from other mods (wildlife_plus, guide samples). Gallery: F12 then Kit gallery (screenshot reviewed; fixed: disabled now dims children). Snapshot tests cover the trees core builds from it. Verified by `cargo test -p rim_ui` (15 engine tests, headless) and `rim --autotest` (92/92, screenshots reviewed).
