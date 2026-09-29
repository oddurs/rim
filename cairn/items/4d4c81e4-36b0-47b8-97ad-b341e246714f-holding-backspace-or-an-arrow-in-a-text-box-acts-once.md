---
id: 4d4c81e4-36b0-47b8-97ad-b341e246714f
title: Holding Backspace or an arrow in a text box acts once
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
layer: client
area: ui
---

## What

`RawInput::gather_ui` (crates/rim_client/src/main.rs) builds a text input's
editing keys (Backspace, Delete, Left, Right, Home, End, and Up/Down for
`on_key`) from `is_key_pressed`. macroquad sets that only on a key's first
down event: `key_down_event` skips `keys_pressed` when `repeat` is true
(macroquad 0.4 lib.rs). Typed characters come from `get_char_pressed`,
which does repeat.

## How it fails

Hold Backspace in the command palette's query or the tray's find box: one
character goes, not the word. Holding Left or Right moves the caret one
place. Holding a letter types it again and again, so the two disagree.
The same holds for a held Down in the palette's list.

## Fix

Take the editing keys' repeats too: `gather_ui` already reads miniquad's
raw events through an input subscriber for the wheel (`Wheel::gather`,
`repeat_all_miniquad_input`); a `key_down_event` with `repeat` set for an
editing key adds it again. Bindings stay on first presses only.

main.rs is calm-forest's (c24ea9b8, mid-refactor); route it there.
