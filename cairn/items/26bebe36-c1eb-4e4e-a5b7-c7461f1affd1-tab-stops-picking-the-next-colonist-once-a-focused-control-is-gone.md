---
id: 26bebe36-c1eb-4e4e-a5b7-c7461f1affd1
title: Tab stops picking the next colonist once a focused control is gone
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

`Ui.focused` (crates/rim_ui/src/lib.rs) is a node key that is only ever
cleared by Escape (`Key::Escape`, `Ui::blur`). Nothing drops it when the
focused node leaves the layout, or when the player clicks the map.

## How it fails

`Ui::route` treats `input.tab && self.focused.is_some()` as "Tab moves UI
focus" and captures the key, so the client's Tab (next colonist) never runs.
Every clickable node is focusable, so the list of focusables is never empty.

- Run a command from the palette (Ctrl+K, type, Enter): the palette closes,
  but its query input's key stays focused. Tab now cycles invisible UI focus
  instead of selecting the next colonist, until Escape.
- Click any toolbar button, then click the map: the button keeps focus, and
  Enter re-clicks it; Tab still doesn't reach the game.

## Fix

Focus on a node that isn't in this frame's layout is dropped, and a left
press that the UI doesn't take (the map) drops it too, as a click outside a
control does in any UI.
