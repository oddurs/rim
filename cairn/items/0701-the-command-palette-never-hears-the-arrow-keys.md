---
id: 701
uid: fa774b10-cdff-4198-8a20-5a6aee9e7779
title: The command palette never hears the arrow keys
type: bug
status: done
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p1
api: none
effort: s
layer: client
area: ui
---

## Why

`RawInput::gather_ui` collects Escape, Tab, Enter, Backspace, Delete, Left, Right, Home and End, but never Up or Down, so `ui_input`'s arrows can never fire and a focused input's `on_key` never hears them. The command palette's up/down selection (mods/core/ui/keys.luau) is dead in real play. The autotest feeds `RawInput.keys` directly, so it couldn't see this. Found by green-forest.

## What

- One list of the keys the UI takes as keys, read by both `gather_ui` and `ui_input`.

## Acceptance criteria

- [x] A test fails if `gather_ui` would miss any key `ui_input` maps
