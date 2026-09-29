---
id: 686
uid: e480fe01-3ec7-42f2-9e72-61def67bae14
title: Middle-drag doesn't pan while a context menu is open
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: client
area: ui
---

## What

`RawInput::pan` (crates/rim_client/src/main.rs, `RawInput::gather`) sums
the keyboard's pan (W/A/S/D, arrows, held) and the middle-button drag into
one value, and `frame` applies it only when the UI didn't take the keys
(`!out.captured_keys`).

Since 94569e71, an open popup (core's context and orders menu) takes the
keyboard every frame it is up, so a held arrow stays the menu's. The drag
pan goes with it: middle-dragging the map while a menu is open does
nothing.

## How it fails

Right-click a tile, then middle-drag: the map stays put and the menu stays
open. Before 94569e71 the drag panned and, the camera having moved, closed
the menu.

## Fix

Keep the keyboard's pan and the drag's pan apart in `RawInput`, and gate
only the keyboard's on `captured_keys`. main.rs is mid-refactor
(c24ea9b8), so this belongs with that work.
