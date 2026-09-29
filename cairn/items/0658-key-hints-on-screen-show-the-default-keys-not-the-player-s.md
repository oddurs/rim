---
id: 658
uid: 9a035c92-0206-4e34-a4e5-f519604456aa
title: Key hints on screen show the default keys, not the player's
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p3
api: none
layer: core
area: ui
---

A player can rebind any action (keybinds.toml, `Ui::rebind`), and the
palette shows the current key (`view.binds`). Other hints don't:

- the top bar's screen tabs (mods/core/ui/screens.luau) print
  `string.upper(s.key)`, the key given to `screens.add`;
- the top bar's help line (mods/core/ui/hud.luau, `core:topbar.help`) is a
  fixed string: "Space pause · 1/2/3 speed · R draft · O overlay · F3
  profiler · F12 devtools".

After a rebind they name keys that no longer do the thing. Fix: look the
key up by the action's id in `view.binds()` where a hint is drawn (a
`kit.key_of(id)` helper), and build the help line from it.
