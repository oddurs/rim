---
id: a8ca37c0-672c-4848-a3f5-08608d1e57cb
title: Enter in a context menu also clicks the last button clicked
type: bug
status: done
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
closed_at: 2026-09-28
priority: p2
api: none
layer: client
area: ui
---

## What

`Ui::route` (crates/rim_ui/src/lib.rs) hands an open popup every named key
("enter" included) and then, separately, activates the focused control's
`on_click` when `input.enter` is set. Nothing stops the second once the
popup has the key. Tab is the same: the popup hears "tab" and focus moves.

Every clickable node takes focus when clicked, and a right-click on the map
doesn't clear it.

## How it fails

Click a colonist in the Colonists list (it takes focus), right-click the
map to open the orders menu, press Enter to pick the highlighted order: the
order runs, and the colonist button's `on_click` runs again (select and
move the camera to them). Any focused button fires with every Enter used
in a menu.

## Fix

While a popup takes the keyboard, Enter and Tab are its alone: the focused
control neither activates nor moves.
