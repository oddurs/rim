---
id: 94569e71-64c4-4365-b333-89c2f6e7b7eb
title: Holding an arrow key in a context menu pans the map and closes it
type: bug
status: doing
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: client
area: ui
---

## What

An open popup (core's context and orders menu) takes the keyboard: `Ui::route`
hands it every named key and sets `captured_keys`. But it sets it only on a
frame where a key was *pressed* (`input.pressed` non-empty,
crates/rim_ui/src/lib.rs ~1118).

The client pans the camera from keys *held down* (`RawInput::gather`:
W/A/S/D and the arrows, `is_key_down`) whenever `captured_keys` is false
(main.rs `frame`), and any camera move calls `Ui::dismiss_popups`.

## How it fails

A key stays down for several frames. Press Down in a context menu: frame one,
the menu steps its highlight; frame two, the key is still down, nothing was
pressed this frame, so the camera pans a little and the menu closes. The same
for Up, and for a first letter that is W, A, S or D ("Deconstruct"). The
menu's arrow navigation, which core documents, works only if the key is
released within one frame.

## Fix

While a popup that takes keys is open (and no text input is typing), the
keyboard is the UI's every frame: `captured_keys` is set whether or not a
key was pressed this frame.
