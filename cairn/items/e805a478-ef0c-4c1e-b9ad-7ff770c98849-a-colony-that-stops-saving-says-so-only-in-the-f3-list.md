---
id: e805a478-ef0c-4c1e-b9ad-7ff770c98849
title: A colony that stops saving says so only in the F3 list
type: bug
status: done
milestone: bare-metal
assignee: Oddur Sigurdsson
created: 2026-09-28
updated: 2026-09-29
closed_at: 2026-09-29
priority: p2
api: additive
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

## 2026-09-29

Load notes go to an info alert (core:load_notes) for the first in-game day, or until hidden, not to the message log: the log is hashed sim state and the notes are one client's. The save status is Writer::failing(): the last write's error until a write works again. The Writer itself has no failure test (a real disk failure is hard to stage); the UI tests cover the alert appearing and clearing.

## 2026-09-29

PAUSED at the merge freeze (main d633f7b5): fix and test done (test fails before, passes after), committed on fix/e805a478-save-failure-alert and rebased on origin/main before the freeze; the full gate has not run on this commit. Next: rebase onto main, run scripts/task check, mark the PR ready.
