---
id: 3b1726ff-ded5-45f6-968f-cb0a5e6a2577
title: 'Context menus: one component, subjects and providers'
type: feature
status: review
milestone: pointer
assignee: Oddur Sigurdsson
claimed: 2026-09-26
created: 2026-09-26
updated: 2026-09-26
priority: p0
api: additive
effort: m
layer: engine
area: ui
---

## Why

The map needs a menu of orders so a right-click can stop taking a default, and every other surface needs one too: a colonist, a stockpile, a tile in a tray, a news line. Built one at a time, they would drift apart in look, keys and order, and a mod couldn't add to any of them without replacing it.

## What

- `kit.menu(spec)`: the look. A caption (subject and actor), up to four groups (do, work, manage, damaging) of 28 px rows with a mark slot, label and trailing key or reason; hairline dividers between groups; tones default, primary, damaging, disabled.
- `@core/ui/menus`: `menus.add(kind, { id, label, group, applies, disabled, run })`. A menu is the rows every provider for its subject's kind gives, in group order.
- A node prop `menu = { kind, id }` makes any node a subject: a right-click on it opens its menu. The engine keeps the open menu's subject and place; `core:menu` draws it on the top layer, 4 px from the pointer, flipped to stay on screen.
- Keys: arrows, Enter, 1–9 for the nth enabled row, a first letter, Escape. Press-drag-release with the button that opened it. Closes on Escape, a click outside, a pan or zoom, or its subject going away.
- Providers run once when the menu opens and their rows are kept while it's open; a closed menu builds nothing.

## Acceptance criteria

- [x] A mod adds a row to a kind of menu with one `menus.add`, and it appears in the right group
- [x] Any node with `menu` opens its menu on right-click, placed on screen
- [x] Keys and press-drag-release pick rows; Escape and a click outside close it
- [x] Providers run once per opening, not per frame

## 2026-09-26

Engine: node props menu/at/on_key/on_outside, a popup layer (placed at at, flipped on screen, solid, takes keys ahead of bindings, hears presses elsewhere), right-release picks a popup row, ui.on_context, Ui::context and Ui::dismiss_popups for the client. Core: kit.menu (replaces the old list) and menus.luau (kinds, providers, groups, primary, keys). Popup on_key reuses the input node's on_key prop name for non-inputs.
