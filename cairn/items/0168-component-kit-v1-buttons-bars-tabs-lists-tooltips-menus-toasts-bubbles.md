---
id: 168
title: 'Component kit v1: buttons, bars, tabs, lists, tooltips, menus, toasts, bubbles'
type: feature
status: planned
milestone: interface
depends_on:
- 166
- 167
created: 2026-09-23
updated: 2026-09-23
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

- [ ] Each component has a snapshot test of its tree
- [ ] Every state styled only through tokens (no literal colours or sizes in the kit)
- [ ] `require("@core/ui/kit")` works from another mod
- [ ] A kit gallery screen (debug) renders every component and state for review
