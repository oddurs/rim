---
id: 587
uid: 2c567482-4817-4b38-b8ab-08d61be19e77
title: Escape leaves the command palette open and deselects instead
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
layer: core
area: ui
---

## What

`core:escape` (mods/core/ui/toolbar.luau) backs out one step: stop placing,
close the tray, close the open sheet, else deselect. The command palette
(`core:palette`, keys.luau) is a window, not a sheet, and has no Escape of
its own: its query input's `on_key` hears only up and down.

## How it fails

Ctrl+K, then Escape to dismiss it. The engine gives the keyboard back (the
query blurs), then `core:escape` runs a step *behind* the palette: with no
tray or sheet up it deselects the colonist. The palette stays on screen.
Pressing Escape again deselects again, or closes a tray the player can't see
under it; nothing short of Ctrl+K again or running a command closes it.

## Repro

Select a colonist, Ctrl+K, Escape: the palette is still open and the
inspector is empty.

## Fix

The palette is the first thing Escape closes: `core:escape` closes it and
stops there when it is open.
