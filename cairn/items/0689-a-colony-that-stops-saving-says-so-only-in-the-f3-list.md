---
id: 689
uid: e805a478-ef0c-4c1e-b9ad-7ff770c98849
title: A colony that stops saving says so only in the F3 list
type: bug
status: backlog
milestone: bare-metal
created: 2026-09-28
updated: 2026-09-28
priority: p2
api: none
layer: client
area: save
---

## What

When the save writer fails (disk full, the folder gone, permissions),
`save::after_step` (crates/rim_client/src/save.rs) pushes "this colony
isn't being saved: …" onto `sim.warnings`. A load's notes (mods that
changed, things dropped, a new epoch, ticks lost, a damaged end copied
aside) go the same way in `game()`.

`sim.warnings` reaches the player through `view.warnings()`, which only
the F3 profiler panel lists (mods/core/ui/hud.luau). No alert, message or
toast carries them.

## How it fails

A player whose disk fills plays on for hours believing the game is saved
("The game is always saved", DESIGN.md §7a) and loses it all on quit.
A player who loads a save after a mod update never learns what was
dropped.

## Fix

A save failure is a standing alert (core's alerts registry, severity
bad) and stays until saving works again; a load's notes go to the
message log once, on the first frame. The engine side is exposing them
separately from load warnings, e.g. a `view.save_status()`.

Unverified in play (needs a failing disk); traced through the code.
