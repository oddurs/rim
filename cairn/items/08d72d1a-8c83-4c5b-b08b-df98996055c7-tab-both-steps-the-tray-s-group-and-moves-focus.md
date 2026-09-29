---
id: 08d72d1a-8c83-4c5b-b08b-df98996055c7
title: Tab both steps the tray's group and moves focus
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

Unverified in play. With a tray open and a control focused (clicking a tray
tile focuses it), Tab runs the `core:dock.next_group` binding (its `when`
holds) and, in the same `Ui::route`, `input.tab && self.focused.is_some()`
also moves UI focus. One key, two effects. A bound action that takes the
key should end it there: focus moves only when no binding took Tab.
