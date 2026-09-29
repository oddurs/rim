---
id: 576
uid: 1be5c83c-d17e-487d-bfcd-1626e8348799
title: Saving a setting drops the comments in the player's settings file
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

`edit_settings` (crates/rim_client/src/settings.rs) parses settings.toml
into a `toml::Table` and writes it back, so a comment or the layout the
player gave the file by hand is gone the first time the game saves any
setting (UI scale, lighting, scroll mode). The test
`saving_a_setting_keeps_the_others` writes "# mine" but doesn't check it
survives. Fix: edit with toml_edit (already a transitive dep? check) or
state plainly that the file is the game's.
